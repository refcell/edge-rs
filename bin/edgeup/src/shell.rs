//! Shell integration for adding edgeup binaries to PATH.

use std::{fs, path::PathBuf};

use anyhow::Result;

/// Supported shell types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
}

/// Shell integration manager
pub struct Shell {
    shell_type: ShellType,
}

impl Shell {
    /// Detect the current shell from environment or default
    pub fn detect() -> Result<Self> {
        let shell_str = std::env::var("SHELL")
            .or_else(|_| std::env::var("SHELL_TYPE"))
            .unwrap_or_else(|_| "/bin/bash".to_string());

        let shell_type = match shell_str.as_str() {
            s if s.contains("zsh") => ShellType::Zsh,
            s if s.contains("fish") => ShellType::Fish,
            s if s.contains("bash") => ShellType::Bash,
            _ => ShellType::Bash, // Default to bash
        };

        Ok(Self { shell_type })
    }

    /// Get the RC file for the detected shell
    pub fn rc_file(&self) -> PathBuf {
        let home = dirs::home_dir().expect("Could not determine home directory");
        match self.shell_type {
            ShellType::Bash => home.join(".bashrc"),
            ShellType::Zsh => home.join(".zshrc"),
            ShellType::Fish => home.join(".config").join("fish").join("config.fish"),
        }
    }

    /// Add the given directory to PATH in the shell RC file
    pub fn add_to_path(&self, bin_dir: &std::path::Path) -> Result<()> {
        let rc_file = self.rc_file();

        // Create the rc file if it doesn't exist
        if !rc_file.exists() {
            fs::File::create(&rc_file)?;
        }

        // Read the current content
        let mut content = fs::read_to_string(&rc_file)?;

        // Check if PATH is already configured for edgeup
        let path_line = match self.shell_type {
            ShellType::Fish => {
                format!("set -gx PATH \"{}\" $PATH", bin_dir.display())
            }
            _ => {
                format!("export PATH=\"{}:$PATH\"", bin_dir.display())
            }
        };

        if content.contains(&path_line) {
            // Already configured
            return Ok(());
        }

        // Append the PATH export if not already present
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&path_line);
        content.push('\n');

        // Write back the file
        fs::write(&rc_file, content)?;

        eprintln!("Updated {} with PATH entry", rc_file.display());

        Ok(())
    }

    /// Get the shell name for display
    pub fn name(&self) -> &'static str {
        match self.shell_type {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
        }
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
