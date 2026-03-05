use edge_lexer::lexer::Lexer;
use edge_types::prelude::*;

/// Helper: collect all non-whitespace token kinds from source.
fn lex_non_ws(source: &str) -> Vec<TokenKind> {
    let lexer = Lexer::new(source);
    lexer
        .filter_map(|r| r.ok())
        .filter(|t| t.kind != TokenKind::Whitespace)
        .map(|t| t.kind)
        .collect()
}

// ─── EVM Primitive Types ────────────────────────────────────────────

#[test]
fn lex_u8_type() {
    let mut lexer = Lexer::new("u8");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(8))),
            Span::new(0..1, None),
        )
    );
    let eof = lexer.next().unwrap().unwrap();
    assert_eq!(eof.kind, TokenKind::Eof);
}

#[test]
fn lex_u256_type() {
    let mut lexer = Lexer::new("u256");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(256))),
            Span::new(0..3, None),
        )
    );
}

#[test]
fn lex_i128_type() {
    let mut lexer = Lexer::new("i128");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::Int(128))),
            Span::new(0..3, None),
        )
    );
}

#[test]
fn lex_b32_type() {
    let mut lexer = Lexer::new("b32");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::FixedBytes(32))),
            Span::new(0..2, None),
        )
    );
}

#[test]
fn lex_addr_type() {
    let mut lexer = Lexer::new("addr");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::Address)),
            Span::new(0..3, None),
        )
    );
}

#[test]
fn lex_bool_type() {
    let mut lexer = Lexer::new("bool");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::Bool)),
            Span::new(0..3, None),
        )
    );
}

#[test]
fn lex_bit_type() {
    let mut lexer = Lexer::new("bit");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(
            TokenKind::DataType(DataType::Primitive(PrimitiveType::Bit)),
            Span::new(0..2, None),
        )
    );
}

// ─── Arithmetic Operators ───────────────────────────────────────────

#[test]
fn lex_add_operator() {
    let mut lexer = Lexer::new("+");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Add))
    );
    assert_eq!(tok.span, Span::new(0..0, None));
}

#[test]
fn lex_sub_operator() {
    let mut lexer = Lexer::new("-");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Sub))
    );
}

#[test]
fn lex_mul_operator() {
    let mut lexer = Lexer::new("*");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Mul))
    );
}

#[test]
fn lex_div_operator() {
    let mut lexer = Lexer::new("/");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Div))
    );
}

#[test]
fn lex_mod_operator() {
    let mut lexer = Lexer::new("%");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Mod))
    );
}

#[test]
fn lex_exp_operator() {
    let mut lexer = Lexer::new("**");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Exp))
    );
}

// ─── Compound Assignment Operators ──────────────────────────────────

#[test]
fn lex_add_assign() {
    let mut lexer = Lexer::new("+=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::CompoundAssignment(
            CompoundAssignmentOperator::AddAssign
        ))
    );
}

#[test]
fn lex_sub_assign() {
    let mut lexer = Lexer::new("-=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::CompoundAssignment(
            CompoundAssignmentOperator::SubAssign
        ))
    );
}

#[test]
fn lex_mul_assign() {
    let mut lexer = Lexer::new("*=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::CompoundAssignment(
            CompoundAssignmentOperator::MulAssign
        ))
    );
}

#[test]
fn lex_div_is_not_compound() {
    // The lexer does not currently implement /= as compound assignment;
    // "/" is always lexed as Div regardless of what follows.
    let mut lexer = Lexer::new("/=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Div))
    );
}

#[test]
fn lex_mod_assign() {
    let mut lexer = Lexer::new("%=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::CompoundAssignment(
            CompoundAssignmentOperator::ModAssign
        ))
    );
}

// ─── Data Location Annotations ──────────────────────────────────────

#[test]
fn lex_storage_pointer() {
    let mut lexer = Lexer::new("&s");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Pointer(Location::PersistentStorage));
}

#[test]
fn lex_memory_pointer() {
    let mut lexer = Lexer::new("&m");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Pointer(Location::Memory));
}

#[test]
fn lex_calldata_pointer() {
    let mut lexer = Lexer::new("&cd");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Pointer(Location::Calldata));
}

#[test]
fn lex_transient_storage_pointer() {
    let mut lexer = Lexer::new("&t");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Pointer(Location::TransientStorage));
}

#[test]
fn lex_returndata_pointer() {
    let mut lexer = Lexer::new("&rd");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Pointer(Location::Returndata));
}

// ─── String Literals ────────────────────────────────────────────────

#[test]
fn lex_string_literal() {
    let mut lexer = Lexer::new("\"hello world\"");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::StringLiteral("hello world".to_string())
    );
    assert_eq!(tok.span, Span::new(0..12, None));
}

#[test]
fn lex_empty_string_literal() {
    let mut lexer = Lexer::new("\"\"");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::StringLiteral("".to_string()));
}

#[test]
fn lex_string_literal_with_escape() {
    let mut lexer = Lexer::new("\"hello\\nworld\"");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::StringLiteral("hello\nworld".to_string())
    );
}

// ─── Hex Literals ───────────────────────────────────────────────────

#[test]
fn lex_hex_literal() {
    let mut lexer = Lexer::new("0xff");
    let tok = lexer.next().unwrap().unwrap();
    // The lexer produces a Literal token for hex values.
    assert!(matches!(tok.kind, TokenKind::Literal(_)));
}

