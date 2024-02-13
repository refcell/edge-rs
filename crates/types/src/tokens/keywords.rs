//! Keywords
//!
//! This module contains the Edge Language keywords.
//! These are reserved words that have special functionality in the language.

use std::fmt;

/// Edge Language Keywords
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum Keyword {
    /// A contract definition
    Contract,
    /// A type declaration
    Type,
    /// A constant variable
    Const,
    /// A function identifier
    Function,
    /// Specifies that the data is packed
    Packed,
    /// A trait definition
    Trait,
    /// An implementation block
    Impl,
    /// A module definition
    Module,
    /// A use statement
    Use,
    /// A `Self` keyword
    Self_,
    /// An abi definition
    Abi,
    /// A return statement
    Return,
    /// A match statement
    Match,
    /// An if statement
    If,
    /// An else statement
    Else,
    /// For loop declaration
    For,
    /// While loop declaration
    While,
}

impl Keyword {
    /// Returns a list of all the keyword variants.
    pub fn all() -> Vec<Keyword> {
        // todo: use strum to return an iterator over the variants
        vec![
            Keyword::Contract,
            Keyword::Type,
            Keyword::Const,
            Keyword::Function,
            Keyword::Packed,
            Keyword::Trait,
            Keyword::Impl,
            Keyword::Module,
            Keyword::Use,
            Keyword::Self_,
            Keyword::Abi,
            Keyword::Return,
            Keyword::Match,
            Keyword::If,
            Keyword::Else,
            Keyword::For,
            Keyword::While,
        ]
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            Keyword::Contract => "contract",
            Keyword::Type => "type",
            Keyword::Const => "const",
            Keyword::Function => "function",
            Keyword::Packed => "packed",
            Keyword::Trait => "trait",
            Keyword::Impl => "impl",
            Keyword::Module => "mod",
            Keyword::Use => "use",
            Keyword::Self_ => "Self",
            Keyword::Abi => "abi",
            Keyword::Return => "return",
            Keyword::Match => "match",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::For => "for",
            Keyword::While => "while",
        };
        write!(f, "{}", x)
    }
}
