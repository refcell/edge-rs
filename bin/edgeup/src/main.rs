//! The Edge toolchain installer and version manager binary.

mod cli;
mod installer;
mod shell;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = cli::Cli::parse();
    cli.execute()
}
