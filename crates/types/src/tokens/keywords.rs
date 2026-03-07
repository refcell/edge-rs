//! Keywords
//!
//! This module contains the Edge Language keywords.
//! These are reserved words that have special functionality in the language.

use derive_more::Display;

/// Edge Language Keywords
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq, Hash, Display)]
pub enum Keyword {
    /// A contract definition
    #[display("contract")]
    Contract,
    /// A type declaration
    #[display("type")]
    Type,
    /// A constant variable
    #[display("const")]
    Const,
    /// A function definition
    #[display("fn")]
    Fn,
    /// Specifies that the data is packed
    #[display("packed")]
    Packed,
    /// A trait definition
    #[display("trait")]
    Trait,
    /// An implementation block
    #[display("impl")]
    Impl,
    /// A module definition
    #[display("mod")]
    Module,
    /// A use statement
    #[display("use")]
    Use,
    /// A `Self` keyword
    #[display("Self")]
    Self_,
    /// An abi definition
    #[display("abi")]
    Abi,
    /// A return statement
    #[display("return")]
    Return,
    /// A match statement
    #[display("match")]
    Match,
    /// An if statement
    #[display("if")]
    If,
    /// An else statement
    #[display("else")]
    Else,
    /// For loop declaration
    #[display("for")]
    For,
    /// While loop declaration
    #[display("while")]
    While,
    /// Variable declaration
    #[display("let")]
    Let,
    /// Mutability modifier
    #[display("mut")]
    Mut,
    /// Public visibility
    #[display("pub")]
    Pub,
    /// Loop declaration
    #[display("loop")]
    Loop,
    /// Do-while loop
    #[display("do")]
    Do,
    /// Break loop
    #[display("break")]
    Break,
    /// Continue loop
    #[display("continue")]
    Continue,
    /// Compile-time keyword
    #[display("comptime")]
    Comptime,
    /// Event declaration
    #[display("event")]
    Event,
    /// Indexed modifier for event fields
    #[display("indexed")]
    Indexed,
    /// External function modifier
    #[display("ext")]
    Ext,
    /// Anonymous event modifier
    #[display("anon")]
    Anon,
    /// Pattern matching keyword
    #[display("matches")]
    Matches,
    /// Super keyword for parent scope
    #[display("super")]
    Super,
    /// Emit event
    #[display("emit")]
    Emit,
    /// Inline assembly block
    #[display("asm")]
    Asm,
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
            "asm" => Some(Keyword::Asm),
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
            Keyword::Asm,
        ]
    }
}
