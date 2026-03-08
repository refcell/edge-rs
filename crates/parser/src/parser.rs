//! The Edge Language Parser
//!
//! Implements a recursive descent parser with Pratt parsing for expressions.

use edge_ast::{
    AsmOp, BinOp, BlockItem, CodeBlock, Expr, FnDecl, Ident, Lit, LoopBlock, LoopItem, Program,
    Stmt, TypeSig, UnaryOp,
};
use edge_lexer::lexer::Lexer;
use edge_types::{
    span::Span,
    tokens::{ComparisonOperator, Keyword, Operator, Token, TokenKind},
};

use crate::errors::{ParseError, ParseResult};

/// The parser struct
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    /// Create a new parser from source code
    pub fn new(source: &str) -> ParseResult<Self> {
        let mut lexer = Lexer::new(source);
        // Heuristic: roughly one meaningful token per 5 source characters.
        let mut tokens = Vec::with_capacity(source.len() / 5 + 1);

        loop {
            let token = lexer
                .next_token()
                .map_err(|e| ParseError::LexerError(format!("{e:?}")))?;

            let is_eof = token.kind == TokenKind::Eof;
            // Drop whitespace and comments eagerly so the parser never has to
            // skip them, and so we store fewer tokens overall.
            if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Comment(_)) {
                tokens.push(token);
            }
            if is_eof {
                break;
            }
        }

        Ok(Self { tokens, cursor: 0 })
    }

    /// Parse the program
    pub fn parse(&mut self) -> ParseResult<Program> {
        let start = self.peek().span.clone();
        let mut stmts = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }

        let end = self.peek().span.clone();
        let span = Span {
            start: start.start,
            end: end.end,
            file: start.file,
        };

        Ok(Program { stmts, span })
    }

    // ============ Helper Methods ============

    /// Check if we're at end of file
    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    /// Look ahead past the current `(` to decide if this is an arrow-function parameter
    /// list. Returns true when `(` is followed by `ident, ident, ..., ) =>` or just `) =>`.
    fn is_arrow_function_params(&self) -> bool {
        let mut i = self.cursor; // cursor is already past the '('
                                 // Skip whitespace
        while i < self.tokens.len()
            && matches!(
                self.tokens[i].kind,
                TokenKind::Whitespace | TokenKind::Comment(_)
            )
        {
            i += 1;
        }
        // Empty parens: check for `) =>`
        if i < self.tokens.len() && self.tokens[i].kind == TokenKind::CloseParen {
            i += 1;
            // skip whitespace
            while i < self.tokens.len()
                && matches!(
                    self.tokens[i].kind,
                    TokenKind::Whitespace | TokenKind::Comment(_)
                )
            {
                i += 1;
            }
            return i < self.tokens.len() && self.tokens[i].kind == TokenKind::FatArrow;
        }
        // Collect tokens until CloseParen to check `ident (,ident)* ) =>`
        loop {
            // skip whitespace
            while i < self.tokens.len()
                && matches!(
                    self.tokens[i].kind,
                    TokenKind::Whitespace | TokenKind::Comment(_)
                )
            {
                i += 1;
            }
            if i >= self.tokens.len() {
                return false;
            }
            match &self.tokens[i].kind {
                TokenKind::Ident(_) => i += 1,
                _ => return false,
            }
            // skip whitespace
            while i < self.tokens.len()
                && matches!(
                    self.tokens[i].kind,
                    TokenKind::Whitespace | TokenKind::Comment(_)
                )
            {
                i += 1;
            }
            if i >= self.tokens.len() {
                return false;
            }
            match &self.tokens[i].kind {
                TokenKind::Comma => i += 1,
                TokenKind::CloseParen => {
                    i += 1;
                    // skip whitespace
                    while i < self.tokens.len()
                        && matches!(
                            self.tokens[i].kind,
                            TokenKind::Whitespace | TokenKind::Comment(_)
                        )
                    {
                        i += 1;
                    }
                    return i < self.tokens.len() && self.tokens[i].kind == TokenKind::FatArrow;
                }
                _ => return false,
            }
        }
    }

    /// Look ahead to check if the tokens after `{` form a struct instantiation
    /// pattern: `{ ident : expr, ... }`. Distinguishes from code blocks.
    fn is_struct_instantiation(&self) -> bool {
        // Look ahead past `{` for `ident :`
        let mut i = self.cursor + 1;
        // Skip whitespace/comments
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Whitespace | TokenKind::Comment(_) => i += 1,
                _ => break,
            }
        }
        if i >= self.tokens.len() {
            return false;
        }
        // Check for `ident`
        if !matches!(self.tokens[i].kind, TokenKind::Ident(_)) {
            return false;
        }
        i += 1;
        // Skip whitespace
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::Whitespace | TokenKind::Comment(_) => i += 1,
                _ => break,
            }
        }
        // Check for `:`
        i < self.tokens.len() && matches!(self.tokens[i].kind, TokenKind::Colon)
    }

    /// Peek at the current token
    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    /// Advance to the next token
    fn advance(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        if !self.is_at_end() {
            self.cursor += 1;
        }
        token
    }

    /// Check if current token matches a kind
    fn check(&self, kind: &TokenKind) -> bool {
        &self.peek().kind == kind
    }

    /// Skip whitespace and comment tokens
    fn skip_whitespace_and_comments(&mut self) {
        while matches!(
            self.peek().kind,
            TokenKind::Whitespace | TokenKind::Comment(_)
        ) {
            self.advance();
        }
    }

    /// Expect a specific token kind and advance
    fn expect(&mut self, kind: TokenKind) -> ParseResult<Token> {
        self.skip_whitespace_and_comments();
        let token = self.peek().clone();
        if token.kind == kind {
            Ok(self.advance())
        } else {
            Err(ParseError::unexpected(&token.kind, &kind, token.span))
        }
    }

    // ============ Statement Parsing ============

    /// Parse a statement
    pub fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        self.skip_whitespace_and_comments();

        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_var_decl(),
            TokenKind::Keyword(Keyword::Const) => self.parse_const_assign(),
            TokenKind::Keyword(Keyword::Type) => self.parse_type_assign(),
            TokenKind::Keyword(Keyword::Fn) => self.parse_fn_assign(),
            TokenKind::Keyword(Keyword::Pub) => self.parse_pub(),
            TokenKind::Keyword(Keyword::Event) => self.parse_event(),
            TokenKind::Keyword(Keyword::Abi) => self.parse_abi(),
            TokenKind::Keyword(Keyword::Module) => self.parse_module(),
            TokenKind::Keyword(Keyword::Use) => self.parse_use(),
            TokenKind::Keyword(Keyword::Contract) => self.parse_contract(),
            TokenKind::Keyword(Keyword::Trait) => self.parse_trait_stub(),
            TokenKind::OpenBrace => self.parse_code_block_stmt(),
            TokenKind::Keyword(Keyword::If) => self.parse_if_else(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match(),
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop(),
            TokenKind::Keyword(Keyword::For) => self.parse_for_loop(),
            TokenKind::Keyword(Keyword::While) => self.parse_while_loop(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return(),
            TokenKind::Keyword(Keyword::Break) => {
                let tok = self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Break(tok.span))
            }
            TokenKind::Keyword(Keyword::Continue) => {
                let tok = self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Continue(tok.span))
            }
            TokenKind::Keyword(Keyword::Do) => self.parse_do_while(),
            TokenKind::Keyword(Keyword::Comptime) => self.parse_comptime(),
            TokenKind::Keyword(Keyword::Impl) => self.parse_impl_block(),
            TokenKind::Keyword(Keyword::Emit) => self.parse_emit(),
            _ => self.parse_expr_stmt(),
        }
    }

    /// Parse variable declaration, with optional `mut` and initialization
    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Let))?;

        // Optional `mut` keyword
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::Keyword(Keyword::Mut)) {
            self.advance(); // consume `mut` (tracked in future AST changes)
        }

        let name = self.parse_ident()?;

        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_sig()?)
        } else {
            None
        };

        // Check for initialization: let x: T = expr;
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::Operator(Operator::Assignment)) {
            self.advance();
            let init_expr = self.parse_expr()?;
            self.expect(TokenKind::Semicolon)?;

            let end_span = self.tokens[self.cursor - 1].span.clone();
            let span = Span {
                start: start_tok.span.start,
                end: end_span.end,
                file: start_tok.span.file,
            };

            Ok(Stmt::VarDecl(name, ty, Some(Box::new(init_expr)), span))
        } else {
            self.expect(TokenKind::Semicolon)?;
            let span = Span {
                start: start_tok.span.start,
                end: self.tokens[self.cursor - 1].span.end,
                file: start_tok.span.file,
            };

            Ok(Stmt::VarDecl(name, ty, None, span))
        }
    }

    /// Parse constant assignment
    fn parse_const_assign(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Const))?;
        let name = self.parse_ident()?;

        // Optional type annotation: const X: u8 = ... or const X = ...
        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_sig()?)
        } else {
            None
        };

        self.expect(TokenKind::Operator(Operator::Assignment))?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ConstAssign(
            edge_ast::ConstDecl {
                name,
                ty,
                span: span.clone(),
            },
            expr,
            span,
        ))
    }

    /// Parse type assignment
    fn parse_type_assign(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Type))?;
        let name = self.parse_ident()?;

        // Optional type parameters: type Stack<T> = ...
        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::Operator(Operator::Assignment))?;

        // Try to parse union type: A | B | C(T)
        // A union starts with an identifier optionally followed by (T) and then |
        let ty = self.parse_type_sig_or_union()?;

        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::TypeAssign(
            edge_ast::TypeDecl {
                name,
                type_params,
                is_pub: false,
                span: span.clone(),
            },
            ty,
            span,
        ))
    }

    /// Parse function assignment
    fn parse_fn_assign(&mut self) -> ParseResult<Stmt> {
        let decl = self.parse_fn_decl()?;
        let block = self.parse_code_block()?;
        Ok(Stmt::FnAssign(decl, block))
    }

    /// Parse pub keyword and dispatch to appropriate parser
    fn parse_pub(&mut self) -> ParseResult<Stmt> {
        self.expect(TokenKind::Keyword(Keyword::Pub))?;
        self.skip_whitespace_and_comments();

        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Ext) => {
                self.advance();
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::Keyword(Keyword::Fn)) {
                    let mut decl = self.parse_fn_decl()?;
                    decl.is_pub = true;
                    decl.is_ext = true;
                    let block = self.parse_code_block()?;
                    Ok(Stmt::FnAssign(decl, block))
                } else {
                    Err(ParseError::InvalidExpr {
                        message: "Expected 'fn' after 'pub ext'".to_string(),
                        span: self.peek().span.clone(),
                    })
                }
            }
            TokenKind::Keyword(Keyword::Mut) => {
                self.advance();
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::Keyword(Keyword::Fn)) {
                    let mut decl = self.parse_fn_decl()?;
                    decl.is_pub = true;
                    decl.is_mut = true;
                    let block = self.parse_code_block()?;
                    Ok(Stmt::FnAssign(decl, block))
                } else {
                    Err(ParseError::InvalidExpr {
                        message: "Expected 'fn' after 'pub mut'".to_string(),
                        span: self.peek().span.clone(),
                    })
                }
            }
            TokenKind::Keyword(Keyword::Fn) => {
                let mut decl = self.parse_fn_decl()?;
                decl.is_pub = true;
                let block = self.parse_code_block()?;
                Ok(Stmt::FnAssign(decl, block))
            }
            TokenKind::Keyword(Keyword::Contract) => {
                // pub contract
                self.parse_contract_with_pub(true)
            }
            _ => Err(ParseError::InvalidExpr {
                message: "Expected 'fn' or 'contract' after 'pub'".to_string(),
                span: self.peek().span.clone(),
            }),
        }
    }

    /// Parse event declaration
    fn parse_event(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Event))?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::OpenParen)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseParen) {
                break;
            }

            let mut indexed = false;
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::Keyword(Keyword::Indexed)) {
                self.advance();
                indexed = true;
            }

            let field_name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let field_type = self.parse_type_sig()?;

            fields.push(edge_ast::EventField {
                name: field_name,
                indexed,
                ty: field_type,
            });

            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::CloseParen) {
                self.expect(TokenKind::Comma)?;
            }
        }

        let _end_tok = self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::EventDecl(edge_ast::EventDecl {
            name,
            is_anon: false,
            fields,
            span,
        }))
    }

    /// Parse ABI declaration
    fn parse_abi(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Abi))?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut functions = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            if self.check(&TokenKind::Keyword(Keyword::Fn)) {
                self.advance();
                let fn_name = self.parse_ident()?;
                self.expect(TokenKind::OpenParen)?;

                let mut params = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::CloseParen) {
                        break;
                    }
                    let param_name = self.parse_ident()?;
                    self.expect(TokenKind::Colon)?;
                    let param_type = self.parse_type_sig()?;
                    params.push((param_name, param_type));

                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;

                let mut returns = Vec::new();
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::Arrow) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::OpenParen) {
                        self.advance();
                        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                            self.skip_whitespace_and_comments();
                            if self.check(&TokenKind::CloseParen) {
                                break;
                            }
                            returns.push(self.parse_type_sig()?);
                            self.skip_whitespace_and_comments();
                            if !self.check(&TokenKind::CloseParen) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        self.expect(TokenKind::CloseParen)?;
                    } else {
                        returns.push(self.parse_type_sig()?);
                    }
                }

                self.skip_whitespace_and_comments();
                self.expect(TokenKind::Semicolon)?;

                functions.push(edge_ast::AbiFnDecl {
                    name: fn_name,
                    params,
                    returns,
                    is_mut: false,
                    span: Span {
                        start: start_tok.span.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start_tok.span.file.clone(),
                    },
                });
            } else {
                self.advance();
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;

        let span = Span {
            start: start_tok.span.start,
            end: end_tok.span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::AbiDecl(edge_ast::AbiDecl {
            name,
            superabis: Vec::new(),
            functions,
            span,
        }))
    }

    /// Parse module declaration
    fn parse_module(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Module))?;
        let name = self.parse_ident()?;
        self.skip_whitespace_and_comments();

        // `mod name;` → external module declaration (no body)
        // `mod name { items }` → inline module with body
        let items = if self.check(&TokenKind::OpenBrace) {
            self.advance(); // consume '{'
            let mut stmts = Vec::new();
            while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::CloseBrace) {
                    break;
                }
                stmts.push(self.parse_stmt()?);
            }
            self.expect(TokenKind::CloseBrace)?;
            stmts
        } else {
            self.expect(TokenKind::Semicolon)?;
            Vec::new()
        };

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ModuleDecl(edge_ast::ModuleDecl {
            name,
            is_pub: false,
            doc: None,
            items,
            span,
        }))
    }

    /// Parse use/import statement
    fn parse_use(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Use))?;
        let root = self.parse_ident()?;

        let (segments, path) = if self.check(&TokenKind::DoubleColon) {
            self.advance();
            self.skip_whitespace_and_comments();

            // Tree import: `use root::{a, b, c}` or `use root::*`
            if self.check(&TokenKind::OpenBrace) {
                self.advance(); // consume '{'
                let mut nested = Vec::new();
                while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::CloseBrace) {
                        break;
                    }
                    let seg = self.parse_ident()?;
                    nested.push(edge_ast::ImportPath::Ident(seg));
                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseBrace) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::CloseBrace)?;
                (Vec::new(), Some(edge_ast::ImportPath::Nested(nested)))
            } else if self.check(&TokenKind::Operator(
                edge_types::tokens::Operator::Arithmetic(
                    edge_types::tokens::ArithmeticOperator::Mul,
                ),
            )) {
                // Glob import: `use root::*`
                self.advance();
                (Vec::new(), Some(edge_ast::ImportPath::All))
            } else {
                // Simple or deep path: `use a::b::c` or `use a::b::c::{d, e}`
                let mut path_segments = vec![self.parse_ident()?];
                while self.check(&TokenKind::DoubleColon) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::OpenBrace) {
                        // `use a::b::{c, d}` — path_segments are intermediate segments
                        self.advance();
                        let mut nested = Vec::new();
                        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                            self.skip_whitespace_and_comments();
                            if self.check(&TokenKind::CloseBrace) {
                                break;
                            }
                            nested.push(edge_ast::ImportPath::Ident(self.parse_ident()?));
                            self.skip_whitespace_and_comments();
                            if !self.check(&TokenKind::CloseBrace) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        self.expect(TokenKind::CloseBrace)?;
                        self.expect(TokenKind::Semicolon)?;
                        let span = Span {
                            start: start_tok.span.start,
                            end: self.tokens[self.cursor - 1].span.end,
                            file: start_tok.span.file,
                        };
                        return Ok(Stmt::ModuleImport(edge_ast::ModuleImport {
                            root,
                            segments: path_segments,
                            path: Some(edge_ast::ImportPath::Nested(nested)),
                            is_pub: false,
                            span,
                        }));
                    }
                    if self.check(&TokenKind::Operator(
                        edge_types::tokens::Operator::Arithmetic(
                            edge_types::tokens::ArithmeticOperator::Mul,
                        ),
                    )) {
                        // `use a::b::*` — glob after intermediate segments
                        self.advance();
                        self.expect(TokenKind::Semicolon)?;
                        let span = Span {
                            start: start_tok.span.start,
                            end: self.tokens[self.cursor - 1].span.end,
                            file: start_tok.span.file,
                        };
                        return Ok(Stmt::ModuleImport(edge_ast::ModuleImport {
                            root,
                            segments: path_segments,
                            path: Some(edge_ast::ImportPath::All),
                            is_pub: false,
                            span,
                        }));
                    }
                    path_segments.push(self.parse_ident()?);
                }
                // Split: all but last are intermediate segments, last is the import target
                let last = path_segments.pop().map(edge_ast::ImportPath::Ident);
                (path_segments, last)
            }
        } else {
            (Vec::new(), None)
        };

        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ModuleImport(edge_ast::ModuleImport {
            root,
            segments,
            path,
            is_pub: false,
            span,
        }))
    }

    /// Parse trait declaration
    fn parse_trait_stub(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Trait))?;
        let name = self.parse_ident()?;

        // Optional type parameters: trait Comparable<T> { ... }
        let type_params = self.parse_optional_type_params()?;

        // Optional supertraits: Trait: SuperA & SuperB
        let mut supertraits = Vec::new();
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::Colon) {
            self.advance();
            supertraits.push(self.parse_ident()?);
            while self.check(&TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
                edge_types::tokens::BitwiseOperator::And,
            ))) {
                self.advance();
                supertraits.push(self.parse_ident()?);
            }
        }

        self.expect(TokenKind::OpenBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }
            items.push(self.parse_trait_item()?);
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        let span = Span {
            start: start_tok.span.start,
            end: end_tok.span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::TraitDecl(
            edge_ast::TraitDecl {
                name,
                type_params,
                supertraits,
                items,
                is_pub: false,
                span: span.clone(),
            },
            span,
        ))
    }

    /// Parse trait item
    fn parse_trait_item(&mut self) -> ParseResult<edge_ast::item::TraitItem> {
        self.skip_whitespace_and_comments();
        let is_pub = if self.check(&TokenKind::Keyword(Keyword::Pub)) {
            self.advance();
            self.skip_whitespace_and_comments();
            true
        } else {
            false
        };
        let _ = is_pub; // may be used later

        match self.peek().kind.clone() {
            TokenKind::Keyword(Keyword::Fn) => {
                let decl = self.parse_fn_decl()?;
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::OpenBrace) {
                    let block = self.parse_code_block()?;
                    Ok(edge_ast::item::TraitItem::FnAssign(decl, block))
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(edge_ast::item::TraitItem::FnDecl(decl))
                }
            }
            TokenKind::Keyword(Keyword::Const) => {
                self.advance();
                let const_name = self.parse_ident()?;
                let span = const_name.span.clone();
                self.expect(TokenKind::Colon)?;
                let ty = self.parse_type_sig()?;
                let const_decl = edge_ast::ConstDecl {
                    name: const_name,
                    ty: Some(ty),
                    span,
                };
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::Operator(Operator::Assignment)) {
                    self.advance();
                    let expr = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(edge_ast::item::TraitItem::ConstAssign(const_decl, expr))
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(edge_ast::item::TraitItem::ConstDecl(const_decl))
                }
            }
            TokenKind::Keyword(Keyword::Type) => {
                self.advance();
                let type_name = self.parse_ident()?;
                let span = type_name.span.clone();
                let type_decl = edge_ast::TypeDecl {
                    name: type_name,
                    type_params: Vec::new(),
                    is_pub: false,
                    span,
                };
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::Operator(Operator::Assignment)) {
                    self.advance();
                    let ty = self.parse_type_sig()?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(edge_ast::item::TraitItem::TypeAssign(type_decl, ty))
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(edge_ast::item::TraitItem::TypeDecl(type_decl))
                }
            }
            _ => Err(ParseError::InvalidExpr {
                message: format!("Unexpected token in trait body: {:?}", self.peek().kind),
                span: self.peek().span.clone(),
            }),
        }
    }

    /// Parse contract declaration
    fn parse_contract(&mut self) -> ParseResult<Stmt> {
        self.parse_contract_with_pub(false)
    }

    /// Parse contract declaration (with optional pub flag)
    fn parse_contract_with_pub(&mut self, _is_pub: bool) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Contract))?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut fields = Vec::new();
        let mut consts = Vec::new();
        let mut functions = Vec::new();

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            if self.check(&TokenKind::Keyword(Keyword::Let)) {
                self.advance();
                let field_name = self.parse_ident()?;
                self.expect(TokenKind::Colon)?;
                let field_type = self.parse_type_sig()?;
                self.expect(TokenKind::Semicolon)?;
                fields.push((field_name, field_type));
            } else if self.check(&TokenKind::Keyword(Keyword::Const)) {
                self.advance();
                let const_name = self.parse_ident()?;
                let const_span = const_name.span.clone();
                self.expect(TokenKind::Colon)?;
                let const_type = self.parse_type_sig()?;
                self.expect(TokenKind::Operator(Operator::Assignment))?;
                let expr = self.parse_expr()?;
                self.expect(TokenKind::Semicolon)?;
                let const_decl = edge_ast::ConstDecl {
                    name: const_name,
                    ty: Some(const_type),
                    span: const_span,
                };
                consts.push((const_decl, expr));
            } else if self.check(&TokenKind::Keyword(Keyword::Fn))
                || self.check(&TokenKind::Keyword(Keyword::Pub))
                || self.check(&TokenKind::Keyword(Keyword::Ext))
            {
                // Parse the function declaration
                let is_pub = if self.check(&TokenKind::Keyword(Keyword::Pub)) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    true
                } else {
                    false
                };
                let is_ext = if self.check(&TokenKind::Keyword(Keyword::Ext)) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    true
                } else {
                    false
                };
                let is_mut = if self.check(&TokenKind::Keyword(Keyword::Mut)) {
                    self.advance();
                    self.skip_whitespace_and_comments();
                    true
                } else {
                    false
                };
                let fn_decl = self.parse_fn_decl()?;
                let body = self.parse_code_block()?;
                functions.push(edge_ast::ContractFnDecl {
                    name: fn_decl.name,
                    params: fn_decl.params,
                    returns: fn_decl.returns,
                    is_pub,
                    is_ext,
                    is_mut,
                    body: Some(body),
                    span: fn_decl.span,
                });
            } else {
                self.advance();
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;

        let span = Span {
            start: start_tok.span.start,
            end: end_tok.span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ContractDecl(edge_ast::ContractDecl {
            name,
            fields,
            consts,
            functions,
            span,
        }))
    }

    /// Parse expression statement
    fn parse_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Expr(expr))
    }

    /// Parse if-else statement
    fn parse_if_else(&mut self) -> ParseResult<Stmt> {
        let _start = self.expect(TokenKind::Keyword(Keyword::If))?;
        let cond = self.parse_if_condition()?;
        let block = self.parse_code_block()?;

        let mut conditions = vec![(cond, block)];
        let mut else_block = None;

        // Handle else if and else chains
        loop {
            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::Keyword(Keyword::Else)) {
                break;
            }
            self.advance(); // consume 'else'
            self.skip_whitespace_and_comments();

            self.skip_whitespace_and_comments();

            if self.check(&TokenKind::Keyword(Keyword::If)) {
                // else if
                self.advance(); // consume 'if'
                let elif_cond = self.parse_if_condition()?;
                let elif_block = self.parse_code_block()?;
                conditions.push((elif_cond, elif_block));
            } else {
                // else
                else_block = Some(self.parse_code_block()?);
                break;
            }
        }

        Ok(Stmt::IfElse(conditions, else_block))
    }

    /// Parse an if condition: either `(expr)` or `expr matches Pattern`
    fn parse_if_condition(&mut self) -> ParseResult<Expr> {
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::OpenParen) {
            self.advance();
            let cond = self.parse_expr()?;
            self.expect(TokenKind::CloseParen)?;
            Ok(cond)
        } else {
            // `if expr matches Pattern { ... }`
            let expr = self.parse_expr()?;
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::Keyword(Keyword::Matches)) {
                self.advance();
                self.skip_whitespace_and_comments();
                let pattern = self.parse_union_pattern()?;
                let span = Span {
                    start: expr.span().start,
                    end: self.tokens[self.cursor - 1].span.end,
                    file: expr.span().file,
                };
                Ok(Expr::PatternMatch(Box::new(expr), pattern, span))
            } else {
                Ok(expr)
            }
        }
    }

    /// Parse a union pattern: `Type::Variant` or `Type::Variant(binding)`
    fn parse_union_pattern(&mut self) -> ParseResult<edge_ast::pattern::UnionPattern> {
        let union_name = self.parse_ident()?;
        self.expect(TokenKind::DoubleColon)?;
        let member_name = self.parse_ident()?;

        let mut bindings = Vec::new();
        if self.check(&TokenKind::OpenParen) {
            self.advance();
            while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::CloseParen) {
                    break;
                }
                bindings.push(self.parse_ident()?);
                self.skip_whitespace_and_comments();
                if !self.check(&TokenKind::CloseParen) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::CloseParen)?;
        }

        let span = Span {
            start: union_name.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: union_name.span.file.clone(),
        };

        Ok(edge_ast::pattern::UnionPattern {
            union_name,
            member_name,
            bindings,
            span,
        })
    }

    /// Parse match statement
    fn parse_match(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Match))?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::OpenBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }
            let pattern = self.parse_match_pattern()?;
            self.expect(TokenKind::FatArrow)?;
            self.skip_whitespace_and_comments();

            // Match arm body: either a code block { ... } or a single statement/expression
            let body = if self.check(&TokenKind::OpenBrace) {
                self.parse_code_block()?
            } else {
                // Single statement/expression arm without braces.
                // Handle `return expr` and regular expressions.
                self.skip_whitespace_and_comments();
                let stmt = if self.check(&TokenKind::Keyword(Keyword::Return)) {
                    // Parse return statement without requiring `;`
                    let start_tok = self.advance();
                    self.skip_whitespace_and_comments();
                    // Check if the return has a value (not immediately followed by , or })
                    let expr =
                        if self.check(&TokenKind::Comma) || self.check(&TokenKind::CloseBrace) {
                            None
                        } else {
                            Some(self.parse_expr()?)
                        };
                    let span = Span {
                        start: start_tok.span.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start_tok.span.file,
                    };
                    Stmt::Return(expr, span)
                } else {
                    let expr = self.parse_expr()?;
                    Stmt::Expr(expr)
                };
                let span = match &stmt {
                    Stmt::Return(_, s) => s.clone(),
                    Stmt::Expr(e) => e.span(),
                    _ => Span::EOF,
                };
                CodeBlock {
                    stmts: vec![BlockItem::Stmt(Box::new(stmt))],
                    span,
                }
            };

            arms.push(edge_ast::pattern::MatchArm { pattern, body });
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        let span = Span {
            start: start_tok.span.start,
            end: end_tok.span.end,
            file: start_tok.span.file,
        };
        Ok(Stmt::Match(expr, arms, span))
    }

    /// Parse match pattern
    fn parse_match_pattern(&mut self) -> ParseResult<edge_ast::pattern::MatchPattern> {
        self.skip_whitespace_and_comments();
        if let TokenKind::Ident(ref name) = self.peek().kind.clone() {
            if name == "_" {
                self.advance();
                return Ok(edge_ast::pattern::MatchPattern::Wildcard);
            }
        }
        let ident = self.parse_ident()?;
        if self.check(&TokenKind::DoubleColon) {
            self.advance();
            let member = self.parse_ident()?;
            let mut bindings = Vec::new();
            if self.check(&TokenKind::OpenParen) {
                self.advance();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::CloseParen) {
                        break;
                    }
                    bindings.push(self.parse_ident()?);
                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
            }
            let span = Span {
                start: ident.span.start,
                end: self.tokens[self.cursor - 1].span.end,
                file: ident.span.file.clone(),
            };
            Ok(edge_ast::pattern::MatchPattern::Union(
                edge_ast::pattern::UnionPattern {
                    union_name: ident,
                    member_name: member,
                    bindings,
                    span,
                },
            ))
        } else {
            Ok(edge_ast::pattern::MatchPattern::Ident(ident))
        }
    }

    /// Parse loop statement
    fn parse_loop(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Loop))?;
        let items = self.parse_loop_block_items()?;

        Ok(Stmt::Loop(LoopBlock {
            items,
            span: start_tok.span,
        }))
    }

    /// Parse for loop
    fn parse_for_loop(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::For))?;
        self.expect(TokenKind::OpenParen)?;
        self.skip_whitespace_and_comments();

        // Parse init: may be empty (;), a statement (let x = 0;), or an expr stmt
        let init = if self.check(&TokenKind::Semicolon) {
            self.advance(); // consume the ;
            None
        } else {
            // parse_stmt already consumes the trailing ;
            Some(Box::new(self.parse_stmt()?))
        };

        self.skip_whitespace_and_comments();

        // Parse condition: may be empty (;) or an expression
        let cond = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(TokenKind::Semicolon)?;

        self.skip_whitespace_and_comments();

        // Parse update: may be empty ()) or an expression (like `i = i + 1`)
        let update = if self.check(&TokenKind::CloseParen) {
            None
        } else {
            // The update clause is an expression (no trailing ;)
            let expr = self.parse_expr()?;
            Some(Box::new(Stmt::Expr(expr)))
        };

        self.expect(TokenKind::CloseParen)?;
        let items = self.parse_loop_block_items()?;

        Ok(Stmt::ForLoop(
            init,
            cond,
            update,
            LoopBlock {
                items,
                span: start_tok.span,
            },
        ))
    }

    /// Parse while loop
    fn parse_while_loop(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::While))?;
        self.expect(TokenKind::OpenParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::CloseParen)?;
        let items = self.parse_loop_block_items()?;

        Ok(Stmt::WhileLoop(
            cond,
            LoopBlock {
                items,
                span: start_tok.span,
            },
        ))
    }

    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Return))?;

        if self.check(&TokenKind::Semicolon) {
            self.advance();
            let end_span = self.tokens[self.cursor - 1].span.clone();
            let span = Span {
                start: start_tok.span.start,
                end: end_span.end,
                file: start_tok.span.file,
            };
            return Ok(Stmt::Return(None, span));
        }

        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        let end_span = self.tokens[self.cursor - 1].span.clone();
        let span = Span {
            start: start_tok.span.start,
            end: end_span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::Return(Some(expr), span))
    }

    /// Parse emit statement: emit EventName(args...)
    fn parse_emit(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Emit))?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::OpenParen)?;

        let mut args = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseParen) {
                break;
            }
            args.push(self.parse_expr()?);
            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::CloseParen) {
                self.expect(TokenKind::Comma)?;
            }
        }

        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::Emit(name, args, span))
    }

    /// Parse do-while statement
    fn parse_do_while(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Do))?;
        let items = self.parse_loop_block_items()?;
        self.expect(TokenKind::Keyword(Keyword::While))?;
        self.expect(TokenKind::OpenParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::CloseParen)?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::DoWhile(
            LoopBlock {
                items,
                span: start_tok.span,
            },
            cond,
        ))
    }

    /// Parse comptime statement
    fn parse_comptime(&mut self) -> ParseResult<Stmt> {
        self.expect(TokenKind::Keyword(Keyword::Comptime))?;
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::Keyword(Keyword::Fn)) {
            let decl = self.parse_fn_decl()?;
            let block = self.parse_code_block()?;
            Ok(Stmt::ComptimeFn(decl, block))
        } else {
            let stmt = self.parse_stmt()?;
            Ok(Stmt::ComptimeBranch(Box::new(stmt)))
        }
    }

    /// Parse impl block
    fn parse_impl_block(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Impl))?;
        let ty_name = self.parse_ident()?;

        // Optional type arguments/parameters: impl Stack<T> or impl Comparable<u256>
        // Parse as type sigs and convert to TypeParams for the AST
        let type_params = self.parse_optional_type_params_or_args()?;

        // Check for trait impl: impl Type : TraitName { ... }
        self.skip_whitespace_and_comments();
        let trait_impl = if self.check(&TokenKind::Colon) {
            self.advance();
            let trait_name = self.parse_ident()?;
            let trait_params = self.parse_optional_type_params_or_args()?;
            Some((trait_name, trait_params))
        } else {
            None
        };

        self.expect(TokenKind::OpenBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            let is_pub = if self.check(&TokenKind::Keyword(Keyword::Pub)) {
                self.advance();
                self.skip_whitespace_and_comments();
                true
            } else {
                false
            };

            match self.peek().kind.clone() {
                TokenKind::Keyword(Keyword::Fn) => {
                    let mut decl = self.parse_fn_decl()?;
                    decl.is_pub = is_pub;
                    let block = self.parse_code_block()?;
                    items.push(edge_ast::item::ImplItem::FnAssign(decl, block));
                }
                TokenKind::Keyword(Keyword::Const) => {
                    self.advance();
                    let const_name = self.parse_ident()?;
                    let const_span = const_name.span.clone();
                    self.expect(TokenKind::Colon)?;
                    let ty = self.parse_type_sig()?;
                    self.expect(TokenKind::Operator(Operator::Assignment))?;
                    let expr = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;
                    let const_decl = edge_ast::ConstDecl {
                        name: const_name,
                        ty: Some(ty),
                        span: const_span,
                    };
                    items.push(edge_ast::item::ImplItem::ConstAssign(const_decl, expr));
                }
                TokenKind::Keyword(Keyword::Type) => {
                    self.advance();
                    let type_name = self.parse_ident()?;
                    let type_decl = edge_ast::TypeDecl {
                        name: type_name.clone(),
                        type_params: Vec::new(),
                        is_pub,
                        span: type_name.span.clone(),
                    };
                    self.expect(TokenKind::Operator(Operator::Assignment))?;
                    let ty = self.parse_type_sig()?;
                    self.expect(TokenKind::Semicolon)?;
                    items.push(edge_ast::item::ImplItem::TypeAssign(type_decl, ty));
                }
                _ => {
                    self.advance();
                }
            }
        }

        let end_tok = self.expect(TokenKind::CloseBrace)?;
        let span = Span {
            start: start_tok.span.start,
            end: end_tok.span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ImplBlock(edge_ast::item::ImplBlock {
            ty_name,
            type_params,
            trait_impl,
            items,
            span,
        }))
    }

    /// Parse code block as statement
    fn parse_code_block_stmt(&mut self) -> ParseResult<Stmt> {
        let block = self.parse_code_block()?;
        Ok(Stmt::CodeBlock(block))
    }

    // ============ Expression Parsing ============

    /// Parse an expression
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        let cond = self.parse_binary_expr(0)?;
        self.skip_whitespace_and_comments();

        // Ternary: cond ? then : else
        if self.check(&TokenKind::Question) {
            self.advance(); // consume '?'
            let then_expr = self.parse_expr()?;
            self.skip_whitespace_and_comments();
            self.expect(TokenKind::Colon)?;
            let else_expr = self.parse_expr()?;
            let span = Span {
                start: cond.span().start,
                end: else_expr.span().end,
                file: cond.span().file,
            };
            return Ok(Expr::Ternary(
                Box::new(cond),
                Box::new(then_expr),
                Box::new(else_expr),
                span,
            ));
        }

        Ok(cond)
    }

    /// Parse binary expression with precedence climbing
    fn parse_binary_expr(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary_expr()?;

        loop {
            self.skip_whitespace_and_comments();

            if self.is_at_end() {
                break;
            }

            let (prec, is_right_assoc) = match self.get_operator_precedence() {
                Some(p) => p,
                None => break,
            };

            if prec < min_prec {
                break;
            }

            // Check for assignment specially
            if self.check(&TokenKind::Operator(
                edge_types::tokens::Operator::Assignment,
            )) {
                let _op_start = self.advance().span.clone();
                let right = self.parse_binary_expr(prec + 1)?;

                let span = Span {
                    start: left.span().start,
                    end: right.span().end,
                    file: left.span().file.clone(),
                };

                left = Expr::Assign(Box::new(left), Box::new(right), span);
            } else {
                let op = self.parse_bin_op()?;
                let next_min_prec = if is_right_assoc { prec } else { prec + 1 };

                let right = self.parse_binary_expr(next_min_prec)?;

                let span = Span {
                    start: left.span().start,
                    end: right.span().end,
                    file: left.span().file.clone(),
                };

                left = Expr::Binary(Box::new(left), op, Box::new(right), span);
            }
        }

        Ok(left)
    }

    /// Parse unary expression
    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        self.skip_whitespace_and_comments();

        if let Some(unary_op) = self.try_parse_unary_op() {
            let start = self.advance().span;
            let expr = self.parse_unary_expr()?;
            let span = Span {
                start: start.start,
                end: expr.span().end,
                file: start.file,
            };
            return Ok(Expr::Unary(unary_op, Box::new(expr), span));
        }

        self.parse_postfix_expr()
    }

    /// Parse postfix expression
    fn parse_postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            self.skip_whitespace_and_comments();

            match &self.peek().kind {
                TokenKind::OpenParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                        args.push(self.parse_expr()?);
                        if !self.check(&TokenKind::CloseParen) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    let end_tok = self.expect(TokenKind::CloseParen)?;

                    let span = Span {
                        start: expr.span().start,
                        end: end_tok.span.end,
                        file: expr.span().file.clone(),
                    };

                    expr = Expr::FunctionCall(Box::new(expr), args, vec![], span);
                }
                TokenKind::OpenBracket => {
                    self.advance();
                    let index = self.parse_expr()?;

                    // Check for range syntax [start:end]
                    let end_expr = if self.check(&TokenKind::Colon) {
                        self.advance();
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    };

                    let end_tok = self.expect(TokenKind::CloseBracket)?;

                    let span = Span {
                        start: expr.span().start,
                        end: end_tok.span.end,
                        file: expr.span().file.clone(),
                    };

                    expr = Expr::ArrayIndex(Box::new(expr), Box::new(index), end_expr, span);
                }
                TokenKind::Dot => {
                    self.advance();
                    match &self.peek().kind {
                        TokenKind::Ident(field_name) => {
                            let field = Ident {
                                name: field_name.clone(),
                                span: self.peek().span.clone(),
                            };
                            self.advance();

                            let span = Span {
                                start: expr.span().start,
                                end: self.tokens[self.cursor - 1].span.end,
                                file: expr.span().file.clone(),
                            };

                            expr = Expr::FieldAccess(Box::new(expr), field, span);
                        }
                        TokenKind::Literal(lit_bytes) => {
                            // Tuple field access: expr.0, expr.1, etc.
                            let mut value: u64 = 0;
                            for byte in lit_bytes {
                                value = (value << 8) | (*byte as u64);
                            }
                            let token = self.advance();
                            let span = Span {
                                start: expr.span().start,
                                end: token.span.end,
                                file: expr.span().file.clone(),
                            };
                            expr = Expr::TupleFieldAccess(Box::new(expr), value, span);
                        }
                        _ => {}
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expression
    fn parse_primary_expr(&mut self) -> ParseResult<Expr> {
        self.skip_whitespace_and_comments();

        let kind = self.peek().kind.clone();
        match kind {
            TokenKind::Literal(lit_bytes) => {
                let token = self.advance();
                // Extract the actual integer value from the literal bytes
                let mut value: u128 = 0;
                for byte in &lit_bytes {
                    value = (value << 8) | (*byte as u128);
                }
                let lit = Lit::Int(value as u64, None, token.span);
                Ok(Expr::Literal(Box::new(lit)))
            }
            TokenKind::StringLiteral(s) => {
                let token = self.advance();
                let lit = Lit::Str(s, token.span);
                Ok(Expr::Literal(Box::new(lit)))
            }
            TokenKind::Ident(name) => {
                let token = self.advance();
                let ident = Ident {
                    name,
                    span: token.span.clone(),
                };

                // Check for :: path expressions
                if self.check(&TokenKind::DoubleColon) {
                    let mut path_segments = vec![ident];
                    let mut turbofish_type_args: Vec<TypeSig> = vec![];
                    while self.check(&TokenKind::DoubleColon) {
                        self.advance();
                        // Check for turbofish: ::<Type, Type>
                        if self.check(&TokenKind::Operator(Operator::Comparison(
                            ComparisonOperator::LessThan,
                        ))) {
                            turbofish_type_args = self.parse_turbofish_type_args()?;
                            break;
                        }
                        if let TokenKind::Ident(next_name) = self.peek().kind.clone() {
                            let next_token = self.advance();
                            path_segments.push(Ident {
                                name: next_name,
                                span: next_token.span,
                            });
                        } else {
                            return Err(ParseError::InvalidExpr {
                                message: "Expected identifier after ::".to_string(),
                                span: self.peek().span.clone(),
                            });
                        }
                    }

                    // Check for function call with parens: Path::Variant(args) or func::<T>(args)
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::OpenParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                            self.skip_whitespace_and_comments();
                            if self.check(&TokenKind::CloseParen) {
                                break;
                            }
                            args.push(self.parse_expr()?);
                            self.skip_whitespace_and_comments();
                            if !self.check(&TokenKind::CloseParen) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        let end = self.expect(TokenKind::CloseParen)?;
                        let span = Span {
                            start: token.span.start,
                            end: end.span.end,
                            file: token.span.file,
                        };
                        Ok(Expr::FunctionCall(
                            Box::new(Expr::Path(path_segments, span.clone())),
                            args,
                            turbofish_type_args,
                            span,
                        ))
                    } else {
                        let end_span = path_segments.last().unwrap().span.clone();
                        let span = Span {
                            start: token.span.start,
                            end: end_span.end,
                            file: token.span.file,
                        };
                        Ok(Expr::Path(path_segments, span))
                    }
                } else {
                    self.skip_whitespace_and_comments();
                    // Arrow function: single-param form  `ident => { body }`
                    if self.check(&TokenKind::FatArrow) {
                        self.advance(); // consume '=>'
                        let body = self.parse_code_block()?;
                        let span = Span {
                            start: token.span.start,
                            end: body.span.end,
                            file: token.span.file,
                        };
                        return Ok(Expr::ArrowFunction(vec![ident], Box::new(body), span));
                    }

                    // Check for struct instantiation: Name { field: expr, ... }
                    if self.check(&TokenKind::OpenBrace) && self.is_struct_instantiation() {
                        self.advance(); // consume {
                        let mut fields = Vec::new();
                        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
                            self.skip_whitespace_and_comments();
                            if self.check(&TokenKind::CloseBrace) {
                                break;
                            }
                            let field_name = self.parse_ident()?;
                            self.expect(TokenKind::Colon)?;
                            let field_expr = self.parse_expr()?;
                            fields.push((field_name, field_expr));
                            self.skip_whitespace_and_comments();
                            if !self.check(&TokenKind::CloseBrace) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        let end = self.expect(TokenKind::CloseBrace)?;
                        let span = Span {
                            start: token.span.start,
                            end: end.span.end,
                            file: token.span.file,
                        };
                        Ok(Expr::StructInstantiation(None, ident, fields, span))
                    } else {
                        Ok(Expr::Ident(ident))
                    }
                }
            }
            TokenKind::At => {
                let start = self.advance().span;
                if let TokenKind::Ident(builtin_name) = self.peek().kind.clone() {
                    let name_token = self.advance();
                    let builtin_ident = Ident {
                        name: builtin_name,
                        span: name_token.span,
                    };

                    // Parse arguments if there are parentheses
                    let args = if self.check(&TokenKind::OpenParen) {
                        self.advance();
                        let mut args = Vec::new();
                        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                            args.push(self.parse_expr()?);
                            if !self.check(&TokenKind::CloseParen) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        let _end = self.expect(TokenKind::CloseParen)?;
                        args
                    } else {
                        Vec::new()
                    };

                    let span = Span {
                        start: start.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start.file,
                    };

                    Ok(Expr::At(builtin_ident, args, span))
                } else {
                    Err(ParseError::InvalidExpr {
                        message: "Expected identifier after @".to_string(),
                        span: self.peek().span.clone(),
                    })
                }
            }
            TokenKind::OpenParen => {
                let start = self.advance().span;

                // Multi-param arrow function: () => body  or  (x, y) => body
                // Detect by lookahead: () => or (ident, ident, ...) =>
                if self.is_arrow_function_params() {
                    // Parse parameter list of identifiers
                    let mut params = Vec::new();
                    while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                        self.skip_whitespace_and_comments();
                        if self.check(&TokenKind::CloseParen) {
                            break;
                        }
                        params.push(self.parse_ident()?);
                        self.skip_whitespace_and_comments();
                        if !self.check(&TokenKind::CloseParen) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect(TokenKind::CloseParen)?;
                    self.skip_whitespace_and_comments();
                    self.expect(TokenKind::FatArrow)?;
                    let body = self.parse_code_block()?;
                    let span = Span {
                        start: start.start,
                        end: body.span.end,
                        file: start.file,
                    };
                    return Ok(Expr::ArrowFunction(params, Box::new(body), span));
                }

                // Check for empty parens → empty tuple
                if self.check(&TokenKind::CloseParen) {
                    self.advance();
                    let span = Span {
                        start: start.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start.file,
                    };
                    return Ok(Expr::TupleInstantiation(None, Vec::new(), span));
                }

                let first = self.parse_expr()?;

                // Check if this is a tuple (has comma) or just parenthesized expr
                if self.check(&TokenKind::Comma) {
                    // Tuple instantiation: (expr, expr, ...)
                    let mut elems = vec![first];
                    while self.check(&TokenKind::Comma) {
                        self.advance();
                        self.skip_whitespace_and_comments();
                        if self.check(&TokenKind::CloseParen) {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                    }
                    self.expect(TokenKind::CloseParen)?;
                    let span = Span {
                        start: start.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start.file,
                    };
                    Ok(Expr::TupleInstantiation(None, elems, span))
                } else {
                    self.expect(TokenKind::CloseParen)?;
                    self.skip_whitespace_and_comments();
                    // Single-param arrow: (x) => body
                    if self.check(&TokenKind::FatArrow) {
                        if let Expr::Ident(param_ident) = first {
                            self.advance(); // consume '=>'
                            let body = self.parse_code_block()?;
                            let span = Span {
                                start: start.start,
                                end: body.span.end,
                                file: start.file,
                            };
                            return Ok(Expr::ArrowFunction(
                                vec![param_ident],
                                Box::new(body),
                                span,
                            ));
                        }
                    }
                    let span = Span {
                        start: start.start,
                        end: self.tokens[self.cursor - 1].span.end,
                        file: start.file,
                    };
                    Ok(Expr::Paren(Box::new(first), span))
                }
            }
            // Array instantiation: [expr, expr, ...]
            TokenKind::OpenBracket => {
                let start = self.advance().span;
                let mut elems = Vec::new();
                while !self.check(&TokenKind::CloseBracket) && !self.is_at_end() {
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::CloseBracket) {
                        break;
                    }
                    elems.push(self.parse_expr()?);
                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseBracket) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                let end_tok = self.expect(TokenKind::CloseBracket)?;
                let span = Span {
                    start: start.start,
                    end: end_tok.span.end,
                    file: start.file,
                };
                Ok(Expr::ArrayInstantiation(None, elems, span))
            }
            TokenKind::Keyword(Keyword::Asm) => self.parse_inline_asm(),
            _ => {
                let token = self.peek().clone();
                Err(ParseError::InvalidExpr {
                    message: format!("Unexpected token: {:?}", token.kind),
                    span: token.span,
                })
            }
        }
    }

    /// Parse an inline assembly block per spec:
    /// `asm(expr, expr, ...) -> (name, name, ...) { opcode opcode ... }`
    ///
    /// Inputs are expressions pushed onto the stack (left = TOS).
    /// Outputs are names bound to stack values after the block (left = TOS).
    /// Use `_` to discard an output.
    fn parse_inline_asm(&mut self) -> ParseResult<Expr> {
        let start = self.expect(TokenKind::Keyword(Keyword::Asm))?.span;

        // Parse inputs: (expr, expr, ...)
        self.expect(TokenKind::OpenParen)?;
        let mut inputs = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseParen) {
                break;
            }
            inputs.push(self.parse_expr()?);
            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::CloseParen) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::CloseParen)?;

        // Parse outputs: -> (name, name, ...)
        self.skip_whitespace_and_comments();
        let mut outputs: Vec<Option<Ident>> = Vec::new();
        if self.check(&TokenKind::Arrow) {
            self.advance(); // consume ->
            self.expect(TokenKind::OpenParen)?;
            while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                self.skip_whitespace_and_comments();
                if self.check(&TokenKind::CloseParen) {
                    break;
                }
                let tok = self.peek().clone();
                match &tok.kind {
                    TokenKind::Ident(name) if name == "_" => {
                        self.advance();
                        outputs.push(None); // discarded output
                    }
                    TokenKind::Ident(_) => {
                        let ident = self.parse_ident()?;
                        outputs.push(Some(ident));
                    }
                    _ => {
                        return Err(ParseError::InvalidExpr {
                            message: "expected identifier or '_' in asm output list".to_string(),
                            span: tok.span,
                        });
                    }
                }
                self.skip_whitespace_and_comments();
                if !self.check(&TokenKind::CloseParen) {
                    self.expect(TokenKind::Comma)?;
                }
            }
            self.expect(TokenKind::CloseParen)?;
        }

        // Parse body: { opcode opcode ... }
        self.skip_whitespace_and_comments();
        self.expect(TokenKind::OpenBrace)?;

        let mut ops = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) || self.is_at_end() {
                break;
            }

            let tok = self.peek().clone();
            match &tok.kind {
                TokenKind::Ident(name) => {
                    self.advance();
                    // Try as opcode first (case-insensitive), otherwise treat as ident
                    let upper = name.to_uppercase();
                    if is_evm_opcode(&upper) {
                        ops.push(AsmOp::Opcode(upper, tok.span));
                    } else {
                        ops.push(AsmOp::Ident(name.clone(), tok.span));
                    }
                }
                TokenKind::Literal(_) => {
                    self.advance();
                    let hex = match &tok.kind {
                        TokenKind::Literal(bytes) => {
                            let mut val: u128 = 0;
                            for b in bytes {
                                val = (val << 8) | (*b as u128);
                            }
                            format!("{val:#x}")
                        }
                        _ => unreachable!(),
                    };
                    ops.push(AsmOp::Literal(hex, tok.span));
                }
                _ => {
                    return Err(ParseError::InvalidExpr {
                        message: format!(
                            "unexpected token in asm block: {:?}, expected opcode, literal, or identifier",
                            tok.kind
                        ),
                        span: tok.span,
                    });
                }
            }
        }

        let end = self.expect(TokenKind::CloseBrace)?;
        let span = Span {
            start: start.start,
            end: end.span.end,
            file: start.file,
        };
        Ok(Expr::InlineAsm(inputs, outputs, ops, span))
    }

    /// Parse turbofish type arguments: `<Type, Type, ...>`
    ///
    /// Called after consuming `::` when the next token is `<`.
    fn parse_turbofish_type_args(&mut self) -> ParseResult<Vec<TypeSig>> {
        // Consume the `<`
        self.expect(TokenKind::Operator(Operator::Comparison(
            ComparisonOperator::LessThan,
        )))?;
        let mut type_args = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            // Check for closing `>`
            if self.check(&TokenKind::Operator(Operator::Comparison(
                ComparisonOperator::GreaterThan,
            ))) {
                self.advance();
                break;
            }
            type_args.push(self.parse_type_sig()?);
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::Comma) {
                self.advance();
            }
        }
        Ok(type_args)
    }

    // ============ Type Parsing ============

    /// Parse a type signature
    fn parse_type_sig(&mut self) -> ParseResult<TypeSig> {
        self.skip_whitespace_and_comments();

        let kind = self.peek().kind.clone();
        match kind {
            TokenKind::DataType(dt) => {
                let token = self.advance();
                // Match on the actual DataType to get the correct PrimitiveType
                let prim_type = match dt {
                    edge_types::tokens::DataType::Primitive(pt) => pt,
                    edge_types::tokens::DataType::Unknown => {
                        return Err(ParseError::InvalidTypeSig {
                            message: "Unknown data type".to_string(),
                            span: token.span,
                        });
                    }
                };
                Ok(TypeSig::Primitive(self.convert_primitive_type(prim_type)))
            }
            TokenKind::Pointer(loc) => {
                let _token = self.advance();
                // Parse the inner type after the pointer location
                let inner = self.parse_type_sig()?;
                let ast_loc = self.convert_location(loc);
                Ok(TypeSig::Pointer(ast_loc, Box::new(inner)))
            }
            TokenKind::Keyword(edge_types::tokens::Keyword::Self_) => {
                let token = self.advance();
                Ok(TypeSig::Named(
                    Ident {
                        name: "Self".to_string(),
                        span: token.span,
                    },
                    Vec::new(),
                ))
            }
            TokenKind::Ident(name) => {
                let token = self.advance();
                let ident = Ident {
                    name,
                    span: token.span,
                };
                // Check for generic type parameters: `Type<T, U>`
                let type_params = if self.check(&TokenKind::Operator(
                    edge_types::tokens::Operator::Comparison(
                        edge_types::tokens::ComparisonOperator::LessThan,
                    ),
                )) {
                    self.advance();
                    let mut params = Vec::new();
                    while !self.is_generic_close() && !self.is_at_end() {
                        params.push(self.parse_type_sig()?);
                        if !self.is_generic_close() {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect_generic_gt()?;
                    params
                } else {
                    Vec::new()
                };
                Ok(TypeSig::Named(ident, type_params))
            }
            TokenKind::OpenParen => {
                self.advance();
                let mut types = Vec::new();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    types.push(self.parse_type_sig()?);
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
                Ok(TypeSig::Tuple(types))
            }
            // Struct type: { field: T, ... }
            TokenKind::OpenBrace => self.parse_struct_type_sig(false),
            // Array type: [T; N]
            TokenKind::OpenBracket => self.parse_array_type_sig(false),
            // Packed struct/tuple/array: packed { ... } or packed (...) or packed [...]
            TokenKind::Keyword(Keyword::Packed) => {
                self.advance();
                self.skip_whitespace_and_comments();
                match self.peek().kind {
                    TokenKind::OpenBrace => self.parse_struct_type_sig(true),
                    TokenKind::OpenParen => {
                        self.advance();
                        let mut types = Vec::new();
                        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                            types.push(self.parse_type_sig()?);
                            if !self.check(&TokenKind::CloseParen) {
                                self.expect(TokenKind::Comma)?;
                            }
                        }
                        self.expect(TokenKind::CloseParen)?;
                        Ok(TypeSig::PackedTuple(types))
                    }
                    TokenKind::OpenBracket => self.parse_array_type_sig(true),
                    _ => {
                        let token = self.peek().clone();
                        Err(ParseError::InvalidTypeSig {
                            message: format!(
                                "Expected '{{', '(' or '[' after 'packed', got: {:?}",
                                token.kind
                            ),
                            span: token.span,
                        })
                    }
                }
            }
            _ => {
                let token = self.peek().clone();
                Err(ParseError::InvalidTypeSig {
                    message: format!("Unexpected token: {:?}", token.kind),
                    span: token.span,
                })
            }
        }
    }

    /// Parse a struct type signature: { field: T, field: T, ... }
    fn parse_struct_type_sig(&mut self, packed: bool) -> ParseResult<TypeSig> {
        self.expect(TokenKind::OpenBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }
            let field_name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let field_ty = self.parse_type_sig()?;
            fields.push(edge_ast::StructField {
                name: field_name,
                ty: field_ty,
            });
            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::CloseBrace) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::CloseBrace)?;

        if packed {
            Ok(TypeSig::PackedStruct(fields))
        } else {
            Ok(TypeSig::Struct(fields))
        }
    }

    /// Parse an array type signature: [T; N]
    fn parse_array_type_sig(&mut self, packed: bool) -> ParseResult<TypeSig> {
        self.expect(TokenKind::OpenBracket)?;
        let elem_ty = self.parse_type_sig()?;
        self.expect(TokenKind::Semicolon)?;
        let size_expr = self.parse_expr()?;
        self.expect(TokenKind::CloseBracket)?;

        if packed {
            Ok(TypeSig::PackedArray(Box::new(elem_ty), Box::new(size_expr)))
        } else {
            Ok(TypeSig::Array(Box::new(elem_ty), Box::new(size_expr)))
        }
    }

    /// Parse optional type parameters: `<T, U: Trait>`
    fn parse_optional_type_params(&mut self) -> ParseResult<Vec<edge_ast::ty::TypeParam>> {
        self.skip_whitespace_and_comments();
        if !self.check(&TokenKind::Operator(
            edge_types::tokens::Operator::Comparison(
                edge_types::tokens::ComparisonOperator::LessThan,
            ),
        )) {
            return Ok(Vec::new());
        }
        self.advance(); // consume <

        let mut params = Vec::new();
        while !self.is_generic_close() && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_generic_close() {
                break;
            }
            let param_name = self.parse_ident()?;

            // Optional trait constraints: T: TraitA & TraitB
            let mut constraints = Vec::new();
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::Colon) {
                self.advance();
                constraints.push(self.parse_ident()?);
                while self.check(&TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
                    edge_types::tokens::BitwiseOperator::And,
                ))) {
                    self.advance();
                    constraints.push(self.parse_ident()?);
                }
            }

            params.push(edge_ast::ty::TypeParam {
                name: param_name,
                constraints,
            });

            self.skip_whitespace_and_comments();
            if !self.is_generic_close() {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect_generic_gt()?;

        Ok(params)
    }

    /// Parse optional type parameters or type arguments: `<T>` or `<u256>`
    ///
    /// Used for impl blocks where the content can be either type parameters
    /// (like `T`) or concrete type arguments (like `u256`).
    fn parse_optional_type_params_or_args(&mut self) -> ParseResult<Vec<edge_ast::ty::TypeParam>> {
        self.skip_whitespace_and_comments();
        if !self.check(&TokenKind::Operator(
            edge_types::tokens::Operator::Comparison(
                edge_types::tokens::ComparisonOperator::LessThan,
            ),
        )) {
            return Ok(Vec::new());
        }
        self.advance(); // consume <

        let mut params = Vec::new();
        while !self.is_generic_close() && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_generic_close() {
                break;
            }
            // Parse as a type signature (handles both idents and concrete types)
            let ty = self.parse_type_sig()?;
            // Convert to TypeParam: use the type name as the param name
            let param_name = match &ty {
                TypeSig::Named(ident, _) => ident.clone(),
                TypeSig::Primitive(p) => Ident {
                    name: p.to_string(),
                    span: Span::EOF,
                },
                _ => Ident {
                    name: format!("{ty:?}"),
                    span: Span::EOF,
                },
            };
            params.push(edge_ast::ty::TypeParam {
                name: param_name,
                constraints: Vec::new(),
            });

            self.skip_whitespace_and_comments();
            if !self.is_generic_close() {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect_generic_gt()?;

        Ok(params)
    }

    /// Parse a type signature that may be a union: `A | B | C(T)`
    ///
    /// Union members use `Ident(Type)` syntax with parentheses, NOT generics.
    /// This is called from `parse_type_assign` where `type X = ...` can be
    /// either a regular type sig or a union.
    fn parse_type_sig_or_union(&mut self) -> ParseResult<TypeSig> {
        self.skip_whitespace_and_comments();

        // Check for leading `|` (optional per spec)
        let has_leading_pipe = self.check(&TokenKind::Operator(
            edge_types::tokens::Operator::Bitwise(edge_types::tokens::BitwiseOperator::Or),
        ));
        if has_leading_pipe {
            self.advance();
            self.skip_whitespace_and_comments();
        }

        // Try to detect a union: if the first token is an ident, check if `(` or `|` follows.
        // Save cursor to backtrack if it's not a union.
        let saved_cursor = self.cursor;

        if let TokenKind::Ident(_) = self.peek().kind {
            let first_member = self.parse_union_member()?;
            self.skip_whitespace_and_comments();

            // If `|` follows (or we had a leading pipe), it's definitely a union
            let is_union = has_leading_pipe
                || self.check(&TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
                    edge_types::tokens::BitwiseOperator::Or,
                )));

            if is_union {
                let mut members = vec![first_member];
                while self.check(&TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
                    edge_types::tokens::BitwiseOperator::Or,
                ))) {
                    self.advance(); // consume |
                    self.skip_whitespace_and_comments();
                    members.push(self.parse_union_member()?);
                    self.skip_whitespace_and_comments();
                }
                return Ok(TypeSig::Union(members));
            }

            // Not a union -- but first_member was parsed as a union member.
            // If the member had inner data `A(T)`, that's only valid in union context.
            // If it was just `A`, it should be treated as a Named type.
            // Backtrack and re-parse as a normal type signature.
            if first_member.inner.is_some() {
                // This is ambiguous -- A(T) is only valid as a union member.
                // But since there's no `|`, treat as error or convert.
                // Actually, backtrack and let normal type sig parsing handle it.
                self.cursor = saved_cursor;
                return self.parse_type_sig();
            }

            // It's just a plain ident -- backtrack and re-parse
            self.cursor = saved_cursor;
            return self.parse_type_sig();
        }

        // Not starting with an ident, just parse as normal type sig
        self.parse_type_sig()
    }

    /// Parse a single union member: `Ident` or `Ident(TypeSig)`
    fn parse_union_member(&mut self) -> ParseResult<edge_ast::UnionMember> {
        let name = self.parse_ident()?;
        self.skip_whitespace_and_comments();
        let inner = if self.check(&TokenKind::OpenParen) {
            self.advance();
            let ty = self.parse_type_sig()?;
            self.expect(TokenKind::CloseParen)?;
            Some(ty)
        } else {
            None
        };
        Ok(edge_ast::UnionMember { name, inner })
    }

    /// Convert from `edge_types` `PrimitiveType` to `edge_ast` `PrimitiveType`
    const fn convert_primitive_type(
        &self,
        pt: edge_types::tokens::types::PrimitiveType,
    ) -> edge_ast::PrimitiveType {
        match pt {
            edge_types::tokens::types::PrimitiveType::UInt(n) => edge_ast::PrimitiveType::UInt(n),
            edge_types::tokens::types::PrimitiveType::Int(n) => edge_ast::PrimitiveType::Int(n),
            edge_types::tokens::types::PrimitiveType::FixedBytes(n) => {
                edge_ast::PrimitiveType::FixedBytes(n)
            }
            edge_types::tokens::types::PrimitiveType::Address => edge_ast::PrimitiveType::Address,
            edge_types::tokens::types::PrimitiveType::Bool => edge_ast::PrimitiveType::Bool,
            edge_types::tokens::types::PrimitiveType::Bit => edge_ast::PrimitiveType::Bit,
            edge_types::tokens::types::PrimitiveType::Pointer(_) => {
                // Pointers in the token type are separate; this shouldn't happen
                edge_ast::PrimitiveType::UInt(256)
            }
        }
    }

    /// Convert from `edge_types` `Location` to `edge_ast` `Location`
    const fn convert_location(&self, loc: edge_types::tokens::Location) -> edge_ast::ty::Location {
        match loc {
            edge_types::tokens::Location::PersistentStorage => edge_ast::ty::Location::Stack,
            edge_types::tokens::Location::TransientStorage => edge_ast::ty::Location::Transient,
            edge_types::tokens::Location::Memory => edge_ast::ty::Location::Memory,
            edge_types::tokens::Location::Calldata => edge_ast::ty::Location::Calldata,
            edge_types::tokens::Location::Returndata => edge_ast::ty::Location::Returndata,
            edge_types::tokens::Location::InternalCode => edge_ast::ty::Location::ImmutableCode,
            edge_types::tokens::Location::ExternalCode => edge_ast::ty::Location::ExternalCode,
        }
    }

    /// Parse function declaration
    fn parse_fn_decl(&mut self) -> ParseResult<FnDecl> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Fn))?;
        let name = self.parse_ident()?;

        // Optional type parameters: fn identity<T>(...)
        let type_params = self.parse_optional_type_params()?;

        self.expect(TokenKind::OpenParen)?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseParen) {
                break;
            }
            let param_name = self.parse_ident()?;

            // `self` parameter can omit type annotation
            self.skip_whitespace_and_comments();
            let param_type = if self.check(&TokenKind::Colon) {
                self.advance();
                self.parse_type_sig()?
            } else {
                // No type annotation -- use Named("Self") as implicit type
                TypeSig::Named(
                    Ident {
                        name: "Self".to_string(),
                        span: param_name.span.clone(),
                    },
                    Vec::new(),
                )
            };
            params.push((param_name, param_type));

            self.skip_whitespace_and_comments();
            if !self.check(&TokenKind::CloseParen) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::CloseParen)?;

        let mut returns = Vec::new();
        self.skip_whitespace_and_comments();
        if self.check(&TokenKind::Arrow) {
            self.advance();
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::OpenParen) {
                self.advance();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    self.skip_whitespace_and_comments();
                    if self.check(&TokenKind::CloseParen) {
                        break;
                    }
                    returns.push(self.parse_type_sig()?);
                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseParen) {
                        self.expect(TokenKind::Comma)?;
                    }
                }
                self.expect(TokenKind::CloseParen)?;
            } else {
                returns.push(self.parse_type_sig()?);
            }
        }

        Ok(FnDecl {
            name,
            type_params,
            params,
            returns,
            is_pub: false,
            is_ext: false,
            is_mut: false,
            span: start_tok.span,
        })
    }

    // ============ Helper Methods for Operators ============

    /// Get precedence and right-associativity of current operator
    fn get_operator_precedence(&self) -> Option<(u8, bool)> {
        use edge_types::tokens::{
            ArithmeticOperator, BitwiseOperator, ComparisonOperator, LogicalOperator, Operator,
        };

        match &self.peek().kind {
            TokenKind::Operator(Operator::Assignment) => Some((0, true)), // Lowest precedence, right-associative
            TokenKind::Operator(Operator::Logical(op)) => Some(match op {
                LogicalOperator::Or => (1, false),
                LogicalOperator::And => (2, false),
                _ => return None,
            }),
            TokenKind::Operator(Operator::Comparison(op)) => Some(match op {
                ComparisonOperator::Equal | ComparisonOperator::NotEqual => (3, false),
                _ => (4, false),
            }),
            TokenKind::Operator(Operator::Bitwise(op)) => Some(match op {
                BitwiseOperator::Or => (5, false),
                BitwiseOperator::Xor => (6, false),
                BitwiseOperator::And => (7, false),
                BitwiseOperator::LeftShift | BitwiseOperator::RightShift => (8, false),
                _ => return None,
            }),
            TokenKind::Operator(Operator::Arithmetic(op)) => {
                Some(match op {
                    ArithmeticOperator::Add | ArithmeticOperator::Sub => (9, false),
                    ArithmeticOperator::Mul | ArithmeticOperator::Div | ArithmeticOperator::Mod => {
                        (10, false)
                    }
                    ArithmeticOperator::Exp => (11, true), // Right associative
                })
            }
            _ => None,
        }
    }

    /// Parse binary operator
    fn parse_bin_op(&mut self) -> ParseResult<BinOp> {
        use edge_types::tokens::{
            ArithmeticOperator, BitwiseOperator, ComparisonOperator, LogicalOperator, Operator,
        };

        let op = match &self.peek().kind {
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Add)) => BinOp::Add,
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Sub)) => BinOp::Sub,
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Mul)) => BinOp::Mul,
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Div)) => BinOp::Div,
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Mod)) => BinOp::Mod,
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Exp)) => BinOp::Exp,
            TokenKind::Operator(Operator::Logical(LogicalOperator::And)) => BinOp::LogicalAnd,
            TokenKind::Operator(Operator::Logical(LogicalOperator::Or)) => BinOp::LogicalOr,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::Equal)) => BinOp::Eq,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::NotEqual)) => BinOp::Neq,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::LessThan)) => BinOp::Lt,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)) => {
                BinOp::Lte
            }
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)) => BinOp::Gt,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)) => {
                BinOp::Gte
            }
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::And)) => BinOp::BitwiseAnd,
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Or)) => BinOp::BitwiseOr,
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Xor)) => BinOp::BitwiseXor,
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::LeftShift)) => BinOp::Shl,
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::RightShift)) => BinOp::Shr,
            _ => {
                return Err(ParseError::InvalidExpr {
                    message: "Expected operator".to_string(),
                    span: self.peek().span.clone(),
                })
            }
        };

        self.advance();
        Ok(op)
    }

    /// Try to parse unary operator
    fn try_parse_unary_op(&self) -> Option<UnaryOp> {
        use edge_types::tokens::{ArithmeticOperator, BitwiseOperator, LogicalOperator, Operator};

        match &self.peek().kind {
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Sub)) => {
                Some(UnaryOp::Neg)
            }
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Not)) => {
                Some(UnaryOp::BitwiseNot)
            }
            TokenKind::Operator(Operator::Logical(LogicalOperator::Not)) => {
                Some(UnaryOp::LogicalNot)
            }
            _ => None,
        }
    }

    // ============ Code Block Parsing ============

    /// Parse a code block
    fn parse_code_block(&mut self) -> ParseResult<CodeBlock> {
        let start = self.expect(TokenKind::OpenBrace)?.span;

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            // Skip orphan semicolons (e.g. after match {} ;)
            if self.check(&TokenKind::Semicolon) {
                self.advance();
                continue;
            }

            // Try parsing as a statement. If the expression statement fails
            // because of a missing semicolon before `}`, treat it as a tail
            // expression (like Rust's implicit return).
            let saved_cursor = self.cursor;
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(BlockItem::Stmt(Box::new(stmt))),
                Err(_) => {
                    // Try parsing as a tail expression (no semicolon needed
                    // when followed by `}`)
                    self.cursor = saved_cursor;
                    let expr = self.parse_expr()?;
                    // If '}' follows, this is a tail expression
                    self.skip_whitespace_and_comments();
                    if !self.check(&TokenKind::CloseBrace) {
                        // Not a tail expression, need semicolon
                        self.expect(TokenKind::Semicolon)?;
                    }
                    stmts.push(BlockItem::Stmt(Box::new(Stmt::Expr(expr))));
                }
            }
        }

        let end = self.expect(TokenKind::CloseBrace)?.span;

        Ok(CodeBlock {
            stmts,
            span: Span {
                start: start.start,
                end: end.end,
                file: start.file,
            },
        })
    }

    /// Parse loop block items
    fn parse_loop_block_items(&mut self) -> ParseResult<Vec<LoopItem>> {
        self.expect(TokenKind::OpenBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            items.push(LoopItem::Stmt(Box::new(self.parse_stmt()?)));
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(items)
    }

    // ============ Generic Type Helpers ============

    /// Returns true if the current token closes a generic parameter list (`>` or `>>`).
    /// `>>` can appear when closing nested generics like `map<K, map<K, V>>`.
    fn is_generic_close(&self) -> bool {
        matches!(
            &self.peek().kind,
            TokenKind::Operator(edge_types::tokens::Operator::Comparison(
                edge_types::tokens::ComparisonOperator::GreaterThan
            )) | TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
                edge_types::tokens::BitwiseOperator::RightShift
            ))
        )
    }

    /// Consume a closing `>` for a generic parameter list.
    /// When `>>` is encountered (nested generics like `map<K, map<K, V>>`), consume it
    /// and insert a synthetic `>` token so the enclosing generic context can close cleanly.
    fn expect_generic_gt(&mut self) -> ParseResult<()> {
        self.skip_whitespace_and_comments();
        let gt = TokenKind::Operator(edge_types::tokens::Operator::Comparison(
            edge_types::tokens::ComparisonOperator::GreaterThan,
        ));
        if self.check(&gt) {
            self.advance();
            Ok(())
        } else if self.check(&TokenKind::Operator(edge_types::tokens::Operator::Bitwise(
            edge_types::tokens::BitwiseOperator::RightShift,
        ))) {
            // Split >> into two >: consume and insert a synthetic > for the enclosing context.
            let tok = self.advance();
            let synthetic = Token {
                kind: gt,
                span: Span {
                    start: tok.span.end,
                    end: tok.span.end,
                    file: tok.span.file,
                },
            };
            self.tokens.insert(self.cursor, synthetic);
            Ok(())
        } else {
            let token = self.peek().clone();
            Err(ParseError::unexpected(
                &token.kind,
                TokenKind::Operator(edge_types::tokens::Operator::Comparison(
                    edge_types::tokens::ComparisonOperator::GreaterThan,
                )),
                token.span,
            ))
        }
    }

    // ============ Identifier Parsing ============

    /// Parse an identifier
    fn parse_ident(&mut self) -> ParseResult<Ident> {
        self.skip_whitespace_and_comments();

        let kind = self.peek().kind.clone();
        match kind {
            TokenKind::Ident(name) => {
                let token = self.advance();
                Ok(Ident {
                    name,
                    span: token.span,
                })
            }
            // `self` and `Self` keywords can be used as identifiers in certain contexts
            TokenKind::Keyword(Keyword::Self_) => {
                let token = self.advance();
                Ok(Ident {
                    name: "self".to_string(),
                    span: token.span,
                })
            }
            // `super` keyword can be used as an identifier in path contexts
            TokenKind::Keyword(Keyword::Super) => {
                let token = self.advance();
                Ok(Ident {
                    name: "super".to_string(),
                    span: token.span,
                })
            }
            _ => {
                let token = self.peek().clone();
                Err(ParseError::unexpected(
                    &token.kind,
                    "identifier",
                    token.span,
                ))
            }
        }
    }
}

