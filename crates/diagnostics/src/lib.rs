//! Edge Language Diagnostics
//!
//! Provides error reporting and diagnostic infrastructure for the Edge compiler.
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all,
)]
#![deny(unused_must_use, rust_2018_idioms)]

use edge_types::span::Span;

/// Diagnostic severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// A note providing additional context
    Note,
    /// A warning - compilation continues
    Warning,
    /// An error - compilation fails
    Error,
}

/// A label pointing to a source location with a message
#[derive(Debug, Clone)]
pub struct Label {
    /// The message for this label
    pub message: String,
    /// The source span this label points to
    pub span: Span,
    /// The severity of this label
    pub severity: Severity,
}

/// A compiler diagnostic with optional source labels and notes
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The severity of this diagnostic
    pub severity: Severity,
    /// The primary message
    pub message: String,
    /// Source labels pointing to relevant code locations
    pub labels: Vec<Label>,
    /// Additional notes appended to the diagnostic
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Add a label pointing to a source location
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            message: message.into(),
            span,
            severity: self.severity,
        });
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// A collection of diagnostics accumulated during compilation
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    /// Create a new empty bag
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a diagnostic
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Returns true if there are any error-level diagnostics
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Returns the number of error diagnostics
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }

    /// Returns the number of warning diagnostics
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }

    /// Returns all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Print all diagnostics to stderr with source context
    pub fn report_all(&self, source: &str) {
        for diag in &self.diagnostics {
            let prefix = match diag.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Note => "note",
            };
            eprintln!("{}: {}", prefix, diag.message);

            for label in &diag.labels {
                let line_num = source[..label.span.start.min(source.len())]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count()
                    + 1;
                let line_start = source[..label.span.start.min(source.len())]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line_end = source[label.span.start.min(source.len())..]
                    .find('\n')
                    .map(|i| label.span.start + i)
                    .unwrap_or(source.len());
                let line = &source[line_start..line_end.min(source.len())];
                let col = label.span.start.saturating_sub(line_start) + 1;
                let len = (label.span.end.saturating_sub(label.span.start)).max(1);

                eprintln!("  --> line {line_num}:{col}");
                eprintln!("   |");
                eprintln!("   | {line}");
                eprintln!("   | {}{} {}", " ".repeat(col.saturating_sub(1)), "^".repeat(len), label.message);
                eprintln!("   |");
            }

            for note in &diag.notes {
                eprintln!("   = note: {note}");
            }
        }
    }
}
