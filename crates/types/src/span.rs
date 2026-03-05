//! Span Module
//!
//! Contains the logic for handling source code spans.

use std::{
    ops::{Add, Range},
    sync::Arc,
};

use crate::source::*;

/// Spanned trait requires a type to have a span.
pub trait Spanned {
    /// Returns a Span.
    fn span(&self) -> Span;
}

/// A Span is a section of a source file.
#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Span {
    /// The start of the span.
    pub start: usize,
    /// The end of the span.
    pub end: usize,
    /// The Associated File
    pub file: Option<Arc<Source>>,
}

impl Span {
    /// An EOF spans [0, 0].
    pub const EOF: Span = Span {
        start: 0,
        end: 0,
        file: None,
    };

    /// Public associated function to instatiate a new span.
    pub fn new(Range { start, end }: Range<usize>, file: Option<Arc<Source>>) -> Self {
        Self { start, end, file }
    }

    /// Converts a span to a range.
    pub fn range(&self) -> Option<Range<usize>> {
        (*self != Self::EOF).then_some(self.start..self.end)
    }

    /// Produces a file identifier string for errors
    pub fn identifier(&self) -> String {
        self.file
            .as_ref()
            .map(|f| format!("\n-> {}:{}-{}", f.path, self.start, self.end))
            .unwrap_or_default()
    }

    /// Produces a source segment string
    pub fn source_seg(&self) -> String {
        self.file
            .as_ref()
            .map(|f| {
                f.source
                    .as_ref()
                    .map(|s| {
                        let line_num = &s.as_bytes()[0..self.start]
                            .iter()
                            .filter(|&&c| c == b'\n')
                            .count()
                            + 1;
                        let line_start = &s[0..self.start].rfind('\n').unwrap_or(0);
                        let line_end = self.end
                            + s[self.end..s.len()]
                                .find('\n')
                                .unwrap_or(s.len() - self.end)
                                .to_owned();
                        let padding = (0..line_num.to_string().len())
                            .map(|_| " ")
                            .collect::<String>();
                        format!(
                            "\n     {}|\n  > {} | {}\n     {}|",
                            padding,
                            line_num,
                            &s[line_start.to_owned()..line_end].replace('\n', ""),
                            padding
                        )
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.range()
            .expect("cannot convert EOF span to Range; check for Span::EOF before converting")
    }
}

impl From<Range<usize>> for Span {
    fn from(Range { start, end }: Range<usize>) -> Self {
        Self {
            start,
            end,
            file: None,
        }
    }
}

impl Add for Span {
    type Output = Span;

    fn add(self, rhs: Span) -> Self::Output {
        Span::new(self.start..rhs.end, None)
    }
}
