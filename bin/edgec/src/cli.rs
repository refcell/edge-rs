//! CLI arguments and command handling for the edgec compiler.

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{ArgAction, Parser, Subcommand};
use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind, OptimizeFor},
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

    /// What to emit: tokens, ast, ir, pretty-ir, asm, abi, bytecode
    #[arg(long, value_parser = ["tokens", "ast", "ir", "pretty-ir", "asm", "abi", "bytecode"], default_value = "bytecode")]
    pub emit: String,

    /// Optimization level (0-3)
    #[arg(short = 'O', value_parser = clap::value_parser!(u8).range(0..=3), default_value = "0")]
    pub opt_level: u8,

    /// What metric to optimize extraction for
    #[arg(long, value_parser = ["gas", "size"], default_value = "gas")]
    pub optimize_for: String,

    /// Path to the Edge standard library directory
    #[arg(long, env = "EDGE_STD_PATH")]
    pub std_path: Option<PathBuf>,

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
    /// Start the LSP server (communicates over stdin/stdout)
    Lsp,
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
            Some(Commands::Check(args)) => Self::check(args, self.std_path),
            Some(Commands::Lex(args)) => Self::lex(args, self.std_path),
            Some(Commands::Lsp) => Self::lsp(),
            Some(Commands::Parse(args)) => Self::parse(args, self.std_path),
            None => {
                if let Some(file) = self.file {
                    Self::compile(
                        file,
                        self.output,
                        &self.emit,
                        self.opt_level,
                        &self.optimize_for,
                        self.std_path,
                    )
                } else {
                    bail!("no input file specified")
                }
            }
        }
    }

    fn compile(
        file: PathBuf,
        output: Option<PathBuf>,
        emit: &str,
        opt_level: u8,
        optimize_for: &str,
        std_path: Option<PathBuf>,
    ) -> Result<()> {
        let emit_kind = match emit {
            "tokens" => EmitKind::Tokens,
            "ast" => EmitKind::Ast,
            "ir" => EmitKind::Ir,
            "pretty-ir" => EmitKind::PrettyIr,
            "asm" => EmitKind::Asm,
            "abi" => EmitKind::Abi,
            "bytecode" => EmitKind::Bytecode,
            _ => EmitKind::Bytecode,
        };

        let mut config = CompilerConfig::new(file);
        config.output_file = output.clone();
        config.emit = emit_kind;
        config.optimization_level = opt_level;
        config.optimize_for = match optimize_for {
            "size" => OptimizeFor::Size,
            _ => OptimizeFor::Gas,
        };
        config.std_path = std_path;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let result = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        match emit_kind {
            EmitKind::Tokens => {
                if let Some(tokens) = result.tokens {
                    for token in tokens {
                        println!("{:#?}", token);
                    }
                }
            }
            EmitKind::Ast => {
                if let Some(ast) = result.ast {
                    println!("{:#?}", ast);
                }
            }
            EmitKind::Ir => {
                if let Some(ref ir) = result.ir {
                    for contract in &ir.contracts {
                        println!(";; Contract: {}", contract.name);
                        println!(";; Storage fields: {}", contract.storage_fields.len());
                        for field in &contract.storage_fields {
                            println!(";;   {}", edge_ir::sexp::expr_to_sexp(field));
                        }
                        println!();
                        println!(";; Constructor:");
                        println!(
                            "{}",
                            edge_ir::sexp::expr_to_pretty(&contract.constructor, 0)
                        );
                        println!();
                        println!(";; Runtime:");
                        println!("{}", edge_ir::sexp::expr_to_pretty(&contract.runtime, 0));
                        if !contract.internal_functions.is_empty() {
                            println!();
                            println!(";; Internal functions:");
                            for func in &contract.internal_functions {
                                println!("{}", edge_ir::sexp::expr_to_pretty(func, 0));
                            }
                        }
                    }
                    for func in &ir.free_functions {
                        println!();
                        println!("{}", edge_ir::sexp::expr_to_pretty(func, 0));
                    }
                }
            }
            EmitKind::PrettyIr => {
                if let Some(ref ir) = result.ir {
                    for contract in &ir.contracts {
                        print!("{}", edge_ir::pretty::pretty_print_contract(contract));
                    }
                    for func in &ir.free_functions {
                        println!("{}", edge_ir::pretty::pretty_print(func));
                    }
                }
            }
            EmitKind::Asm => {
                if let Some(ref asm_outputs) = result.asm {
                    for (name, asm_out) in asm_outputs {
                        print!(
                            "{}",
                            edge_codegen::pretty_asm::pretty_print_asm(asm_out, name)
                        );
                    }
                }
            }
            EmitKind::Abi => {
                if let Some(ref abi) = result.abi {
                    println!("{}", serde_json::to_string_pretty(abi).unwrap());
                }
            }
            EmitKind::Bytecode => {
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
            }
        }

        Ok(())
    }

    fn check(args: FileArgs, std_path: Option<PathBuf>) -> Result<()> {
        let mut config = CompilerConfig::new(args.file);
        config.std_path = std_path;
        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(())
    }

    fn lex(args: FileArgs, std_path: Option<PathBuf>) -> Result<()> {
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Tokens;
        config.std_path = std_path;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(tokens) = output.tokens {
            for token in tokens {
                println!("{:#?}", token);
            }
        }

        Ok(())
    }

    fn lsp() -> Result<()> {
        tokio::runtime::Runtime::new()?.block_on(edge_lsp::run());
        Ok(())
    }

    fn parse(args: FileArgs, std_path: Option<PathBuf>) -> Result<()> {
        let mut config = CompilerConfig::new(args.file);
        config.emit = EmitKind::Ast;
        config.std_path = std_path;

        let mut compiler = Compiler::new(config).map_err(|e| anyhow::anyhow!("{}", e))?;
        let output = compiler.compile().map_err(|e| anyhow::anyhow!("{}", e))?;

        if let Some(ast) = output.ast {
            println!("{:#?}", ast);
        }

        Ok(())
    }
}
