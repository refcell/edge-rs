//! CLI arguments and command handling for the edgeup installer.

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::{installer::Installer, shell::Shell};

/// The Edge toolchain installer and version manager
#[derive(Debug, Parser)]
#[command(name = "edgeup")]
#[command(about = "The Edge toolchain installer and version manager")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands for edgeup
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install the Edge toolchain
    Install {
        /// Version to install (default: latest)
        version: Option<String>,
    },
    /// Update to the latest Edge toolchain
    Update,
    /// List installed Edge versions
    List,
    /// Switch to a specific installed version
    Use {
        /// Version to switch to
        version: String,
    },
    /// Uninstall Edge toolchain
    Uninstall {
        /// Version to uninstall (default: all)
        version: Option<String>,
    },
    /// Update edgeup itself
    SelfUpdate,
    /// Print version information
    Version,
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        let installer = Installer::new()?;

        match self.command {
            Commands::Install { version } => {
                installer.install(version)?;
                // Add the bin directory to PATH in the user's shell RC file.
                let shell = Shell::detect()?;
                let bin_dir = dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
                    .join(".edgeup")
                    .join("bin");
                shell.add_to_path(&bin_dir)?;
                eprintln!(
                    "To start using Edge toolchain, run: source {}",
                    shell.rc_file().display()
                );
                Ok(())
            }
            Commands::Update => installer.update(),
            Commands::List => installer.list(),
            Commands::Use { version } => installer.use_version(&version),
            Commands::Uninstall { version } => installer.uninstall(version),
            Commands::SelfUpdate => installer.self_update(),
            Commands::Version => {
                println!("edgeup {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
        }
    }
}
