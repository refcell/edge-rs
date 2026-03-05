//! Keywords
//!
//! This module contains the Edge Language keywords.
//! These are reserved words that have special functionality in the language.

use std::fmt;

/// Edge Language Keywords
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// A contract definition
    Contract,
    /// A type declaration
    Type,
    /// A constant variable
    Const,
    /// A function definition
    Fn,
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
    /// Variable declaration
    Let,
    /// Mutability modifier
    Mut,
    /// Public visibility
    Pub,
    /// Loop declaration
    Loop,
    /// Do-while loop
    Do,
    /// Break loop
    Break,
    /// Continue loop
    Continue,
    /// Compile-time keyword
    Comptime,
    /// Event declaration
    Event,
    /// Indexed modifier for event fields
    Indexed,
    /// External function modifier
    Ext,
    /// Anonymous event modifier
    Anon,
    /// Pattern matching keyword
    Matches,
    /// Super keyword for parent scope
    Super,
    /// Emit event
    Emit,
}

impl Keyword {
    /// Map a raw identifier string to a keyword without any heap allocation.
    ///
    /// Returns `None` when the string is not a reserved keyword.
    pub fn from_word(word: &str) -> Option<Keyword> {
        match word {
            "contract" => Some(Keyword::Contract),
            "type" => Some(Keyword::Type),
            "const" => Some(Keyword::Const),
            "fn" => Some(Keyword::Fn),
            "packed" => Some(Keyword::Packed),
            "trait" => Some(Keyword::Trait),
            "impl" => Some(Keyword::Impl),
            "mod" => Some(Keyword::Module),
            "use" => Some(Keyword::Use),
            "Self" => Some(Keyword::Self_),
            "abi" => Some(Keyword::Abi),
            "return" => Some(Keyword::Return),
            "match" => Some(Keyword::Match),
            "if" => Some(Keyword::If),
            "else" => Some(Keyword::Else),
            "for" => Some(Keyword::For),
            "while" => Some(Keyword::While),
            "let" => Some(Keyword::Let),
            "mut" => Some(Keyword::Mut),
            "pub" => Some(Keyword::Pub),
            "loop" => Some(Keyword::Loop),
            "do" => Some(Keyword::Do),
            "break" => Some(Keyword::Break),
            "continue" => Some(Keyword::Continue),
            "comptime" => Some(Keyword::Comptime),
            "event" => Some(Keyword::Event),
            "indexed" => Some(Keyword::Indexed),
            "ext" => Some(Keyword::Ext),
            "anon" => Some(Keyword::Anon),
            "matches" => Some(Keyword::Matches),
            "super" => Some(Keyword::Super),
            "emit" => Some(Keyword::Emit),
            _ => None,
        }
    }

    /// Returns a list of all the keyword variants.
    pub fn all() -> Vec<Keyword> {
        // todo: use strum to return an iterator over the variants
        vec![
            Keyword::Contract,
            Keyword::Type,
            Keyword::Const,
            Keyword::Fn,
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
            Keyword::Let,
            Keyword::Mut,
            Keyword::Pub,
            Keyword::Loop,
            Keyword::Do,
            Keyword::Break,
            Keyword::Continue,
            Keyword::Comptime,
            Keyword::Event,
            Keyword::Indexed,
            Keyword::Ext,
            Keyword::Anon,
            Keyword::Matches,
            Keyword::Super,
            Keyword::Emit,
        ]
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let x = match self {
            Keyword::Contract => "contract",
            Keyword::Type => "type",
            Keyword::Const => "const",
            Keyword::Fn => "fn",
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
            Keyword::Let => "let",
            Keyword::Mut => "mut",
            Keyword::Pub => "pub",
            Keyword::Loop => "loop",
            Keyword::Do => "do",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Comptime => "comptime",
            Keyword::Event => "event",
            Keyword::Indexed => "indexed",
            Keyword::Ext => "ext",
            Keyword::Anon => "anon",
            Keyword::Matches => "matches",
            Keyword::Super => "super",
            Keyword::Emit => "emit",
        };
        write!(f, "{}", x)
    }
}
