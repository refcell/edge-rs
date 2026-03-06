//! The main compiler struct that orchestrates the compilation pipeline

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use edge_ast::Program;
use edge_diagnostics::Diagnostic;
use edge_lexer::lexer::Lexer;
use edge_parser::Parser;
use edge_types::tokens::Token;
use indexmap::IndexMap;

use crate::{
    config::{CompilerConfig, EmitKind},
    session::Session,
};

/// Output from a compilation
#[derive(Debug)]
pub struct CompileOutput {
    /// Emitted tokens (if emit=tokens)
    pub tokens: Option<Vec<Token>>,
    /// Emitted AST (if emit=ast)
    pub ast: Option<Program>,
    /// Emitted IR (if emit=ir)
    pub ir: Option<edge_ir::EvmProgram>,
    /// Emitted bytecode for the last contract (backward compat)
    pub bytecode: Option<Vec<u8>>,
    /// Emitted bytecodes for all contracts, keyed by contract name
    pub bytecodes: Option<IndexMap<String, Vec<u8>>>,
    /// Emitted assembly (if emit=asm), keyed by contract name
    pub asm: Option<Vec<(String, edge_codegen::AsmOutput)>>,
}

/// Compiler errors
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// I/O error reading source
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Lexer errors were encountered
    #[error("lex errors encountered")]
    LexErrors,
    /// Parse errors were encountered
    #[error("parse errors encountered")]
    ParseErrors,
    /// Type check errors were encountered
    #[error("type check errors encountered")]
    TypeCheckErrors,
    /// IR lowering errors were encountered
    #[error("IR lowering errors encountered")]
    LowerErrors,
    /// Code generation errors were encountered
    #[error("code generation errors encountered")]
    CodeGenErrors,
    /// Compilation aborted due to errors
    #[error("compilation aborted due to errors")]
    Aborted,
}

/// The Edge compiler
#[derive(Debug)]
pub struct Compiler {
    session: Session,
}

impl Compiler {
    /// Create a new compiler with the given config
    pub fn new(config: CompilerConfig) -> Result<Self, CompileError> {
        let source = fs::read_to_string(&config.input_file)?;
        Ok(Self {
            session: Session::new(config, source),
        })
    }

    /// Create a compiler from an in-memory source string, bypassing file I/O.
    ///
    /// Useful for testing and benchmarks where the source is already in memory.
    pub fn from_source(source: impl Into<String>) -> Self {
        let config = CompilerConfig::new(std::path::PathBuf::from("<stdin>"));
        Self {
            session: Session::new(config, source.into()),
        }
    }

