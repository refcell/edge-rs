//! Compiler session - holds per-compilation state

use edge_diagnostics::{Diagnostic, DiagnosticBag};

use crate::config::CompilerConfig;

/// A single compiler session
#[derive(Debug)]
pub struct Session {
    /// Compiler configuration
    pub config: CompilerConfig,
    /// Accumulated diagnostics
    pub diagnostics: DiagnosticBag,
    /// The source code being compiled
    pub source: String,
}

impl Session {
    /// Create a new session with the given config and source
    pub fn new(config: CompilerConfig, source: String) -> Self {
        Self {
            config,
            diagnostics: DiagnosticBag::new(),
            source,
        }
    }

    /// Emit an error diagnostic
    pub fn emit_error(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    /// Emit a warning diagnostic
    pub fn emit_warning(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    /// Returns true if there have been any errors
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Print all accumulated diagnostics to stderr via ariadne.
    pub fn report_diagnostics(&self) {
        let path = self.config.input_file.display().to_string();
        self.diagnostics.report_all_with_path(&path, &self.source);
    }
}
