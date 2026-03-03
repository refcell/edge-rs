//! The Edge Language Compiler CLI binary.

mod cli;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = cli::Cli::parse();
    cli.execute()
}
