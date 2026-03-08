//! Programmatic version resolution and management for the Edge toolchain.
//!
//! This crate provides the core version-management logic used by the
//! `edgeup` CLI, exposed as a library so other tools (IDE integrations,
//! build scripts, test harnesses) can resolve and install Edge toolchain
//! versions programmatically.
//!
//! # Quick start
//!
//! ```rust,no_run
//! // Resolve a specific version, installing it if needed:
//! let edgec = edgeup_lib::resolve_version("v0.1.18").unwrap();
//! println!("edgec binary at: {}", edgec.display());
//!
//! // List all locally installed versions:
//! let versions = edgeup_lib::list_installed().unwrap();
//! for v in &versions {
//!     println!("  {v}");
//! }
//! ```

mod installer;

#[cfg(test)]
mod tests;

pub use installer::{platform_suffix, GithubAsset, GithubRelease, Installer};

/// Resolve a version tag to the absolute path of the `edgec` binary.
///
/// If the requested version is already installed locally it is returned
/// immediately; otherwise it is downloaded and installed first.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined, the
/// GitHub API is unreachable, or the download / install fails.
pub fn resolve_version(version: &str) -> anyhow::Result<std::path::PathBuf> {
    Installer::new()?.resolve_version(version)
}

/// Install a specific version of the Edge toolchain.
///
/// Downloads the release binary for the current platform and writes it
/// to `~/.edgeup/versions/<tag>/edgec`.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined, the
/// GitHub API is unreachable, or the download / install fails.
pub fn install_version(version: &str) -> anyhow::Result<()> {
    Installer::new()?.install_version(version)
}

/// Return a sorted list of installed version tag strings.
///
/// Scans `~/.edgeup/versions/` for subdirectories and returns their
/// names. Returns an empty `Vec` if no versions are installed.
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined or the
/// versions directory cannot be read.
pub fn list_installed() -> anyhow::Result<Vec<String>> {
    Installer::new()?.list_installed_versions()
}
