use edge_lexer::lexer::Lexer;
use edge_types::prelude::*;

#[test]
fn parses_type() {
    let mut lexer = Lexer::new("type PrimitiveTuple = (u8, u8, u8);");

    // type
    let tok = lexer.next().unwrap().unwrap();
    let type_span = Span::new(0..3, None);
    assert_eq!(
        tok,
        Token::new(TokenKind::Keyword(Keyword::Type), type_span.clone())
    );

    // Whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(4..4, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // PrimitiveTuple Ident
    let tok = lexer.next().unwrap().unwrap();
    let ident_span = Span::new(5..18, None);
    let kind = TokenKind::Ident("PrimitiveTuple".to_string());
    assert_eq!(tok, Token::new(kind, ident_span));

    // Whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(19..19, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // Equals
    let _tok = lexer.next().unwrap().unwrap();
    let _eq_span = Span::new(20..20, None);

    // Whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(21..21, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // OpenParen
    let tok = lexer.next().unwrap().unwrap();
    let op_span = Span::new(22..22, None);
    assert_eq!(tok, Token::new(TokenKind::OpenParen, op_span.clone()));

    // u8
    let tok = lexer.next().unwrap().unwrap();
    let u8_span = Span::new(23..24, None);
    let kind = TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(8)));
    assert_eq!(tok, Token::new(kind, u8_span.clone()));

    // Comma
    let tok = lexer.next().unwrap().unwrap();
    let comma_span = Span::new(25..25, None);
    assert_eq!(tok, Token::new(TokenKind::Comma, comma_span.clone()));

    // Whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(26..26, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // u8
    let tok = lexer.next().unwrap().unwrap();
    let u8_span = Span::new(27..28, None);
    let kind = TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(8)));
    assert_eq!(tok, Token::new(kind, u8_span.clone()));

    // Comma
    let tok = lexer.next().unwrap().unwrap();
    let comma_span = Span::new(29..29, None);
    assert_eq!(tok, Token::new(TokenKind::Comma, comma_span.clone()));

    // Whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(30..30, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // u8
    let tok = lexer.next().unwrap().unwrap();
    let u8_span = Span::new(31..32, None);
    let kind = TokenKind::DataType(DataType::Primitive(PrimitiveType::UInt(8)));
    assert_eq!(tok, Token::new(kind, u8_span.clone()));

    // CloseParen
    let tok = lexer.next().unwrap().unwrap();
    let cp_span = Span::new(33..33, None);
    assert_eq!(tok, Token::new(TokenKind::CloseParen, cp_span.clone()));

    // Semicolon
    let tok = lexer.next().unwrap().unwrap();
    let semi_span = Span::new(34..34, None);
    assert_eq!(tok, Token::new(TokenKind::Semicolon, semi_span.clone()));

    // EOF
    let tok = lexer.next().unwrap().unwrap();
    let eof_span = Span::new(35..35, None);
    assert_eq!(tok, Token::new(TokenKind::Eof, eof_span.clone()));

    // We covered the whole source
    assert!(lexer.eof);
}
