//! Edge Language Diagnostics
//!
//! Provides error reporting and diagnostic infrastructure for the Edge compiler.
//! Uses [ariadne](https://docs.rs/ariadne) for pretty-printed diagnostics.
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
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

    /// Add a primary label pointing to a source location (colored by diagnostic severity)
    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            message: message.into(),
            span,
            severity: self.severity,
        });
        self
    }

    /// Add a secondary/info label (blue) pointing to a source location
    pub fn with_help_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            message: message.into(),
            span,
            severity: Severity::Note,
        });
        self
    }

    /// Add a note appended after the diagnostic
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
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Returns the number of error diagnostics
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Returns the number of warning diagnostics
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Returns all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Print all diagnostics to stderr using ariadne for pretty output.
    ///
    /// `source` is the main source text, used as fallback when a span has no
    /// associated file.
    pub fn report_all(&self, source: &str) {
        self.report_all_with_path("<input>", source);
    }

    /// Print all diagnostics to stderr using ariadne, with an explicit file path
    /// for the primary source.
    pub fn report_all_with_path(&self, path: &str, source: &str) {
        use std::collections::HashMap;

        // Collect all unique sources referenced by spans.
        let mut source_map: HashMap<String, String> = HashMap::new();
        source_map.insert(path.to_string(), source.to_string());

        for diag in &self.diagnostics {
            for label in &diag.labels {
                if let Some(ref file) = label.span.file {
                    if let Some(ref src) = file.source {
                        source_map
                            .entry(file.path.clone())
                            .or_insert_with(|| src.clone());
                    }
                }
            }
        }

        let mut cache = ariadne::sources(source_map);

        for diag in &self.diagnostics {
            let kind = match diag.severity {
                Severity::Error => ariadne::ReportKind::Error,
                Severity::Warning => ariadne::ReportKind::Warning,
                Severity::Note => ariadne::ReportKind::Advice,
            };

            // Determine the primary span for the report header.
            // Prefer the first label matching the diagnostic severity (the primary error site).
            let primary_label = diag
                .labels
                .iter()
                .find(|l| l.severity == diag.severity)
                .or_else(|| diag.labels.first());
            let (file_id, offset) = primary_label.map_or_else(
                || (path.to_string(), 0),
                |label| {
                    let fid = label
                        .span
                        .file
                        .as_ref()
                        .map_or_else(|| path.to_string(), |f| f.path.clone());
                    (fid, label.span.start)
                },
            );

            let mut builder = ariadne::Report::build(kind, (file_id.clone(), offset..offset))
                .with_message(&diag.message);

            for (i, label) in diag.labels.iter().enumerate() {
                let fid = label
                    .span
                    .file
                    .as_ref()
                    .map_or_else(|| path.to_string(), |f| f.path.clone());
                let start = label.span.start;
                // Our spans use inclusive end; ariadne uses exclusive ranges
                let end = (label.span.end + 1).max(start + 1);
                let color = match label.severity {
                    Severity::Error => ariadne::Color::Red,
                    Severity::Warning => ariadne::Color::Yellow,
                    Severity::Note => ariadne::Color::Blue,
                };
                builder.add_label(
                    ariadne::Label::new((fid, start..end))
                        .with_message(&label.message)
                        .with_color(color)
                        .with_order(i as i32),
                );
            }

            for note in &diag.notes {
                builder.add_note(note);
            }

            let report = builder.finish();
            let _ = report.eprint(&mut cache);
        }
    }
}