    /// Run the compilation pipeline
    pub fn compile(&mut self) -> Result<CompileOutput, CompileError> {
        tracing::info!("Compiling {:?}", self.session.config.input_file);

        let emit = self.session.config.emit;

        // Lex phase
        let tokens = self.lex()?;

        if emit == EmitKind::Tokens {
            return Ok(CompileOutput {
                tokens: Some(tokens),
                ast: None,
                ir: None,
                bytecode: None,
                bytecodes: None,
                asm: None,
            });
        }

        // Parse phase
        let mut ast = self.parse()?;

        // Resolve `use std::` imports before type checking
        self.resolve_imports(&mut ast)?;

        if emit == EmitKind::Ast {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: None,
                bytecode: None,
                bytecodes: None,
                asm: None,
            });
        }

        // Type check pass
        let _checked = edge_typeck::TypeChecker::new().check(&ast).map_err(|e| {
            self.session
                .emit_error(Diagnostic::error(format!("type error: {e}")));
            self.session.diagnostics.report_all(&self.session.source);
            CompileError::TypeCheckErrors
        })?;

        // IR lowering + optimization
        let ir_program = edge_ir::lower_and_optimize(
            &ast,
            self.session.config.optimization_level,
            self.session.config.optimize_for,
        )
        .map_err(|e| {
            let diag = Diagnostic::error(format!("IR lowering error: {e}"));
            self.session.emit_error(diag);
            self.session.diagnostics.report_all(&self.session.source);
            CompileError::Aborted
        })?;

        if emit == EmitKind::Ir || emit == EmitKind::PrettyIr {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: Some(ir_program),
                bytecode: None,
                bytecodes: None,
                asm: None,
            });
        }

        // No contracts → nothing to codegen; return after type check + IR
        if ir_program.contracts.is_empty() {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: Some(ir_program),
                bytecode: None,
                bytecodes: None,
                asm: None,
            });
        }

        // Assembly output (pre-final-assembly)
        if emit == EmitKind::Asm {
            let mut asm_outputs = Vec::new();
            for contract in &ir_program.contracts {
                let asm_out = edge_codegen::compile_to_asm(
                    &edge_ir::EvmProgram {
                        contracts: vec![contract.clone()],
                        free_functions: Vec::new(),
                    },
                    self.session.config.optimization_level,
                    self.session.config.optimize_for,
                )
                .map_err(|e| {
                    self.session
                        .emit_error(Diagnostic::error(format!("codegen error: {e}")));
                    CompileError::Aborted
                })?;
                asm_outputs.push((contract.name.clone(), asm_out));
            }
            return Ok(CompileOutput {
                tokens: None,
                ast: None,
                ir: None,
                bytecode: None,
                bytecodes: None,
                asm: Some(asm_outputs),
            });
        }

        // Code generation — compile each contract individually
        let mut all_bytecodes: IndexMap<String, Vec<u8>> = IndexMap::new();
        for contract in &ir_program.contracts {
            let single_program = edge_ir::EvmProgram {
                contracts: vec![contract.clone()],
                free_functions: Vec::new(),
            };
            let bytecode = edge_codegen::compile(
                &single_program,
                self.session.config.optimization_level,
                self.session.config.optimize_for,
            )
            .map_err(|e| {
                self.session
                    .emit_error(Diagnostic::error(format!("codegen error: {e}")));
                CompileError::Aborted
            })?;
            all_bytecodes.insert(contract.name.clone(), bytecode);
        }

        // Also compile free functions if no contracts
        if ir_program.contracts.is_empty() && !ir_program.free_functions.is_empty() {
            let bytecode = edge_codegen::compile(
                &ir_program,
                self.session.config.optimization_level,
                self.session.config.optimize_for,
            )
            .map_err(|e| {
                self.session
                    .emit_error(Diagnostic::error(format!("codegen error: {e}")));
                CompileError::Aborted
            })?;
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: None,
                bytecode: Some(bytecode),
                bytecodes: None,
                asm: None,
            });
        }

        let last_bytecode = all_bytecodes.values().last().cloned();

        Ok(CompileOutput {
            tokens: None,
            ast: Some(ast),
            ir: None,
            bytecode: last_bytecode,
            bytecodes: Some(all_bytecodes),
            asm: None,
        })
    }

    /// Run the lexer and collect tokens
    fn lex(&mut self) -> Result<Vec<Token>, CompileError> {
        // Clone source to avoid borrow conflict with session during error reporting
        let source = self.session.source.clone();
        let lexer = Lexer::new(&source);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for result in lexer {
            match result {
                Ok(token) => tokens.push(token),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            for e in errors {
                self.session.emit_error(
                    Diagnostic::error(format!("lexer error: {e:?}"))
                        .with_label(e.span, "invalid token here"),
                );
            }
            self.session.diagnostics.report_all(&self.session.source);
            return Err(CompileError::LexErrors);
        }

        Ok(tokens)
    }

    /// Run the parser and produce an AST
    fn parse(&mut self) -> Result<Program, CompileError> {
        let mut parser = Parser::new(&self.session.source).map_err(|e| {
            self.session
                .emit_error(Diagnostic::error(format!("parse error: {e}")));
            CompileError::ParseErrors
        })?;

        match parser.parse() {
            Ok(program) => Ok(program),
            Err(e) => {
                self.session
                    .emit_error(Diagnostic::error(format!("parse error: {e}")));
                self.session.diagnostics.report_all(&self.session.source);
                Err(CompileError::ParseErrors)
            }
        }
    }

    /// Resolve `use std::...` imports by locating source for each imported module —
    /// first from an explicit filesystem override, then from the stdlib embedded in
    /// the binary — and merging their top-level items into the program AST.
    ///
    /// Resolution order:
    /// 1. Explicit `--std-path` / `EDGE_STD_PATH` filesystem directory (for development
    ///    overrides and custom installations).
    /// 2. Embedded sources baked into the binary at compile time via `build.rs`
    ///    (works on any machine with no extra setup).
    fn resolve_imports(&mut self, ast: &mut Program) -> Result<(), CompileError> {
        // Collect std imports from the AST.
        // Build a full path-segments list by combining intermediate `segments` with the final
        // `path` identifier.  For example:
        //   `use std::math;`                  → file_segments = ["math"]
        //   `use std::tokens::erc20;`         → file_segments = ["tokens", "erc20"]
        //   `use std::tokens::erc20::IERC20;` → file_segments = ["tokens", "erc20", "IERC20"]
        //     (IERC20 is a name *inside* the file; the symbol-level fallback handles it)
        let mut imports_to_resolve: Vec<Vec<String>> = Vec::new();
        for stmt in &ast.stmts {
            if let edge_ast::Stmt::ModuleImport(ref import) = stmt {
                if import.root.name == "std" {
                    let mut file_segments: Vec<String> =
                        import.segments.iter().map(|s| s.name.clone()).collect();

                    // Append the final path component — it names the file/module
                    match &import.path {
                        Some(edge_ast::ImportPath::Ident(ident)) => {
                            file_segments.push(ident.name.clone());
                        }
                        Some(edge_ast::ImportPath::All | edge_ast::ImportPath::Nested(_))
                        | None => {
                            // Glob/tree import or bare import: segments are the module path
                        }
                    }

                    if !file_segments.is_empty() {
                        imports_to_resolve.push(file_segments);
                    }
                }
            }
        }

        if imports_to_resolve.is_empty() {
            return Ok(());
        }

        // Canonicalize the explicit override path once (if provided).
        let explicit_std_path: Option<PathBuf> =
            self.session.config.std_path.as_ref().and_then(|p| {
                let canon = fs::canonicalize(p).unwrap_or_else(|_| p.clone());
                if canon.is_dir() {
                    Some(canon)
                } else {
                    tracing::warn!(
                        "--std-path / EDGE_STD_PATH points to non-existent directory: {}",
                        p.display()
                    );
                    None
                }
            });

        let mut already_parsed: HashSet<String> = HashSet::new();
        let mut new_stmts: Vec<edge_ast::Stmt> = Vec::new();

        for file_segments in &imports_to_resolve {
            // Resolve to a module key ("tokens/erc20") and source text.
            let (module_key, source) =
                self.resolve_module_source(&explicit_std_path, file_segments)?;

            if already_parsed.contains(&module_key) {
                continue;
            }
            already_parsed.insert(module_key.clone());

            let mut parser = Parser::new(&source).map_err(|e| {
                self.session.emit_error(Diagnostic::error(format!(
                    "parse error in std module `{module_key}`: {e}"
                )));
                CompileError::ParseErrors
            })?;

            let imported_program = parser.parse().map_err(|e| {
                self.session.emit_error(Diagnostic::error(format!(
                    "parse error in std module `{module_key}`: {e}"
                )));
                self.session.diagnostics.report_all(&self.session.source);
                CompileError::ParseErrors
            })?;

            // Collect all top-level items from the imported module, skipping its own imports.
            for stmt in imported_program.stmts {
                if matches!(&stmt, edge_ast::Stmt::ModuleImport(_) | edge_ast::Stmt::ModuleDecl(_))
                {
                    continue;
                }
                new_stmts.push(stmt);
            }
        }

        // Prepend imported items so they're available to the user's code.
        if !new_stmts.is_empty() {
            new_stmts.append(&mut ast.stmts);
            ast.stmts = new_stmts;
        }

        Ok(())
    }

    /// Resolve a set of import path segments to a `(module_key, source)` pair.
    ///
    /// Tries, in order:
    /// 1. Explicit filesystem override (`--std-path` / `EDGE_STD_PATH`)
    /// 2. Embedded sources baked into the binary at compile time
    ///
    /// Supports symbol-level imports: if `segments = ["auth", "IOwned"]` and no file
    /// exists at `std/auth/IOwned.edge`, retries with `["auth"]` treating `IOwned` as
    /// a symbol name within `std/auth.edge`.
    fn resolve_module_source(
        &mut self,
        explicit_std_path: &Option<PathBuf>,
        segments: &[String],
    ) -> Result<(String, String), CompileError> {
        if segments.is_empty() {
            self.session.emit_error(Diagnostic::error(
                "`use std;` is not a valid import — specify a module path like `use std::math;`"
                    .to_string(),
            ));
            self.session.diagnostics.report_all(&self.session.source);
            return Err(CompileError::Aborted);
        }

        // Try the full segments, then fall back to stripping the last one (symbol-level import).
        let candidates: &[&[String]] = if segments.len() > 1 {
            &[segments, &segments[..segments.len() - 1]]
        } else {
            &[segments]
        };

        for &segs in candidates {
            let key = segs.join("/");

            // 1. Explicit filesystem override.
            if let Some(ref std_path) = explicit_std_path {
                if let Some(source) = Self::try_read_from_fs(std_path, segs) {
                    tracing::debug!("resolved std::{} from filesystem override", key);
                    return Ok((key, source));
                }
            }

            // 2. Embedded binary sources.
            if let Some(source) = Self::try_read_from_embedded(segs) {
                tracing::debug!("resolved std::{} from embedded stdlib", key);
                return Ok((key, source.to_string()));
            }
        }

        // Nothing found — emit a helpful error.
        let module_path = segments.join("::");
        self.session.emit_error(Diagnostic::error(format!(
            "cannot find std module `{module_path}` in the embedded standard library or any \
             provided --std-path directory.\n\
             hint: available modules include `std::math`, `std::auth`, `std::tokens::erc20`, etc."
        )));
        self.session.diagnostics.report_all(&self.session.source);
        Err(CompileError::Aborted)
    }

    /// Try to read a std module from the filesystem override directory.
    fn try_read_from_fs(std_path: &Path, segments: &[String]) -> Option<String> {
        let mut dir = std_path.to_path_buf();
        for seg in &segments[..segments.len() - 1] {
            dir.push(seg);
        }
        let last = &segments[segments.len() - 1];

        // Try <dir>/<last>.edge
        let file_path = dir.join(format!("{last}.edge"));
        if file_path.is_file() {
            return fs::read_to_string(&file_path).ok();
        }

        // Try <dir>/<last>/mod.edge
        let mod_path = dir.join(last).join("mod.edge");
        if mod_path.is_file() {
            return fs::read_to_string(&mod_path).ok();
        }

        None
    }

    /// Try to read a std module from the embedded binary sources.
    fn try_read_from_embedded(segments: &[String]) -> Option<&'static str> {
        let key = segments.join("/");
        crate::std_embedded::STD_SOURCES
            .iter()
            .find(|(k, _)| *k == key.as_str())
            .map(|(_, src)| *src)
    }

    /// Get a reference to the session
    pub const fn session(&self) -> &Session {
        &self.session
    }
}
