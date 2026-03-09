//! The main compiler struct that orchestrates the compilation pipeline

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

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
    /// Emitted ABI JSON entries (if emit=abi)
    pub abi: Option<Vec<edge_typeck::AbiEntry>>,
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
        let mut config = CompilerConfig::new(std::path::PathBuf::from("<stdin>"));
        config.quiet = true;
        Self {
            session: Session::new(config, source.into()),
        }
    }

    /// Get the diagnostic messages accumulated during compilation.
    pub fn diagnostic_messages(&self) -> Vec<String> {
        self.session
            .diagnostics
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect()
    }

    /// Render all diagnostics to a plain-text string (no ANSI colors).
    ///
    /// Useful for snapshot tests — captures the same output as ariadne would
    /// print to stderr.
    pub fn render_diagnostics(&self) -> String {
        let path = self.session.config.input_file.display().to_string();
        self.session
            .diagnostics
            .render_to_string(&path, &self.session.source)
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
                abi: None,
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
                abi: None,
            });
        }

        // Type check pass
        let checked = edge_typeck::TypeChecker::new().check(&ast).map_err(|e| {
            self.session
                .emit_error(Diagnostic::error(format!("type error: {e}")));
            self.session.report_diagnostics();
            CompileError::TypeCheckErrors
        })?;

        // ABI extraction — always compute so downstream consumers (e.g. standard-json)
        // can access the ABI alongside bytecode.
        let abi: Vec<edge_typeck::AbiEntry> = checked
            .contracts
            .iter()
            .flat_map(|c| edge_typeck::extract_abi(c, &checked.events))
            .collect();

        // Return early if ABI is all the user requested
        if emit == EmitKind::Abi {
            return Ok(CompileOutput {
                tokens: None,
                ast: None,
                ir: None,
                bytecode: None,
                bytecodes: None,
                asm: None,
                abi: Some(abi),
            });
        }

        // IR lowering + optimization
        let t_ir = std::time::Instant::now();
        let ir_program = edge_ir::lower_and_optimize(
            &ast,
            self.session.config.optimization_level,
            self.session.config.optimize_for,
        )
        .map_err(|e| {
            let diag = match e {
                edge_ir::IrError::Diagnostic(d) => d,
                edge_ir::IrError::LoweringSpanned { message, span } => {
                    Diagnostic::error(message).with_label(span, "error occurred here")
                }
                other => Diagnostic::error(format!("IR lowering error: {other}")),
            };
            self.session.emit_error(diag);
            self.session.report_diagnostics();
            CompileError::Aborted
        })?;

        tracing::info!("IR lowering + optimization: {:?}", t_ir.elapsed());

        // Emit any warnings from IR lowering
        for warning in &ir_program.warnings {
            self.session.emit_warning(warning.clone());
        }
        if !ir_program.warnings.is_empty() {
            self.session.report_diagnostics();
        }

        if emit == EmitKind::Ir || emit == EmitKind::PrettyIr {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: Some(ir_program),
                bytecode: None,
                bytecodes: None,
                asm: None,
                abi: None,
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
                abi: None,
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
                        warnings: Vec::new(),
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
                abi: None,
            });
        }

        // Code generation — compile each contract individually
        let t_codegen = std::time::Instant::now();
        let mut all_bytecodes: IndexMap<String, Vec<u8>> = IndexMap::new();
        for contract in &ir_program.contracts {
            let single_program = edge_ir::EvmProgram {
                contracts: vec![contract.clone()],
                free_functions: Vec::new(),
                warnings: Vec::new(),
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
                abi: None,
            });
        }

        tracing::info!("Codegen: {:?}", t_codegen.elapsed());

        let last_bytecode = all_bytecodes.values().last().cloned();

        Ok(CompileOutput {
            tokens: None,
            ast: Some(ast),
            ir: None,
            bytecode: last_bytecode,
            bytecodes: Some(all_bytecodes),
            asm: None,
            abi: Some(abi),
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
            self.session.report_diagnostics();
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
                self.session.report_diagnostics();
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

        // Cache parsed module stmts so we don't re-parse the same module multiple times.
        let mut parsed_modules: HashMap<String, Vec<edge_ast::Stmt>> = HashMap::new();
        let mut new_stmts: Vec<edge_ast::Stmt> = Vec::new();
        // Track which modules were fully imported (no symbol filter) to avoid duplicates.
        let mut fully_imported: HashSet<String> = HashSet::new();

        for file_segments in &imports_to_resolve {
            // Resolve to a module key ("tokens/erc20"), source text, and optional symbol filter.
            let (module_key, source, symbol_filter) =
                self.resolve_module_source(&explicit_std_path, file_segments)?;

            // If this module was already fully imported, skip.
            if fully_imported.contains(&module_key) {
                continue;
            }

            if symbol_filter.is_none() {
                fully_imported.insert(module_key.clone());
            }

            // Parse the module if not already cached.
            if !parsed_modules.contains_key(&module_key) {
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
                    self.session.report_diagnostics();
                    CompileError::ParseErrors
                })?;

                // Filter out module imports/decls from the parsed stmts.
                let stmts: Vec<_> = imported_program
                    .stmts
                    .into_iter()
                    .filter(|s| {
                        !matches!(
                            s,
                            edge_ast::Stmt::ModuleImport(_) | edge_ast::Stmt::ModuleDecl(_)
                        )
                    })
                    .collect();
                parsed_modules.insert(module_key.clone(), stmts);
            }

            let module_stmts = &parsed_modules[&module_key];

            // Collect items, filtering to the specific symbol if requested.
            for stmt in module_stmts {
                if let Some(ref symbol) = symbol_filter {
                    let name = match stmt {
                        edge_ast::Stmt::FnAssign(fn_decl, _)
                        | edge_ast::Stmt::ComptimeFn(fn_decl, _) => Some(&fn_decl.name.name),
                        edge_ast::Stmt::ConstAssign(decl, _, _) => Some(&decl.name.name),
                        edge_ast::Stmt::AbiDecl(abi) => Some(&abi.name.name),
                        edge_ast::Stmt::TraitDecl(tr, _) => Some(&tr.name.name),
                        edge_ast::Stmt::TypeAssign(td, _, _) => Some(&td.name.name),
                        edge_ast::Stmt::EventDecl(ev) => Some(&ev.name.name),
                        _ => None,
                    };
                    if name != Some(symbol) {
                        continue;
                    }
                }
                new_stmts.push(stmt.clone());
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
    /// Returns `(module_key, source, symbol_filter)` where `symbol_filter` is
    /// `Some(name)` when the last segment was a symbol inside a file (not a file itself).
    fn resolve_module_source(
        &mut self,
        explicit_std_path: &Option<PathBuf>,
        segments: &[String],
    ) -> Result<(String, String, Option<String>), CompileError> {
        if segments.is_empty() {
            self.session.emit_error(Diagnostic::error(
                "`use std;` is not a valid import — specify a module path like `use std::math;`"
                    .to_string(),
            ));
            self.session.report_diagnostics();
            return Err(CompileError::Aborted);
        }

        // Try the full segments first. If that fails and there are 2+ segments,
        // fall back to stripping the last one (it's a symbol inside the file).
        {
            let key = segments.join("/");

            // 1. Explicit filesystem override.
            if let Some(ref std_path) = explicit_std_path {
                if let Some(source) = Self::try_read_from_fs(std_path, segments) {
                    tracing::debug!("resolved std::{} from filesystem override", key);
                    return Ok((key, source, None));
                }
            }

            // 2. Embedded binary sources.
            if let Some(source) = Self::try_read_from_embedded(segments) {
                tracing::debug!("resolved std::{} from embedded stdlib", key);
                return Ok((key, source.to_string(), None));
            }
        }

        // Symbol-level fallback: strip last segment as a symbol name.
        if segments.len() > 1 {
            let file_segs = &segments[..segments.len() - 1];
            let symbol = segments.last().unwrap().clone();
            let key = file_segs.join("/");

            if let Some(ref std_path) = explicit_std_path {
                if let Some(source) = Self::try_read_from_fs(std_path, file_segs) {
                    tracing::debug!(
                        "resolved std::{} (symbol `{}`) from filesystem override",
                        key,
                        symbol
                    );
                    return Ok((key, source, Some(symbol)));
                }
            }

            if let Some(source) = Self::try_read_from_embedded(file_segs) {
                tracing::debug!(
                    "resolved std::{} (symbol `{}`) from embedded stdlib",
                    key,
                    symbol
                );
                return Ok((key, source.to_string(), Some(symbol)));
            }
        }

        // Nothing found — emit a helpful error.
        let module_path = segments.join("::");
        self.session.emit_error(Diagnostic::error(format!(
            "cannot find std module `{module_path}` in the embedded standard library or any \
             provided --std-path directory.\n\
             hint: available modules include `std::math`, `std::auth`, `std::tokens::erc20`, etc."
        )));
        self.session.report_diagnostics();
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

    /// Get a mutable reference to the session
    pub const fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}
