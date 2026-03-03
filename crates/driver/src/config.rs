//! Compiler configuration

use std::path::PathBuf;

/// What the compiler should emit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmitKind {
    /// Emit tokens only (debugging)
    Tokens,
    /// Emit AST only (debugging)
    Ast,
    /// Emit EVM bytecode (default)
    #[default]
    Bytecode,
}

/// Configuration for a compiler invocation
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// The input source file
    pub input_file: PathBuf,
    /// Optional output file path
    pub output_file: Option<PathBuf>,
    /// What to emit
    pub emit: EmitKind,
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Verbose output
    pub verbose: bool,
}

impl CompilerConfig {
    /// Create a new compiler config for the given input file
    pub fn new(input_file: PathBuf) -> Self {
        Self {
            input_file,
            output_file: None,
            emit: EmitKind::default(),
            optimization_level: 0,
            verbose: false,
        }
    }
}
