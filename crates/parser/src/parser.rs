//! The Edge Language Parser
//!
//! Implements a recursive descent parser with Pratt parsing for expressions.

use edge_ast::{
    BinOp, CodeBlock, BlockItem, LoopBlock, LoopItem, Expr, FnDecl, Ident, Lit, Program, Stmt, TypeSig, UnaryOp,
};
use edge_lexer::lexer::Lexer;
use edge_types::span::Span;
use edge_types::tokens::{Keyword, Operator, Token, TokenKind};

use crate::errors::{ParseError, ParseResult};

/// The parser struct
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
            let token = lexer.next_token().map_err(|e| {
                ParseError::LexerError(format!("{:?}", e))
            })?;

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
            edge_ast::ConstDecl { name, ty, span: span.clone() },
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
            edge_ast::TypeDecl { name, type_params: Vec::new(), is_pub: false, span: span.clone() },
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

    /// Parse expression statement
    fn parse_expr_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        let span = expr.span();
        Ok(Stmt::VarAssign(expr, Expr::Literal(Box::new(Lit::Bool(false, span.clone()))), span))
    }

    /// Parse if-else statement
    fn parse_if_else(&mut self) -> ParseResult<Stmt> {
        let _start = self.expect(TokenKind::Keyword(Keyword::If))?;
        self.expect(TokenKind::OpenParen)?;
        let _cond = self.parse_expr()?;
        self.expect(TokenKind::CloseParen)?;
        let _block = self.parse_code_block()?;

        // For now, return a placeholder loop statement
        Ok(Stmt::Loop(LoopBlock {
            items: vec![],
            span: _start.span,
        }))
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
            LoopBlock { items, span: start_tok.span },
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
            LoopBlock { items, span: start_tok.span },
        ))
    }

    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Return))?;
        let expr = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::VarAssign(
            Expr::Ident(Ident {
                name: "return".to_string(),
                span: start_tok.span.clone(),
            }),
            expr,
            start_tok.span,
        ))
    }

    /// Parse code block as statement
    fn parse_code_block_stmt(&mut self) -> ParseResult<Stmt> {
        let block = self.parse_code_block()?;
        let span = block.span.clone();

        let loop_items = block.stmts.into_iter().map(|item| match item {
            BlockItem::Expr(e) => LoopItem::Expr(e),
            BlockItem::Stmt(s) => LoopItem::Stmt(s),
        }).collect();

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

        Ok(left)
    }

    /// Parse unary expression
    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        self.skip_whitespace_and_comments();

        if let Some(unary_op) = self.try_parse_unary_op() {
            let start = self.advance().span.clone();
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
            TokenKind::Literal(_) => {
                let token = self.advance();
                let lit = Lit::Int(42, None, token.span);
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
                    span: token.span,
                };
                Ok(Expr::Ident(ident))
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
            TokenKind::DataType(_) => {
                self.advance();
                Ok(TypeSig::Primitive(edge_ast::PrimitiveType::UInt(8)))
            }
            TokenKind::Ident(name) => {
                let token = self.advance();
                let ident = Ident {
                    name,
                    span: token.span,
                };
                Ok(TypeSig::Named(ident, Vec::new()))
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

    /// Parse function declaration
    fn parse_fn_decl(&mut self) -> ParseResult<FnDecl> {
        let start_tok = self.expect(TokenKind::Keyword(Keyword::Fn))?;
        let name = self.parse_ident()?;

        self.expect(TokenKind::OpenParen)?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
            let param_name = self.parse_ident()?;
            self.expect(TokenKind::Colon)?;
            let param_type = self.parse_type_sig()?;
            params.push((param_name, param_type));

            if !self.check(&TokenKind::CloseParen) {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::CloseParen)?;

        let mut returns = Vec::new();
        if self.check(&TokenKind::Arrow) {
            self.advance();
            if self.check(&TokenKind::OpenParen) {
                self.advance();
                while !self.check(&TokenKind::CloseParen) && !self.is_at_end() {
                    returns.push(self.parse_type_sig()?);
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
        use edge_types::tokens::{Operator, LogicalOperator, ComparisonOperator, BitwiseOperator, ArithmeticOperator};

        match &self.peek().kind {
            TokenKind::Operator(Operator::Logical(op)) => {
                Some(match op {
                    LogicalOperator::Or => (1, false),
                    LogicalOperator::And => (2, false),
                    _ => return None,
                })
            }
            TokenKind::Operator(Operator::Comparison(op)) => {
                Some(match op {
                    ComparisonOperator::Equal | ComparisonOperator::NotEqual => (3, false),
                    _ => (4, false),
                })
            }
            TokenKind::Operator(Operator::Bitwise(op)) => {
                Some(match op {
                    BitwiseOperator::Or => (5, false),
                    BitwiseOperator::Xor => (6, false),
                    BitwiseOperator::And => (7, false),
                    BitwiseOperator::LeftShift | BitwiseOperator::RightShift => (8, false),
                    _ => return None,
                })
            }
            TokenKind::Operator(Operator::Arithmetic(op)) => {
                Some(match op {
                    ArithmeticOperator::Add | ArithmeticOperator::Sub => (9, false),
                    ArithmeticOperator::Mul | ArithmeticOperator::Div | ArithmeticOperator::Mod => (10, false),
                    ArithmeticOperator::Exp => (11, true),  // Right associative
                })
            }
            _ => None,
        }
    }

    /// Parse binary operator
    fn parse_bin_op(&mut self) -> ParseResult<BinOp> {
        use edge_types::tokens::{Operator, LogicalOperator, ComparisonOperator, BitwiseOperator, ArithmeticOperator};

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
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::LessThanOrEqual)) => BinOp::Lte,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::GreaterThan)) => BinOp::Gt,
            TokenKind::Operator(Operator::Comparison(ComparisonOperator::GreaterThanOrEqual)) => BinOp::Gte,
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
        use edge_types::tokens::{Operator, ArithmeticOperator, BitwiseOperator, LogicalOperator};

        match &self.peek().kind {
            TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Sub)) => Some(UnaryOp::Neg),
            TokenKind::Operator(Operator::Bitwise(BitwiseOperator::Not)) => Some(UnaryOp::BitwiseNot),
            TokenKind::Operator(Operator::Logical(LogicalOperator::Not)) => Some(UnaryOp::LogicalNot),
            _ => None,
        }
    }

    // ============ Code Block Parsing ============

    /// Parse a code block
    fn parse_code_block(&mut self) -> ParseResult<CodeBlock> {
        let start = self.expect(TokenKind::OpenBrace)?.span.clone();

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.check(&TokenKind::CloseBrace) {
                break;
            }

            stmts.push(BlockItem::Stmt(Box::new(self.parse_stmt()?)));
        }

        let end = self.expect(TokenKind::CloseBrace)?.span.clone();

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
