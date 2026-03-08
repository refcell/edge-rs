//! Error types for the Edge compilation pipeline.

use std::fmt;

use foundry_compilers::{
    artifacts::{error::SourceLocation, Severity},
    CompilationError,
};

/// Errors that can occur during Edge compilation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EdgeCompilationError {
    /// The error message.
    pub message: String,
    /// Whether this is a warning rather than an error.
    pub is_warning: bool,
}

impl fmt::Display for EdgeCompilationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl CompilationError for EdgeCompilationError {
    fn is_warning(&self) -> bool {
        self.is_warning
    }

    fn is_error(&self) -> bool {
        !self.is_warning
    }

    fn source_location(&self) -> Option<SourceLocation> {
        None
    }

    fn severity(&self) -> Severity {
        if self.is_warning {
            Severity::Warning
        } else {
            Severity::Error
        }
    }

    fn error_code(&self) -> Option<u64> {
        None
    }
}
