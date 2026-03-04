//! CLI arguments and command handling for the edgec compiler.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};

/// The Edge Language Compiler
#[derive(Debug, Parser)]
#[command(name = "edgec")]
#[command(about = "The Edge Language Compiler")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for the edgec compiler
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Compile an Edge source file
    Build(BuildArgs),
    /// Check an Edge source file without producing output
    Check(CheckArgs),
    /// Lex a file and print tokens (debugging)
    Lex(FileArgs),
    /// Parse a file and print AST (debugging)
    Parse(FileArgs),
    /// Print version information
    Version,
}

/// Arguments for the build command
#[derive(Debug, Parser)]
pub struct BuildArgs {
    /// Source file to compile
    pub file: PathBuf,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// What to emit: tokens, ast, bytecode
    #[arg(long, value_parser = ["tokens", "ast", "bytecode"], default_value = "bytecode")]
    pub emit: String,

    /// Optimization level (0-3)
    #[arg(short = 'O', value_parser = clap::value_parser!(u8).range(0..=3), default_value = "0")]
    pub opt_level: u8,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Arguments for the check command
#[derive(Debug, Parser)]
pub struct CheckArgs {
    /// Source file to check
    pub file: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Arguments for file-based commands (lex, parse)
#[derive(Debug, Parser)]
pub struct FileArgs {
    /// Source file to process
    pub file: PathBuf,
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        match self.command {
            Commands::Build(args) => Self::build(args),
            Commands::Check(args) => Self::check(args),
            Commands::Lex(args) => Self::lex(args),
            Commands::Parse(args) => Self::parse(args),
            Commands::Version => {
                println!("edgec {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }

    fn build(args: BuildArgs) -> Result<()> {
        if args.verbose {
            eprintln!("Building: {}", args.file.display());
        }

        // Parse the emit kind
        let emit = match args.emit.as_str() {
            "tokens" => EmitKind::Tokens,
            "ast" => EmitKind::Ast,
            "bytecode" => EmitKind::Bytecode,
            _ => EmitKind::Bytecode,
        };

        // Create compiler config
        let mut config = CompilerConfig::new(args.file.clone());
        config.output_file = args.output.clone();
        config.emit = emit;
        config.optimization_level = args.opt_level;
        config.verbose = args.verbose;

        // Create and run compiler
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;

        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        // Output the result based on emit kind
        match emit {
            EmitKind::Tokens => {
                if let Some(tokens) = output.tokens {
                    println!("=== Tokens ===");
                    for token in tokens {
                        println!("{:#?}", token);
                    }
                }
            }
            EmitKind::Ast => {
                if let Some(ast) = output.ast {
                    println!("=== Abstract Syntax Tree ===");
                    println!("{:#?}", ast);
                }
            }
            EmitKind::Bytecode => {
                if let Some(ref bytecode) = output.bytecode {
                    if bytecode.is_empty() {
                        eprintln!("warning: empty bytecode produced");
                    } else {
                        // Print hex representation
                        let hex: String = bytecode.iter().map(|b| format!("{b:02x}")).collect();
                        println!("Bytecode ({} bytes):", bytecode.len());
                        println!("0x{hex}");

                        // Write to output file if specified
                        if let Some(output_file) = &args.output {
                            std::fs::write(output_file, bytecode)?;
                            if args.verbose {
                                eprintln!("Wrote bytecode to {}", output_file.display());
                            }
                        }

                        if args.verbose {
                            eprintln!("Compilation successful");
                        }
                    }
                } else {
                    eprintln!("warning: bytecode generation produced no output");
                }
            }
        }

        Ok(())
    }

    fn check(args: CheckArgs) -> Result<()> {
        if args.verbose {
            eprintln!("Checking: {}", args.file.display());
        }

        // Create compiler config (don't emit anything, just check)
        let mut config = CompilerConfig::new(args.file);
        config.verbose = args.verbose;

        // Create and run compiler
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;

        // Just run through the compilation pipeline to check for errors
        compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if args.verbose {
            eprintln!("Check passed!");
        }
        Ok(())
    }

    fn lex(args: FileArgs) -> Result<()> {
        // Create compiler config with emit=tokens
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Tokens;

        // Create and run compiler
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;

        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(tokens) = output.tokens {
            println!("=== Tokens ===");
            for token in tokens {
                println!("{:#?}", token);
            }
        }

        Ok(())
    }

    fn parse(args: FileArgs) -> Result<()> {
        // Create compiler config with emit=ast
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Ast;

        // Create and run compiler
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;

        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(ast) = output.ast {
            println!("=== Abstract Syntax Tree ===");
            println!("{:#?}", ast);
        }

        Ok(())
    }
}
