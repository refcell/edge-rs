//! Converts Edge compiler errors into LSP diagnostics.

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

/// Run the Edge compiler frontend on `source` and return LSP diagnostics.
pub(crate) fn check(source: &str) -> Vec<Diagnostic> {
    // 1. Parse
    let ast = match edge_parser::parse(source) {
        Ok(ast) => ast,
        Err(e) => return vec![parse_error_to_diagnostic(source, &e)],
    };

    // 2. Type check
    if let Err(e) = edge_typeck::TypeChecker::new().check(&ast) {
        return vec![type_error_to_diagnostic(&e)];
    }

    // No errors — clear diagnostics.
    Vec::new()
}

fn parse_error_to_diagnostic(source: &str, err: &edge_parser::ParseError) -> Diagnostic {
    let (range, message) = match err {
        edge_parser::ParseError::UnexpectedToken {
            found,
            expected,
            span,
        } => {
            let start = offset_to_position(source, span.start);
            let end = offset_to_position(source, span.end.saturating_add(1));
            (
                Range::new(start, end),
                format!("unexpected token `{found}`, expected {expected}"),
            )
        }
        edge_parser::ParseError::UnexpectedEof => {
            let pos = eof_position(source);
            (Range::new(pos, pos), "unexpected end of file".to_string())
        }
        edge_parser::ParseError::InvalidTypeSig { message, span } => {
            let start = offset_to_position(source, span.start);
            let end = offset_to_position(source, span.end.saturating_add(1));
            (Range::new(start, end), format!("invalid type: {message}"))
        }
        edge_parser::ParseError::InvalidExpr { message, span } => {
            let start = offset_to_position(source, span.start);
            let end = offset_to_position(source, span.end.saturating_add(1));
            (
                Range::new(start, end),
                format!("invalid expression: {message}"),
            )
        }
        edge_parser::ParseError::InvalidStmt { message, span } => {
            let start = offset_to_position(source, span.start);
            let end = offset_to_position(source, span.end.saturating_add(1));
            (
                Range::new(start, end),
                format!("invalid statement: {message}"),
            )
        }
        edge_parser::ParseError::InvalidPattern { message, span } => {
            let start = offset_to_position(source, span.start);
            let end = offset_to_position(source, span.end.saturating_add(1));
            (
                Range::new(start, end),
                format!("invalid pattern: {message}"),
            )
        }
        edge_parser::ParseError::LexerError(msg) => {
            let pos = Position::new(0, 0);
            (Range::new(pos, pos), format!("lexer error: {msg}"))
        }
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("edge".to_string()),
        message,
        ..Default::default()
    }
}

fn type_error_to_diagnostic(err: &edge_typeck::TypeCheckError) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("edge".to_string()),
        message: err.to_string(),
        ..Default::default()
    }
}

/// Convert a character index into an LSP `Position` (0-indexed line and UTF-16 character offset).
///
/// Edge's lexer counts character indices (via `chars().zip(0..)`), not byte offsets,
/// so we enumerate characters ordinally rather than using `char_indices()`.
fn offset_to_position(source: &str, char_offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.chars().enumerate() {
        if i >= char_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }
    Position::new(line, col)
}

/// Return the position at the end of the source.
fn eof_position(source: &str) -> Position {
    offset_to_position(source, source.len())
}
