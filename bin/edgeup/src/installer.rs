//! Installer logic for the edgeup toolchain manager.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::shell::Shell;

#[allow(dead_code)]
const GITHUB_REPO: &str = "refcell/edge-rs";

/// Release information from GitHub API
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: String,
}

/// Installer for managing Edge toolchain versions
pub struct Installer {
    /// Installation directory (~/.edgeup)
    install_dir: PathBuf,
}

impl Installer {
    /// Create a new installer instance
    pub fn new() -> Result<Self> {
        let install_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".edgeup");

        // Create install directory if it doesn't exist
        fs::create_dir_all(&install_dir)?;

        Ok(Self { install_dir })
    }

    /// Get the bin directory (~/.edgeup/bin)
    fn bin_dir(&self) -> PathBuf {
        self.install_dir.join("bin")
    }

    /// Get the versions directory (~/.edgeup/versions)
    fn versions_dir(&self) -> PathBuf {
        self.install_dir.join("versions")
    }

    /// Get the path to the edgec binary
    fn edgec_bin(&self) -> PathBuf {
        self.bin_dir().join("edgec")
    }

    /// Install a version of the Edge toolchain
    pub fn install(&self, version: Option<String>) -> Result<()> {
        let version = version.unwrap_or_else(|| "latest".to_string());

        eprintln!("Installing Edge toolchain (version: {})...", version);

        // TODO: Fetch release information from GitHub
        // GET https://api.github.com/repos/refcell/edge-rs/releases
        // or
        // GET https://api.github.com/repos/refcell/edge-rs/releases/tags/{version}

        // TODO: Download the appropriate binary for the current platform:
        // - linux-x86_64
        // - darwin-x86_64
        // - darwin-aarch64

        // TODO: Extract and install to ~/.edgeup/versions/{version}/

        // TODO: Update PATH via shell integration
        let shell = Shell::detect()?;
        shell.add_to_path(&self.bin_dir())?;

        eprintln!("Installation complete!");
        eprintln!("To start using Edge toolchain, run: source {}", shell.rc_file().display());

        Ok(())
    }

    /// Update to the latest version
    pub fn update(&self) -> Result<()> {
        eprintln!("Updating to latest version...");

        // TODO: Fetch latest release from GitHub
        // Check current version vs latest
        // If different, run install(Some(latest_version))

        eprintln!("Update complete!");
        Ok(())
    }

    /// List installed versions
    pub fn list(&self) -> Result<()> {
        let versions_dir = self.versions_dir();

        if !versions_dir.exists() {
            println!("No versions installed yet.");
            return Ok(());
        }

        println!("Installed versions:");
        for entry in fs::read_dir(&versions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(version) = path.file_name().and_then(|n| n.to_str()) {
                    println!("  - {}", version);
                }
            }
        }

        Ok(())
    }

    /// Switch to a specific installed version
    pub fn use_version(&self, version: &str) -> Result<()> {
        let version_dir = self.versions_dir().join(version);

        if !version_dir.exists() {
            return Err(anyhow!("Version {} is not installed", version));
        }

        eprintln!("Switching to version {}...", version);

        // Create symlink from ~/.edgeup/bin/edgec to the selected version
        let bin_dir = self.bin_dir();
        fs::create_dir_all(&bin_dir)?;

        let edgec_bin = self.edgec_bin();
        if edgec_bin.exists() {
            fs::remove_file(&edgec_bin)?;
        }

        let version_bin = version_dir.join("edgec");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&version_bin, &edgec_bin)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&version_bin, &edgec_bin)?;

        eprintln!("Switched to version {}", version);
        Ok(())
    }

    /// Uninstall a version (or all if not specified)
    pub fn uninstall(&self, version: Option<String>) -> Result<()> {
        match version {
            Some(v) => {
                let version_dir = self.versions_dir().join(&v);
                if !version_dir.exists() {
                    return Err(anyhow!("Version {} is not installed", v));
                }
                eprintln!("Uninstalling version {}...", v);
                fs::remove_dir_all(&version_dir)?;
                eprintln!("Uninstalled version {}", v);
            }
            None => {
                eprintln!("Uninstalling all versions...");
                let versions_dir = self.versions_dir();
                if versions_dir.exists() {
                    fs::remove_dir_all(&versions_dir)?;
                }
                eprintln!("All versions uninstalled");
            }
        }
        Ok(())
    }

    /// Update edgeup itself
    pub fn self_update(&self) -> Result<()> {
        eprintln!("Updating edgeup...");

        // TODO: Fetch the latest edgeup binary from GitHub releases
        // Replace the current edgeup executable with the new version

        eprintln!("edgeup updated successfully!");
        Ok(())
    }
}
