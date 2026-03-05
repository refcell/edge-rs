//! The Edge Language Compiler CLI binary.

mod cli;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    // Only initialize tracing if verbose mode is requested.
    // Egglog is extremely noisy at INFO level, so we suppress it by default.
    // Verbose mode enables edge crate logs at INFO; egglog stays at WARN.
    let cli = cli::Cli::parse();

    if cli.is_verbose() {
        use tracing_subscriber::EnvFilter;
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::new("edge=info,egglog=warn,info")
            )
            .init();
    }

    cli.execute()
}
