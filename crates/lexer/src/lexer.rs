use crate::errors::*;
use edge_types::prelude::*;
use std::{
    iter::{Peekable, Zip},
    ops::RangeFrom,
    str::Chars,
};

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
        }
    }

    /// Consumes and returns the next character.
    pub fn consume(&mut self) -> Option<char> {
        let (c, index) = self.chars.next()?;
        self.position = index;
        Some(c)
    }

    /// Try to peek at the next character from the source.
    pub fn peek(&mut self) -> Option<char> {
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

        let kind = TokenKind::Literal(str_to_bytes32(integer_str[2..].as_ref()));

        start += 2;
        let span = Span {
            start: start as usize,
            end: end as usize,
            file: None,
        };
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

    pub fn single_char_token(&self, token_kind: TokenKind) -> TokenResult {
        Ok(token_kind.into_single_span(self.position))
    }

    // pub fn eat_digit(&mut self, initial_char: char) -> TokenResult {
    //     let (integer_str, start, end) =
    //         self.eat_while(Some(initial_char), |ch| ch.is_ascii_digit());
    //     let integer = integer_str.parse().unwrap();
    //     let integer_token = TokenKind::Num(integer);
    //     let span = Span {
    //         start: start as usize,
    //         end: end as usize,
    //         file: None,
    //     };
    //     Ok(Token {
    //         kind: integer_token,
    //         span,
    //     })
    // }

    /// Check if a given keyword follows the keyword rules in the `source`.
    /// If not, it is a `TokenKind::Ident`.
    ///
    /// Rules:
    /// - ...
    pub fn check_keyword_rules(&mut self, found_kind: &Option<TokenKind>) -> bool {
        match found_kind {
            // TODO: Add keyword rules here
            _ => true,
        }
    }

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
                        _ => self.single_char_token(TokenKind::Div),
                    }
                } else {
                    self.single_char_token(TokenKind::Div)
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
                let (word, start, mut end) =
                    self.eat_while(Some(ch), |c| c.is_alphanumeric() || c == '_');

                let mut found_kind: Option<TokenKind> = None;
                let keys = Keyword::all();
                for kind in keys.into_iter() {
                    let key = kind.to_string();
                    let peeked = word.clone();
                    if key == peeked {
                        found_kind = Some(TokenKind::Keyword(kind));
                        break;
                    }
                }

                // Check to see if the found kind is, in fact, a keyword and not the name of
                // a function. If it is, set `found_kind` to `None` so that it is set to a
                // `TokenKind::Ident` in the following control flow.
                if !self.check_keyword_rules(&found_kind) {
                    found_kind = None;
                }

                if let Some(kind) = &found_kind {
                    match kind {
                        TokenKind::Keyword(Keyword::Contract) => self.context = Context::Contract,
                        // TokenKind::Macro | TokenKind::Fn | TokenKind::Test => {
                        //     self.context = Context::MacroDefinition
                        // }
                        // TokenKind::Function | TokenKind::Event | TokenKind::Error => {
                        //     self.context = Context::Abi
                        // }
                        // TokenKind::Constant => self.context = Context::Constant,
                        // TokenKind::CodeTable => self.context = Context::CodeTableBody,
                        _ => (),
                    }
                }

                // if let Some(':') = self.peek() {
                //     found_kind = Some(TokenKind::Label(word.clone()));
                // }

                // Syntax sugar: true evaluates to 0x01, false evaluates to 0x00
                if matches!(word.as_str(), "true" | "false") {
                    found_kind = Some(TokenKind::Literal(str_to_bytes32(
                        if word.as_str() == "true" { "1" } else { "0" },
                    )));
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
            '{' => self.single_char_token(TokenKind::OpenBrace),
            '}' => self.single_char_token(TokenKind::CloseBrace),
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
