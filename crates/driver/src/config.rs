//! Compiler configuration

use std::path::PathBuf;

pub use edge_ir::OptimizeFor;

/// What the compiler should emit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmitKind {
    /// Emit tokens only (debugging)
    Tokens,
    /// Emit AST only (debugging)
    Ast,
    /// Emit IR only (s-expression format)
    Ir,
    /// Emit IR in pretty-printed format
    PrettyIr,
    /// Emit post-optimization assembly (disassembly with labeled blocks)
    Asm,
    /// Emit Ethereum-compatible ABI JSON
    Abi,
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
    /// What metric to optimize for during extraction
    pub optimize_for: OptimizeFor,
    /// Path to the Edge standard library directory
    pub std_path: Option<PathBuf>,
    /// Suppress diagnostic output to stderr (diagnostics are still collected)
    pub quiet: bool,
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
            optimize_for: OptimizeFor::default(),
            std_path: None,
            quiet: false,
        }
    }
}
