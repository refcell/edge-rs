//! CLI arguments and command handling for the edgec compiler.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{ArgAction, Parser, Subcommand};
use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};

/// The Edge Language Compiler
#[derive(Debug, Parser)]
#[command(name = "edgec")]
#[command(about = "The Edge Language Compiler")]
#[command(version)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Source file to compile (outputs bytecode to stdout)
    pub file: Option<PathBuf>,

    /// Output file path (write raw bytecode bytes)
    #[arg(short, long, requires = "file")]
    pub output: Option<PathBuf>,

    /// Verbosity level (-v warn, -vv info, -vvv debug, -vvvv trace)
    #[arg(short, action = ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Subcommands for inspecting compiler pipeline stages
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Check an Edge source file for errors without producing output
    Check(FileArgs),
    /// Lex a file and print tokens
    Lex(FileArgs),
    /// Parse a file and print AST
    Parse(FileArgs),
}

/// Arguments for file-based subcommands
#[derive(Debug, Parser)]
pub struct FileArgs {
    /// Source file to process
    pub file: PathBuf,
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        match self.command {
            Some(Commands::Check(args)) => Self::check(args),
            Some(Commands::Lex(args)) => Self::lex(args),
            Some(Commands::Parse(args)) => Self::parse(args),
            None => {
                if let Some(file) = self.file {
                    Self::compile(file, self.output)
                } else {
                    bail!("no input file specified")
                }
            }
        }
    }

    fn compile(file: PathBuf, output: Option<PathBuf>) -> Result<()> {
        let mut config = CompilerConfig::new(file);
        config.output_file = output.clone();
        config.emit = EmitKind::Bytecode;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let result = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(ref bytecode) = result.bytecode {
            if bytecode.is_empty() {
                eprintln!("warning: empty bytecode produced");
            } else {
                let hex: String = bytecode.iter().map(|b| format!("{b:02x}")).collect();
                println!("0x{hex}");

                if let Some(path) = output {
                    std::fs::write(&path, bytecode)?;
                }
            }
        } else {
            eprintln!("warning: bytecode generation produced no output");
        }

        Ok(())
    }

    fn check(args: FileArgs) -> Result<()> {
        let config = CompilerConfig::new(args.file);
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    fn lex(args: FileArgs) -> Result<()> {
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Tokens;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(tokens) = output.tokens {
            for token in tokens {
                println!("{:#?}", token);
            }
        }

        Ok(())
    }

    fn parse(args: FileArgs) -> Result<()> {
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Ast;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(ast) = output.ast {
            println!("{:#?}", ast);
        }

        Ok(())
    }
}