#[test]
fn lex_hex_literal_single_digit() {
    let mut lexer = Lexer::new("0x01");
    let tok = lexer.next().unwrap().unwrap();
    assert!(matches!(tok.kind, TokenKind::Literal(_)));
}

// ─── Decimal Literals ───────────────────────────────────────────────

#[test]
fn lex_decimal_literal() {
    let mut lexer = Lexer::new("42");
    let tok = lexer.next().unwrap().unwrap();
    assert!(matches!(tok.kind, TokenKind::Literal(_)));
    // Decimal 42 = 0x2a
    assert_eq!(
        tok.kind,
        TokenKind::Literal(edge_types::bytes::decimal_to_bytes32("42").into())
    );
}

// ─── Keywords ───────────────────────────────────────────────────────

#[test]
fn lex_keyword_let() {
    let mut lexer = Lexer::new("let");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(TokenKind::Keyword(Keyword::Let), Span::new(0..2, None))
    );
}

#[test]
fn lex_keyword_fn() {
    let mut lexer = Lexer::new("fn");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok,
        Token::new(TokenKind::Keyword(Keyword::Fn), Span::new(0..1, None))
    );
}

#[test]
fn lex_keyword_contract() {
    let mut lexer = Lexer::new("contract");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Keyword(Keyword::Contract));
    assert_eq!(tok.span, Span::new(0..7, None));
}

#[test]
fn lex_keyword_event() {
    let mut lexer = Lexer::new("event");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Keyword(Keyword::Event));
    assert_eq!(tok.span, Span::new(0..4, None));
}

#[test]
fn lex_keyword_comptime() {
    let mut lexer = Lexer::new("comptime");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Keyword(Keyword::Comptime));
    assert_eq!(tok.span, Span::new(0..7, None));
}

// ─── Comparison and Logical Operators ───────────────────────────────

#[test]
fn lex_equal_operator() {
    let mut lexer = Lexer::new("==");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Comparison(ComparisonOperator::Equal))
    );
}

#[test]
fn lex_not_equal_operator() {
    let mut lexer = Lexer::new("!=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Comparison(ComparisonOperator::NotEqual))
    );
}

#[test]
fn lex_logical_and() {
    let mut lexer = Lexer::new("&&");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Logical(LogicalOperator::And))
    );
}

#[test]
fn lex_logical_or() {
    let mut lexer = Lexer::new("||");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(
        tok.kind,
        TokenKind::Operator(Operator::Logical(LogicalOperator::Or))
    );
}

// ─── Punctuation ────────────────────────────────────────────────────

#[test]
fn lex_arrow() {
    let mut lexer = Lexer::new("->");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Arrow);
}

#[test]
fn lex_fat_arrow() {
    let mut lexer = Lexer::new("=>");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::FatArrow);
}

#[test]
fn lex_double_colon() {
    let mut lexer = Lexer::new("::");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::DoubleColon);
}

// ─── Combined Token Sequences ───────────────────────────────────────

#[test]
fn lex_var_decl_tokens() {
    // "let x: u256;" should produce: let, ws, x, ws? (:), ws?, u256, ;, eof
    let kinds = lex_non_ws("let x: u256;");
    assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Let));
    assert_eq!(kinds[1], TokenKind::Ident("x".to_string()));
    assert_eq!(kinds[2], TokenKind::Colon);
    assert_eq!(
        kinds[3],
        TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(256)))
    );
    assert_eq!(kinds[4], TokenKind::Semicolon);
    assert_eq!(kinds[5], TokenKind::Eof);
}

#[test]
fn lex_fn_signature_tokens() {
    let kinds = lex_non_ws("fn foo() {}");
    assert_eq!(kinds[0], TokenKind::Keyword(Keyword::Fn));
    assert_eq!(kinds[1], TokenKind::Ident("foo".to_string()));
    assert_eq!(kinds[2], TokenKind::OpenParen);
    assert_eq!(kinds[3], TokenKind::CloseParen);
    assert_eq!(kinds[4], TokenKind::OpenBrace);
    assert_eq!(kinds[5], TokenKind::CloseBrace);
    assert_eq!(kinds[6], TokenKind::Eof);
}

#[test]
fn lex_binary_expression_tokens() {
    let kinds = lex_non_ws("x + y");
    assert_eq!(kinds[0], TokenKind::Ident("x".to_string()));
    assert_eq!(
        kinds[1],
        TokenKind::Operator(Operator::Arithmetic(ArithmeticOperator::Add))
    );
    assert_eq!(kinds[2], TokenKind::Ident("y".to_string()));
    assert_eq!(kinds[3], TokenKind::Eof);
}

#[test]
fn lex_comment_skipped_in_non_ws() {
    let kinds = lex_non_ws("// a comment\nlet");
    // Comment is not whitespace, so it should appear in non-ws
    assert!(matches!(kinds[0], TokenKind::Comment(_)));
    assert_eq!(kinds[1], TokenKind::Keyword(Keyword::Let));
}

#[test]
fn lex_assignment_operator() {
    let mut lexer = Lexer::new("=");
    let tok = lexer.next().unwrap().unwrap();
    assert_eq!(tok.kind, TokenKind::Operator(Operator::Assignment));
}

#[test]
fn lex_bool_literals() {
    // true and false are syntax sugar for literals
    let kinds = lex_non_ws("true");
    assert!(matches!(kinds[0], TokenKind::Literal(_)));

    let kinds = lex_non_ws("false");
    assert!(matches!(kinds[0], TokenKind::Literal(_)));
}
