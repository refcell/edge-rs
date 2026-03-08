//! Installer logic for the Edge toolchain version manager.
//!
//! This module provides the core [`Installer`] type for downloading,
//! resolving, and managing Edge toolchain versions from GitHub releases.

use std::{fs, path::PathBuf};

use anyhow::{anyhow, Result};
use serde::Deserialize;

/// GitHub repository used for release lookups.
const GITHUB_REPO: &str = "refcell/edge-rs";

/// Release information from the GitHub API.
#[derive(Debug, Deserialize)]
pub struct GithubRelease {
    /// The tag name for the release (e.g. "v0.1.6").
    pub tag_name: String,
    /// Assets attached to the release.
    pub assets: Vec<GithubAsset>,
}

/// A single asset attached to a GitHub release.
#[derive(Debug, Deserialize)]
pub struct GithubAsset {
    /// The filename of the asset.
    pub name: String,
    /// Direct download URL for the asset.
    pub browser_download_url: String,
}

/// Return the platform suffix matching the current OS and architecture.
///
/// These correspond to the Rust target triples used in the release workflow.
pub const fn platform_suffix() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "x86_64-apple-darwin"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    {
        compile_error!("unsupported platform");
    }
}

/// Fetch release metadata from GitHub.
///
/// Pass `None` or `Some("latest")` to get the latest release,
/// or `Some("v0.1.6")` for a specific tag.
pub(crate) fn fetch_release(version: Option<&str>) -> Result<GithubRelease> {
    let url = match version {
        None | Some("latest") => {
            format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest")
        }
        Some(v) => {
            format!("https://api.github.com/repos/{GITHUB_REPO}/releases/tags/{v}")
        }
    };
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("edgeup-lib/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let resp = client.get(&url).send()?.error_for_status()?;
    Ok(resp.json::<GithubRelease>()?)
}

/// Download the raw bytes of a release asset.
pub(crate) fn download_asset(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("edgeup-lib/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let bytes = client.get(url).send()?.error_for_status()?.bytes()?;
    Ok(bytes.to_vec())
}

/// Installer for managing Edge toolchain versions.
///
/// Manages a directory layout under the installation root:
///
/// ```text
/// <install_dir>/
///   bin/
///     edgec -> ../versions/<tag>/edgec   (symlink)
///   versions/
///     v0.1.0/edgec
///     v0.1.1/edgec
/// ```
#[derive(Debug)]
pub struct Installer {
    /// Installation directory (~/.edgeup by default).
    install_dir: PathBuf,
}

impl Installer {
    /// Create a new installer instance using the default install directory
    /// (`~/.edgeup`).
    pub fn new() -> Result<Self> {
        let install_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".edgeup");

        // Create install directory if it doesn't exist.
        fs::create_dir_all(&install_dir)?;

        Ok(Self { install_dir })
    }

    /// Create an installer rooted at an arbitrary directory.
    ///
    /// Useful for testing without touching `~/.edgeup`.
    pub const fn with_dir(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    /// Get the bin directory (`<install_dir>/bin`).
    fn bin_dir(&self) -> PathBuf {
        self.install_dir.join("bin")
    }

    /// Get the versions directory (`<install_dir>/versions`).
    fn versions_dir(&self) -> PathBuf {
        self.install_dir.join("versions")
    }

    /// Get the path to the edgec binary symlink.
    fn edgec_bin(&self) -> PathBuf {
        self.bin_dir().join("edgec")
    }

    /// Install a specific version of the Edge toolchain.
    ///
    /// Downloads the release binary for the current platform and writes it
    /// to `<install_dir>/versions/<tag>/edgec`, then points the active
    /// symlink at this version.
    ///
    /// This is the core install logic without any shell PATH manipulation.
    pub fn install_version(&self, version: &str) -> Result<()> {
        let query = if version == "latest" {
            None
        } else {
            Some(version)
        };

        eprintln!(
            "Installing Edge toolchain (version: {})...",
            query.unwrap_or("latest")
        );

        let release = fetch_release(query)?;
        let suffix = platform_suffix();

        // Find the asset whose name contains the platform suffix.
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.contains(suffix))
            .ok_or_else(|| {
                anyhow!(
                    "no release asset found for platform {suffix} in release {}",
                    release.tag_name
                )
            })?;

        eprintln!("Downloading {}...", asset.name);
        let bytes = download_asset(&asset.browser_download_url)?;

        // Write binary to <install_dir>/versions/{tag_name}/edgec.
        let version_dir = self.versions_dir().join(&release.tag_name);
        fs::create_dir_all(&version_dir)?;

        let binary_path = version_dir.join("edgec");
        fs::write(&binary_path, &bytes)?;

        // Make binary executable on Unix.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755))?;
        }

        // Point the symlink at this version.
        self.use_version(&release.tag_name)?;

        eprintln!("Installation complete!");

        Ok(())
    }

    /// Install a version of the Edge toolchain (backward-compatible wrapper).
    ///
    /// Pass `None` to install the latest version, or `Some("v0.1.6")` for a
    /// specific tag. Delegates to [`Self::install_version`].
    pub fn install(&self, version: Option<String>) -> Result<()> {
        let tag = version.as_deref().unwrap_or("latest");
        self.install_version(tag)
    }

    /// Resolve a version tag to the absolute path of the `edgec` binary.
    ///
    /// If the requested version is already installed, returns the path
    /// immediately. Otherwise downloads and installs the version first.
    ///
    /// When `version` is `"latest"`, the latest release tag is fetched from
    /// GitHub before checking local installs.
    pub fn resolve_version(&self, version: &str) -> Result<PathBuf> {
        let tag = if version == "latest" {
            fetch_release(Some("latest"))?.tag_name
        } else {
            version.to_string()
        };

        let binary = self.versions_dir().join(&tag).join("edgec");
        if binary.exists() {
            return Ok(binary);
        }

        // Not installed yet -- install it.
        self.install_version(&tag)?;
        Ok(self.versions_dir().join(&tag).join("edgec"))
    }

    /// Return a sorted list of installed version tag strings.
    ///
    /// Scans `<install_dir>/versions/` for subdirectories and returns their
    /// names. Returns an empty `Vec` if no versions are installed.
    pub fn list_installed_versions(&self) -> Result<Vec<String>> {
        let versions_dir = self.versions_dir();

        if !versions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in fs::read_dir(&versions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(version) = path.file_name().and_then(|n| n.to_str()) {
                    versions.push(version.to_string());
                }
            }
        }
        versions.sort();
        Ok(versions)
    }

    /// Update to the latest version.
    pub fn update(&self) -> Result<()> {
        self.install(None)
    }

    /// List installed versions to stdout (for CLI display).
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
                    println!("  - {version}");
                }
            }
        }

        Ok(())
    }

    /// Switch to a specific installed version.
    ///
    /// Creates (or replaces) the `edgec` symlink in `<install_dir>/bin/`
    /// to point at the given version's binary.
    pub fn use_version(&self, version: &str) -> Result<()> {
        let version_dir = self.versions_dir().join(version);

        if !version_dir.exists() {
            return Err(anyhow!("Version {version} is not installed"));
        }

        eprintln!("Switching to version {version}...");

        // Create symlink from <install_dir>/bin/edgec to the selected version.
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

        eprintln!("Switched to version {version}");
        Ok(())
    }

    /// Uninstall a version (or all versions if `None`).
    pub fn uninstall(&self, version: Option<String>) -> Result<()> {
        match version {
            Some(v) => {
                let version_dir = self.versions_dir().join(&v);
                if !version_dir.exists() {
                    return Err(anyhow!("Version {v} is not installed"));
                }
                eprintln!("Uninstalling version {v}...");
                fs::remove_dir_all(&version_dir)?;
                eprintln!("Uninstalled version {v}");
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

    /// Update edgeup itself by downloading the latest release binary.
    pub fn self_update(&self) -> Result<()> {
        eprintln!("Updating edgeup...");

        let release = fetch_release(None)?;
        let suffix = platform_suffix();
        let target_name = format!("edgeup-{suffix}");

        // Find the edgeup asset for this platform.
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.contains(&target_name))
            .ok_or_else(|| {
                anyhow!(
                    "no edgeup asset found for platform {suffix} in release {}",
                    release.tag_name
                )
            })?;

        eprintln!("Downloading {}...", asset.name);
        let bytes = download_asset(&asset.browser_download_url)?;

        // Atomically replace the current executable:
        // write to a temp file next to the exe, then rename.
        let current_exe = std::env::current_exe()?;
        let temp_path = current_exe.with_extension("tmp");
        fs::write(&temp_path, &bytes)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temp_path, fs::Permissions::from_mode(0o755))?;
        }

        fs::rename(&temp_path, &current_exe)?;

        eprintln!("edgeup updated successfully!");
        Ok(())
    }
}