/// Check if an uppercase string is a known EVM opcode mnemonic.
fn is_evm_opcode(name: &str) -> bool {
    matches!(
        name,
        "STOP"
            | "ADD"
            | "MUL"
            | "SUB"
            | "DIV"
            | "SDIV"
            | "MOD"
            | "SMOD"
            | "ADDMOD"
            | "MULMOD"
            | "EXP"
            | "SIGNEXTEND"
            | "LT"
            | "GT"
            | "SLT"
            | "SGT"
            | "EQ"
            | "ISZERO"
            | "AND"
            | "OR"
            | "XOR"
            | "NOT"
            | "BYTE"
            | "SHL"
            | "SHR"
            | "SAR"
            | "KECCAK256"
            | "SHA3"
            | "ADDRESS"
            | "BALANCE"
            | "ORIGIN"
            | "CALLER"
            | "CALLVALUE"
            | "CALLDATALOAD"
            | "CALLDATASIZE"
            | "CALLDATACOPY"
            | "CODESIZE"
            | "CODECOPY"
            | "GASPRICE"
            | "EXTCODESIZE"
            | "EXTCODECOPY"
            | "RETURNDATASIZE"
            | "RETURNDATACOPY"
            | "EXTCODEHASH"
            | "BLOCKHASH"
            | "COINBASE"
            | "TIMESTAMP"
            | "NUMBER"
            | "PREVRANDAO"
            | "DIFFICULTY"
            | "GASLIMIT"
            | "CHAINID"
            | "SELFBALANCE"
            | "BASEFEE"
            | "POP"
            | "MLOAD"
            | "MSTORE"
            | "MSTORE8"
            | "SLOAD"
            | "SSTORE"
            | "JUMP"
            | "JUMPI"
            | "PC"
            | "MSIZE"
            | "GAS"
            | "JUMPDEST"
            | "TLOAD"
            | "TSTORE"
            | "MCOPY"
            | "PUSH0"
            | "PUSH1"
            | "PUSH2"
            | "PUSH3"
            | "PUSH4"
            | "PUSH5"
            | "PUSH6"
            | "PUSH7"
            | "PUSH8"
            | "PUSH9"
            | "PUSH10"
            | "PUSH11"
            | "PUSH12"
            | "PUSH13"
            | "PUSH14"
            | "PUSH15"
            | "PUSH16"
            | "PUSH17"
            | "PUSH18"
            | "PUSH19"
            | "PUSH20"
            | "PUSH21"
            | "PUSH22"
            | "PUSH23"
            | "PUSH24"
            | "PUSH25"
            | "PUSH26"
            | "PUSH27"
            | "PUSH28"
            | "PUSH29"
            | "PUSH30"
            | "PUSH31"
            | "PUSH32"
            | "DUP1"
            | "DUP2"
            | "DUP3"
            | "DUP4"
            | "DUP5"
            | "DUP6"
            | "DUP7"
            | "DUP8"
            | "DUP9"
            | "DUP10"
            | "DUP11"
            | "DUP12"
            | "DUP13"
            | "DUP14"
            | "DUP15"
            | "DUP16"
            | "SWAP1"
            | "SWAP2"
            | "SWAP3"
            | "SWAP4"
            | "SWAP5"
            | "SWAP6"
            | "SWAP7"
            | "SWAP8"
            | "SWAP9"
            | "SWAP10"
            | "SWAP11"
            | "SWAP12"
            | "SWAP13"
            | "SWAP14"
            | "SWAP15"
            | "SWAP16"
            | "LOG0"
            | "LOG1"
            | "LOG2"
            | "LOG3"
            | "LOG4"
            | "CREATE"
            | "CALL"
            | "CALLCODE"
            | "RETURN"
            | "DELEGATECALL"
            | "CREATE2"
            | "STATICCALL"
            | "REVERT"
            | "INVALID"
            | "SELFDESTRUCT"
            | "BLOBHASH"
            | "BLOBBASEFEE"
    )
}
