//! The Edge Language Compiler CLI binary.

mod cli;

use anyhow::Result;
use clap::Parser;
use tracing::Level;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let level = match cli.verbose {
        0 => None,
        1 => Some(Level::WARN),
        2 => Some(Level::INFO),
        3 => Some(Level::DEBUG),
        _ => Some(Level::TRACE),
    };

    if let Some(level) = level {
        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_writer(std::io::stderr)
            .init();
    }

    cli.execute()
}
