//! The Edge Language Parser
//!
//! Implements a recursive descent parser with Pratt parsing for expressions.

use edge_ast::{
    BinOp, BlockItem, CodeBlock, Expr, FnDecl, Ident, Lit, LoopBlock, LoopItem, Program, Stmt,
    TypeSig, UnaryOp,
};
use edge_lexer::lexer::Lexer;
use edge_types::{
    span::Span,
    tokens::{Keyword, Operator, Token, TokenKind},
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
        let mut tokens = Vec::new();

        loop {
            let token = lexer
                .next_token()
                .map_err(|e| ParseError::LexerError(format!("{e:?}")))?;

            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
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
            _ => self.parse_expr_stmt(),
        }
    }

    /// Parse variable declaration
    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Let))?;
        let name = self.parse_ident()?;

        let ty = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_sig()?)
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;
        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::VarDecl(name, ty, span))
    }

    /// Parse constant assignment
    fn parse_const_assign(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Const))?;
        let name = self.parse_ident()?;
        self.expect(TokenKind::Colon)?;
        let ty = Some(self.parse_type_sig()?);
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
        self.expect(TokenKind::Operator(Operator::Assignment))?;
        let ty = self.parse_type_sig()?;
        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::TypeAssign(
            edge_ast::TypeDecl {
                name,
                type_params: Vec::new(),
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
        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ModuleDecl(edge_ast::ModuleDecl {
            name,
            is_pub: false,
            doc: None,
            items: Vec::new(),
            span,
        }))
    }

    /// Parse use/import statement
    fn parse_use(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Use))?;
        let root = self.parse_ident()?;

        let path = if self.check(&TokenKind::DoubleColon) {
            self.advance();
            let mut path_segments = vec![self.parse_ident()?];

            while self.check(&TokenKind::DoubleColon) {
                self.advance();
                path_segments.push(self.parse_ident()?);
            }

            // For now, use the last segment as the import path
            path_segments
                .last()
                .map(|last| edge_ast::ImportPath::Ident(last.clone()))
        } else {
            None
        };

        self.expect(TokenKind::Semicolon)?;

        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        Ok(Stmt::ModuleImport(edge_ast::ModuleImport {
            root,
            path,
            is_pub: false,
            span,
        }))
    }

    /// Parse trait declaration (stub - just skip for now)
    fn parse_trait_stub(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Trait))?;
        let _name = self.parse_ident()?;

        // Skip to opening brace
        while !self.check(&TokenKind::OpenBrace) && !self.is_at_end() {
            self.advance();
        }

        self.expect(TokenKind::OpenBrace)?;

        // Skip everything until closing brace
        let mut depth = 1;
        while depth > 0 && !self.is_at_end() {
            if self.check(&TokenKind::OpenBrace) {
                depth += 1;
            } else if self.check(&TokenKind::CloseBrace) {
                depth -= 1;
            }
            self.advance();
        }

        // Create a stub TraitDecl
        let span = Span {
            start: start_tok.span.start,
            end: self.tokens[self.cursor - 1].span.end,
            file: start_tok.span.file,
        };

        let stub_name_span = span.clone();

        Ok(Stmt::TraitDecl(
            edge_ast::TraitDecl {
                name: Ident {
                    name: "Stub".to_string(),
                    span: stub_name_span,
                },
                type_params: Vec::new(),
                supertraits: Vec::new(),
                items: Vec::new(),
                is_pub: false,
                span: span.clone(),
            },
            span,
        ))
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
                // Skip const fields for now
                self.advance();
                let _const_name = self.parse_ident()?;
                self.expect(TokenKind::Colon)?;
                let _const_type = self.parse_type_sig()?;
                self.expect(TokenKind::Operator(Operator::Assignment))?;
                let _expr = self.parse_expr()?;
                self.expect(TokenKind::Semicolon)?;
            } else if self.check(&TokenKind::Keyword(Keyword::Fn))
                || self.check(&TokenKind::Keyword(Keyword::Pub))
            {
                // Skip function definitions
                // Consume tokens until we hit the next let/const or closing brace
                let mut brace_depth = 0;
                loop {
                    if self.check(&TokenKind::OpenBrace) {
                        brace_depth += 1;
                    } else if self.check(&TokenKind::CloseBrace) {
                        if brace_depth == 0 {
                            break;
                        }
                        brace_depth -= 1;
                    }
                    self.advance();
                    if self.is_at_end() {
                        break;
                    }

                    if brace_depth == 0
                        && (self.check(&TokenKind::Keyword(Keyword::Let))
                            || self.check(&TokenKind::Keyword(Keyword::Const))
                            || self.check(&TokenKind::Keyword(Keyword::Fn))
                            || self.check(&TokenKind::Keyword(Keyword::Pub))
                            || self.check(&TokenKind::CloseBrace))
                    {
                        break;
                    }
                }
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
            span,
        }))
    }

    /// Parse expression statement
    fn parse_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        let span = expr.span();
        Ok(Stmt::VarAssign(
            expr,
            Expr::Literal(Box::new(Lit::Bool(false, span.clone()))),
            span,
        ))
    }

    /// Parse if-else statement
    fn parse_if_else(&mut self) -> ParseResult<Stmt> {
        let _start = self.expect(TokenKind::Keyword(Keyword::If))?;
        self.expect(TokenKind::OpenParen)?;
        let cond = self.parse_expr()?;
        self.expect(TokenKind::CloseParen)?;
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

            if self.check(&TokenKind::Keyword(Keyword::If)) {
                // else if
                self.advance(); // consume 'if'
                self.expect(TokenKind::OpenParen)?;
                let elif_cond = self.parse_expr()?;
                self.expect(TokenKind::CloseParen)?;
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

    /// Parse match statement
    fn parse_match(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Match))?;
        let _expr = self.parse_expr()?;
        self.expect(TokenKind::OpenBrace)?;

        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }
            // Skip match arms for now
            self.advance();
        }

        self.expect(TokenKind::CloseBrace)?;

        Ok(Stmt::Loop(LoopBlock {
            items: vec![],
            span: start_tok.span,
        }))
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

        let init = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_stmt()?))
        };

        let cond = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(TokenKind::Semicolon)?;

        let update = if self.check(&TokenKind::CloseParen) {
            None
        } else {
            Some(Box::new(self.parse_stmt()?))
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

    /// Parse code block as statement
    fn parse_code_block_stmt(&mut self) -> ParseResult<Stmt> {
        let block = self.parse_code_block()?;
        let span = block.span.clone();

        let loop_items = block
            .stmts
            .into_iter()
            .map(|item| match item {
                BlockItem::Expr(e) => LoopItem::Expr(e),
                BlockItem::Stmt(s) => LoopItem::Stmt(s),
            })
            .collect();

        Ok(Stmt::Loop(LoopBlock {
            items: loop_items,
            span,
        }))
    }

    // ============ Expression Parsing ============

    /// Parse an expression
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_binary_expr(0)
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

                    expr = Expr::FunctionCall(Box::new(expr), args, span);
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
                    if let TokenKind::Ident(field_name) = &self.peek().kind {
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
                    while self.check(&TokenKind::DoubleColon) {
                        self.advance();
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
                    let end_span = path_segments.last().unwrap().span.clone();
                    let span = Span {
                        start: token.span.start,
                        end: end_span.end,
                        file: token.span.file,
                    };
                    Ok(Expr::Path(path_segments, span))
                } else {
                    Ok(Expr::Ident(ident))
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
                let expr = self.parse_expr()?;
                self.expect(TokenKind::CloseParen)?;

                let span = Span {
                    start: start.start,
                    end: self.tokens[self.cursor - 1].span.end,
                    file: start.file,
                };

                Ok(Expr::Paren(Box::new(expr), span))
            }
            _ => {
                let token = self.peek().clone();
                Err(ParseError::InvalidExpr {
                    message: format!("Unexpected token: {:?}", token.kind),
                    span: token.span,
                })
            }
        }
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
                    while !self.check(&TokenKind::Operator(
                        edge_types::tokens::Operator::Comparison(
                            edge_types::tokens::ComparisonOperator::GreaterThan,
                        ),
                    )) && !self.is_at_end()
                    {
                        params.push(self.parse_type_sig()?);
                        if !self.check(&TokenKind::Operator(
                            edge_types::tokens::Operator::Comparison(
                                edge_types::tokens::ComparisonOperator::GreaterThan,
                            ),
                        )) {
                            self.expect(TokenKind::Comma)?;
                        }
                    }
                    self.expect(TokenKind::Operator(
                        edge_types::tokens::Operator::Comparison(
                            edge_types::tokens::ComparisonOperator::GreaterThan,
                        ),
                    ))?;
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
            _ => {
                let token = self.peek().clone();
                Err(ParseError::InvalidTypeSig {
                    message: format!("Unexpected token: {:?}", token.kind),
                    span: token.span,
                })
            }
        }
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

        Ok(FnDecl {
            name,
            type_params: Vec::new(),
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

            stmts.push(BlockItem::Stmt(Box::new(self.parse_stmt()?)));
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

    // ============ Identifier Parsing ============

    /// Parse an identifier
    fn parse_ident(&mut self) -> ParseResult<Ident> {
        self.skip_whitespace_and_comments();

        let kind = self.peek().kind.clone();
        if let TokenKind::Ident(name) = kind {
            let token = self.advance();
            Ok(Ident {
                name,
                span: token.span,
            })
        } else {
            let token = self.peek().clone();
            Err(ParseError::unexpected(
                &token.kind,
                "identifier",
                token.span,
            ))
        }
    }
}
