//! The main compiler struct that orchestrates the compilation pipeline

use std::fs;

use edge_ast::Program;
use edge_diagnostics::Diagnostic;
use edge_lexer::lexer::Lexer;
use edge_parser::Parser;
use edge_types::tokens::Token;

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
    /// Emitted bytecode (if emit=bytecode)
    pub bytecode: Option<Vec<u8>>,
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
            });
        }

        // Parse phase
        let ast = self.parse()?;

        if emit == EmitKind::Ast {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: None,
                bytecode: None,
            });
        }

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

        if emit == EmitKind::Ir {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                ir: Some(ir_program),
                bytecode: None,
            });
        }

        // Code generation
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

        Ok(CompileOutput {
            tokens: None,
            ast: Some(ast),
            ir: None,
            bytecode: Some(bytecode),
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

    /// Get a reference to the session
    pub const fn session(&self) -> &Session {
        &self.session
    }
}
