use std::{
    iter::{Peekable, Zip},
    ops::RangeFrom,
    str::Chars,
};

use edge_types::prelude::*;

use crate::errors::*;

/// Defines a context in which the lexing happens.
/// Allows to differientate between EVM types and opcodes that can either
/// be identical or the latter being a substring of the former (example : bytes32 and byte)
#[derive(Debug, PartialEq, Eq)]
pub enum Context {
    /// global context
    Global,
    /// Contract context
    Contract,
}

/// ## Lexer
///
/// The lexer encapsulated in a struct.
#[allow(missing_debug_implementations)]
pub struct Lexer<'a> {
    /// The source code as peekable chars.
    pub chars: Peekable<Zip<Chars<'a>, RangeFrom<u32>>>,
    position: u32,
    /// The previous lexed Token (excluding whitespace).
    pub lookback: Option<Token>,
    /// Bool indicating if we have reached EOF
    pub eof: bool,
    /// Current context.
    pub context: Context,
    /// A character that was consumed speculatively and needs to be re-processed.
    /// Used to handle ambiguous two-character data-location prefixes like `&cd`.
    pending_char: Option<char>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from a source string.
    pub fn new(source: &'a str) -> Self {
        Lexer {
            // We zip with the character index here to ensure the first char has index 0
            chars: source.chars().zip(0..).peekable(),
            position: 0,
            lookback: None,
            eof: false,
            context: Context::Global,
            pending_char: None,
        }
    }

    /// Consumes and returns the next character.
    /// If a character was pushed back via `push_back`, that is returned first.
    pub fn consume(&mut self) -> Option<char> {
        if let Some(c) = self.pending_char.take() {
            return Some(c);
        }
        let (c, index) = self.chars.next()?;
        self.position = index;
        Some(c)
    }

    /// Push a character back so it will be the next one returned by `consume`.
    /// Used to handle ambiguous two-character prefixes without a second-level lookahead.
    fn push_back(&mut self, c: char) {
        debug_assert!(
            self.pending_char.is_none(),
            "push_back called with a character already pending"
        );
        self.pending_char = Some(c);
    }

    /// Try to peek at the next character from the source.
    /// Returns the pending character if one exists, otherwise peeks the real stream.
    pub fn peek(&mut self) -> Option<char> {
        if let Some(c) = self.pending_char {
            return Some(c);
        }
        self.chars.peek().map(|(c, _)| *c)
    }

    /// Checks the previous token kind against the input.
    pub fn checked_lookback(&self, kind: TokenKind) -> bool {
        self.lookback
            .clone()
            .and_then(|t| if t.kind == kind { Some(true) } else { None })
            .is_some()
    }

    /// Keeps consuming tokens as long as the predicate is satisfied.
    pub fn eat_while<F: Fn(char) -> bool>(
        &mut self,
        initial_char: Option<char>,
        predicate: F,
    ) -> (String, u32, u32) {
        let start = self.position;

        // This function is only called when we want to continue consuming a character of the same
        // type. For example, we see a digit and we want to consume the whole integer
        // Therefore, the current character which triggered this function will need to be appended
        let mut word = String::new();
        if let Some(init_char) = initial_char {
            word.push(init_char)
        }

        // Keep checking that we are not at the EOF
        while let Some(peek_char) = self.peek() {
            // Then check for the predicate, if predicate matches append char and increment the
            // cursor If not, return word. The next character will be analyzed on the
            // next iteration of next_token, Which will increment the cursor
            if !predicate(peek_char) {
                return (word, start, self.position);
            }
            word.push(peek_char);

            // If we arrive at this point, then the char has been added to the word and we should
            // increment the cursor
            self.consume();
        }

        (word, start, self.position)
    }

    /// Lexes a hexadecimal integer literal starting with the given initial character.
    pub fn eat_hex_digit(&mut self, initial_char: char) -> TokenResult {
        let (integer_str, mut start, end) = self.eat_while(Some(initial_char), |ch| {
            ch.is_ascii_hexdigit() | (ch == 'x')
        });

        // // TODO: check for sure that we have a correct hex string, eg. 0x56 and not 0x56x34
        // let kind = if self.context == Context::CodeTableBody {
        //     // In codetables, the bytecode provided is of arbitrary length. We pass
        //     // the code as an Ident, and it is appended to the end of the runtime
        //     // bytecode in codegen.
        //     if &integer_str[0..2] == "0x" {
        //         TokenKind::Ident(integer_str[2..].to_owned())
        //     } else {
        //         TokenKind::Ident(integer_str)
        //     }
        // } else {
        //     TokenKind::Literal(str_to_bytes32(integer_str[2..].as_ref()))
        // };

        start += 2;
        let span = Span {
            start: start as usize,
            end: end as usize,
            file: None,
        };
        let literal = str_to_bytes32(integer_str[2..].as_ref()).map_err(|e| {
            LexicalError::new(
                LexicalErrorKind::InvalidHexLiteral(e.to_string()),
                span.clone(),
            )
        })?;
        let kind = TokenKind::Literal(literal);
        Ok(Token { kind, span })
    }

    /// Skips white space. They are not significant in the source language
    pub fn eat_whitespace(&mut self) -> (String, u32, u32) {
        self.eat_while(None, |ch| ch.is_whitespace())
    }

    // pub fn eat_string_literal(&mut self) -> Token {
    //     let (str_literal, start_span, end_span) =
    //         self.eat_while(None, |ch| ch != '"' && ch != '\'');
    //     let str_literal_token = TokenKind::Str(str_literal);
    //     self.consume(); // Advance past the closing quote
    //     str_literal_token.into_span(start_span, end_span + 1)
    // }

    /// Creates a single-character token with the current position span.
    pub fn single_char_token(&self, token_kind: TokenKind) -> TokenResult {
        Ok(token_kind.into_single_span(self.position))
    }

    /// Parse EVM primitive type from string (e.g., "u256", "i8", "b32")
    fn parse_evm_type(word: &str) -> Option<PrimitiveType> {
        if word == "addr" {
            return Some(PrimitiveType::Address);
        }
        if word == "bool" {
            return Some(PrimitiveType::Bool);
        }
        if word == "bit" {
            return Some(PrimitiveType::Bit);
        }

        // Check for u<size>
        if let Some(size_str) = word.strip_prefix('u') {
            if let Ok(size) = size_str.parse::<u16>() {
                if (8..=256).contains(&size) && size % 8 == 0 {
                    return Some(PrimitiveType::UInt(size));
                }
            }
            return None;
        }

        // Check for i<size>
        if let Some(size_str) = word.strip_prefix('i') {
            if let Ok(size) = size_str.parse::<u16>() {
                if (8..=256).contains(&size) && size % 8 == 0 {
                    return Some(PrimitiveType::Int(size));
                }
            }
            return None;
        }

        // Check for b<size>
        if let Some(size_str) = word.strip_prefix('b') {
            if let Ok(size) = size_str.parse::<u8>() {
                if (1..=32).contains(&size) {
                    return Some(PrimitiveType::FixedBytes(size));
                }
            }
            return None;
        }

        None
    }

    /// Consume a numeric literal with optional type suffix
    pub fn eat_digit(&mut self, initial_char: char) -> TokenResult {
        let (integer_str, start, end) =
            self.eat_while(Some(initial_char), |ch| ch.is_ascii_digit() || ch == '_');

        // Check for type suffix (u8, u16, ..., u256, i8, ..., i256)
        let suffix_start = self.position;
        let (suffix_word, _, suffix_end) =
            self.eat_while(None, |c| c.is_alphanumeric() || c == '_');

        if !suffix_word.is_empty() {
            // We have a potential type suffix
            if Self::parse_evm_type(&suffix_word).is_some() {
                // Valid type suffix, consume it
                let span = Span {
                    start: start as usize,
                    end: suffix_end as usize,
                    file: None,
                };
                let literal = decimal_str_to_bytes32(integer_str.replace('_', "").as_ref())
                    .map_err(|e| {
                        LexicalError::new(
                            LexicalErrorKind::InvalidHexLiteral(e.to_string()),
                            span.clone(),
                        )
                    })?;
                return Ok(Token {
                    kind: TokenKind::Literal(literal),
                    span,
                });
            } else {
                // Not a valid type suffix, reset position
                self.position = suffix_start;
            }
        }

        let span = Span {
            start: start as usize,
            end: end as usize,
            file: None,
        };
        let literal =
            decimal_str_to_bytes32(integer_str.replace('_', "").as_ref()).map_err(|e| {
                LexicalError::new(
                    LexicalErrorKind::InvalidHexLiteral(e.to_string()),
                    span.clone(),
                )
            })?;
        Ok(Token {
            kind: TokenKind::Literal(literal),
            span,
        })
    }

    /// Consume a binary literal (0b...)
    pub fn eat_binary(&mut self) -> TokenResult {
        let start = self.position;
        self.consume(); // consume 'b'
        let (binary_str, _, end) = self.eat_while(None, |ch| ch == '0' || ch == '1' || ch == '_');

        // Check for type suffix
        let suffix_start = self.position;
        let (suffix_word, _, suffix_end) =
            self.eat_while(None, |c| c.is_alphanumeric() || c == '_');

        let end_pos = if !suffix_word.is_empty() && Self::parse_evm_type(&suffix_word).is_some() {
            suffix_end
        } else {
            self.position = suffix_start;
            end
        };

        let span = Span {
            start: (start - 1) as usize, // Include the '0'
            end: end_pos as usize,
            file: None,
        };
        let literal = str_to_bytes32(binary_str.replace('_', "").as_ref()).map_err(|e| {
            LexicalError::new(
                LexicalErrorKind::InvalidHexLiteral(e.to_string()),
                span.clone(),
            )
        })?;
        Ok(Token {
            kind: TokenKind::Literal(literal),
            span,
        })
    }

    /// Consume a string literal
    pub fn eat_string_literal(&mut self, quote_char: char) -> TokenResult {
        let start = self.position;
        let mut string_content = String::new();
        let mut escaped = false;

        while let Some(ch) = self.consume() {
            if escaped {
                // Handle escape sequences
                match ch {
                    'n' => string_content.push('\n'),
                    't' => string_content.push('\t'),
                    'r' => string_content.push('\r'),
                    '\\' => string_content.push('\\'),
                    '"' => string_content.push('"'),
                    '\'' => string_content.push('\''),
                    _ => {
                        string_content.push('\\');
                        string_content.push(ch);
                    }
                }
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                let span = Span {
                    start: start as usize,
                    end: self.position as usize,
                    file: None,
                };
                return Ok(Token {
                    kind: TokenKind::StringLiteral(string_content),
                    span,
                });
            } else {
                string_content.push(ch);
            }
        }

        // Unclosed string literal
        Err(LexicalError::new(
            LexicalErrorKind::UnterminatedString,
            Span {
                start: start as usize,
                end: self.position as usize,
                file: None,
            },
        ))
    }

    /// Check if a given keyword follows the keyword rules in the `source`.
    /// If not, it is a `TokenKind::Ident`.
    ///
    /// Rules:
    /// - ...
    pub fn check_keyword_rules(&mut self, _found_kind: &Option<TokenKind>) -> bool {
        true
    }

    /// Advances the lexer and returns the next token in the source.
    pub fn next_token(&mut self) -> TokenResult {
        let ch = if let Some(ch) = self.consume() {
            ch
        } else {
            self.eof = true;
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span {
                    start: (self.position + 1) as usize,
                    end: (self.position + 1) as usize,
                    file: None,
                },
            });
        };

        let token = match ch {
            '/' => {
                let mut comment_string = String::new();
                let start = self.position;
                comment_string.push(ch);
                if let Some(ch2) = self.peek() {
                    match ch2 {
                        // TODO: Add support for /// and //! comments
                        '/' => {
                            // Consume until newline
                            comment_string.push(ch2);
                            let (comment_string, start, end) =
                                self.eat_while(Some(ch), |c| c != '\n');
                            Ok(TokenKind::Comment(comment_string).into_span(start, end))
                        }
                        '*' => {
                            let c = self.consume();
                            comment_string.push(c.unwrap());
                            let mut depth = 1usize;
                            while let Some(c) = self.consume() {
                                match c {
                                    '/' if self.peek() == Some('*') => {
                                        comment_string.push(c);
                                        let c2 = self.consume();
                                        comment_string.push(c2.unwrap());
                                        depth += 1;
                                    }
                                    '*' if self.peek() == Some('/') => {
                                        comment_string.push(c);
                                        let c2 = self.consume();
                                        comment_string.push(c2.unwrap());
                                        depth -= 1;
                                        if depth == 0 {
                                            // This block comment is closed, so for a
                                            // construction like "/* */ */"
                                            // there will be a successfully parsed block comment
                                            // "/* */"
                                            // and " */" will be processed separately.
                                            break;
                                        }
                                    }
                                    _ => {
                                        comment_string.push(c);
                                    }
                                }
                            }

                            Ok(TokenKind::Comment(comment_string).into_span(start, self.position))
                        }
                        _ => self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                            ArithmeticOperator::Div,
                        ))),
                    }
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                        ArithmeticOperator::Div,
                    )))
                }
            }

            // # keywords
            // '#' => {
            //     let (word, start, end) = self.eat_while(Some(ch), |ch| ch.is_ascii_alphabetic());
            //
            //     let mut found_kind: Option<TokenKind> = None;
            //
            //     let keys = [TokenKind::Define, TokenKind::Include];
            //     for kind in keys.into_iter() {
            //         let key = kind.to_string();
            //         let peeked = word.clone();
            //         if key == peeked {
            //             found_kind = Some(kind);
            //             break;
            //         }
            //     }
            //
            //     if let Some(kind) = &found_kind {
            //         Ok(kind.clone().into_span(start, end))
            //     } else if self.context == Context::Global && self.peek().unwrap() == '[' {
            //         Ok(TokenKind::Pound.into_single_span(self.position))
            //     } else {
            //         // Otherwise we don't support # prefixed indentifiers
            //         tracing::error!(target: "lexer", "INVALID '#' CHARACTER USAGE");
            //         return Err(LexicalError::new(
            //             LexicalErrorKind::InvalidCharacter('#'),
            //             Span {
            //                 start: self.position as usize,
            //                 end: self.position as usize,
            //                 file: None,
            //             },
            //         ));
            //     }
            // }

            // Alphabetical characters
            ch if ch.is_alphabetic() || ch.eq(&'_') => {
                let (word, start, end) =
                    self.eat_while(Some(ch), |c| c.is_alphanumeric() || c == '_');

                let mut found_kind: Option<TokenKind> = None;

                // First check for EVM primitive types
                if let Some(prim_type) = Self::parse_evm_type(&word) {
                    found_kind = Some(TokenKind::DataType(DataType::Primitive(prim_type)));
                }

                // If not a type, check for keywords
                if found_kind.is_none() {
                    let keys = Keyword::all();
                    for kind in keys.into_iter() {
                        let key = kind.to_string();
                        let peeked = word.clone();
                        if key == peeked {
                            found_kind = Some(TokenKind::Keyword(kind));
                            break;
                        }
                    }
                }

                // Check to see if the found kind is, in fact, a keyword and not the name of
                // a function. If it is, set `found_kind` to `None` so that it is set to a
                // `TokenKind::Ident` in the following control flow.
                if !self.check_keyword_rules(&found_kind) {
                    found_kind = None;
                }

                if let Some(TokenKind::Keyword(Keyword::Contract)) = &found_kind {
                    self.context = Context::Contract;
                }

                // if let Some(':') = self.peek() {
                //     found_kind = Some(TokenKind::Label(word.clone()));
                // }

                // Syntax sugar: true evaluates to 0x01, false evaluates to 0x00
                if matches!(word.as_str(), "true" | "false") {
                    found_kind = Some(TokenKind::Literal(
                        str_to_bytes32(if word.as_str() == "true" { "1" } else { "0" })
                            .expect("single hex digit is always valid"),
                    ));
                    self.eat_while(None, |c| c.is_alphanumeric());
                }

                // if !(self.context != Context::MacroBody || found_kind.is_some()) {
                //     if let Some(o) = OPCODES_MAP.get(&word) {
                //         found_kind = Some(TokenKind::Opcode(o.to_owned()));
                //     }
                // }

                let kind = if let Some(kind) = &found_kind {
                    kind.clone()
                } else {
                    TokenKind::Ident(word)
                };

                Ok(kind.into_span(start, end))
            }

            // Decimal digits
            ch if ch.is_ascii_digit() => self.eat_digit(ch),
            '{' => self.single_char_token(TokenKind::OpenBrace),
            '}' => self.single_char_token(TokenKind::CloseBrace),
            '(' => self.single_char_token(TokenKind::OpenParen),
            ')' => self.single_char_token(TokenKind::CloseParen),
            '[' => self.single_char_token(TokenKind::OpenBracket),
            ']' => self.single_char_token(TokenKind::CloseBracket),
            ',' => self.single_char_token(TokenKind::Comma),
            ';' => self.single_char_token(TokenKind::Semicolon),

            // Operators and special tokens
            '+' => {
                if self.peek() == Some('=') {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::AddAssign,
                    )))
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                        ArithmeticOperator::Add,
                    )))
                }
            }

            '-' => match self.peek() {
                Some('=') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::SubAssign,
                    )))
                }
                Some('>') => {
                    self.consume();
                    self.single_char_token(TokenKind::Arrow)
                }
                _ => self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                    ArithmeticOperator::Sub,
                ))),
            },

            '*' => {
                if self.peek() == Some('*') {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                            CompoundAssignmentOperator::ExpAssign,
                        )))
                    } else {
                        self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                            ArithmeticOperator::Exp,
                        )))
                    }
                } else if self.peek() == Some('=') {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::MulAssign,
                    )))
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                        ArithmeticOperator::Mul,
                    )))
                }
            }

            '%' => {
                if self.peek() == Some('=') {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::ModAssign,
                    )))
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Arithmetic(
                        ArithmeticOperator::Mod,
                    )))
                }
            }

            '&' => {
                // Check for data location annotations first (&s, &t, &m, &cd, &rd, &ic, &ec)
                // or compound assignment (&=) or logical AND (&&)
                if let Some(next) = self.peek() {
                    match next {
                        's' => {
                            self.consume();
                            self.single_char_token(TokenKind::Pointer(Location::PersistentStorage))
                        }
                        't' => {
                            self.consume();
                            self.single_char_token(TokenKind::Pointer(Location::TransientStorage))
                        }
                        'm' => {
                            self.consume();
                            self.single_char_token(TokenKind::Pointer(Location::Memory))
                        }
                        'c' => {
                            // Peek ahead to see if it's &cd
                            self.consume(); // consume 'c'
                            if self.peek() == Some('d') {
                                self.consume(); // consume 'd'
                                self.single_char_token(TokenKind::Pointer(Location::Calldata))
                            } else {
                                // Not &cd — push 'c' back so the next token starts with it
                                self.push_back('c');
                                self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                                    BitwiseOperator::And,
                                )))
                            }
                        }
                        'r' => {
                            self.consume(); // consume 'r'
                            if self.peek() == Some('d') {
                                self.consume(); // consume 'd'
                                self.single_char_token(TokenKind::Pointer(Location::Returndata))
                            } else {
                                self.push_back('r');
                                self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                                    BitwiseOperator::And,
                                )))
                            }
                        }
                        'i' => {
                            self.consume(); // consume 'i'
                            if self.peek() == Some('c') {
                                self.consume(); // consume 'c'
                                self.single_char_token(TokenKind::Pointer(Location::InternalCode))
                            } else {
                                self.push_back('i');
                                self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                                    BitwiseOperator::And,
                                )))
                            }
                        }
                        'e' => {
                            self.consume(); // consume 'e'
                            if self.peek() == Some('c') {
                                self.consume(); // consume 'c'
                                self.single_char_token(TokenKind::Pointer(Location::ExternalCode))
                            } else {
                                self.push_back('e');
                                self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                                    BitwiseOperator::And,
                                )))
                            }
                        }
                        '=' => {
                            self.consume();
                            self.single_char_token(TokenKind::Operator(
                                Operator::CompoundAssignment(CompoundAssignmentOperator::AndAssign),
                            ))
                        }
                        '&' => {
                            self.consume();
                            self.single_char_token(TokenKind::Operator(Operator::Logical(
                                LogicalOperator::And,
                            )))
                        }
                        _ => self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                            BitwiseOperator::And,
                        ))),
                    }
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                        BitwiseOperator::And,
                    )))
                }
            }

            '|' => match self.peek() {
                Some('=') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::OrAssign,
                    )))
                }
                Some('|') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::Logical(
                        LogicalOperator::Or,
                    )))
                }
                _ => self
                    .single_char_token(TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Or))),
            },

            '^' => {
                if self.peek() == Some('=') {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                        CompoundAssignmentOperator::XorAssign,
                    )))
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                        BitwiseOperator::Xor,
                    )))
                }
            }

            '~' => {
                self.single_char_token(TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Not)))
            }

            '!' => {
                if self.peek() == Some('=') {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::Comparison(
                        ComparisonOperator::NotEqual,
                    )))
                } else {
                    self.single_char_token(TokenKind::Operator(Operator::Logical(
                        LogicalOperator::Not,
                    )))
                }
            }

            '<' => match self.peek() {
                Some('<') => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                            CompoundAssignmentOperator::ShlAssign,
                        )))
                    } else {
                        self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                            BitwiseOperator::LeftShift,
                        )))
                    }
                }
                Some('=') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::Comparison(
                        ComparisonOperator::LessThanOrEqual,
                    )))
                }
                _ => self.single_char_token(TokenKind::Operator(Operator::Comparison(
                    ComparisonOperator::LessThan,
                ))),
            },

            '>' => match self.peek() {
                Some('>') => {
                    self.consume();
                    if self.peek() == Some('=') {
                        self.consume();
                        self.single_char_token(TokenKind::Operator(Operator::CompoundAssignment(
                            CompoundAssignmentOperator::ShrAssign,
                        )))
                    } else {
                        self.single_char_token(TokenKind::Operator(Operator::Bitwise(
                            BitwiseOperator::RightShift,
                        )))
                    }
                }
                Some('=') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::Comparison(
                        ComparisonOperator::GreaterThanOrEqual,
                    )))
                }
                _ => self.single_char_token(TokenKind::Operator(Operator::Comparison(
                    ComparisonOperator::GreaterThan,
                ))),
            },

            '=' => match self.peek() {
                Some('=') => {
                    self.consume();
                    self.single_char_token(TokenKind::Operator(Operator::Comparison(
                        ComparisonOperator::Equal,
                    )))
                }
                Some('>') => {
                    self.consume();
                    self.single_char_token(TokenKind::FatArrow)
                }
                _ => self.single_char_token(TokenKind::Operator(Operator::Assignment)),
            },

            '.' => self.single_char_token(TokenKind::Dot),

            '?' => self.single_char_token(TokenKind::Question),

            '@' => self.single_char_token(TokenKind::At),

            ':' => {
                if self.peek() == Some(':') {
                    self.consume();
                    self.single_char_token(TokenKind::DoubleColon)
                } else {
                    self.single_char_token(TokenKind::Colon)
                }
            }

            '"' | '\'' => self.eat_string_literal(ch),

            '0' => match self.peek() {
                Some('x') | Some('X') => {
                    self.consume();
                    self.eat_hex_digit('0')
                }
                Some('b') | Some('B') => {
                    self.consume();
                    self.eat_binary()
                }
                Some(c) if c.is_ascii_digit() => self.eat_digit(ch),
                _ => self.eat_digit(ch),
            },

            ch if ch.is_ascii_whitespace() => {
                let (_, start, end) = self.eat_whitespace();
                Ok(TokenKind::Whitespace.into_span(start, end))
            }
            ch => {
                tracing::error!(target: "lexer", "UNSUPPORTED TOKEN '{}'", ch);
                return Err(LexicalError::new(
                    LexicalErrorKind::InvalidCharacter(ch),
                    Span {
                        start: self.position as usize,
                        end: self.position as usize,
                        file: None,
                    },
                ));
            }
        }?;

        if token.kind != TokenKind::Whitespace {
            self.lookback = Some(token.clone());
        }

        Ok(token)
    }
}

/// Result type for lexer operations.
pub type TokenResult = Result<Token, LexicalError>;

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenResult;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            None
        } else {
            Some(self.next_token())
        }
    }
}
