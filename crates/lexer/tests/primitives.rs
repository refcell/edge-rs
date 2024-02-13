use edge_lexer::lexer::Lexer;
use edge_types::prelude::*;

#[test]
fn parses_empty_contract() {
    let mut lexer = Lexer::new("contract EmptyContract {}");

    // contract
    let tok = lexer.next().unwrap().unwrap();
    let type_span = Span::new(0..7, None);
    assert_eq!(
        tok,
        Token::new(TokenKind::Keyword(Keyword::Contract), type_span.clone())
    );

    // whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(8..8, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // Identifier "EmptyContract"
    let tok = lexer.next().unwrap().unwrap();
    let id_span = Span::new(9..21, None);
    assert_eq!(
        tok,
        Token::new(
            TokenKind::Ident("EmptyContract".to_string()),
            id_span.clone()
        )
    );

    // whitespace
    let tok = lexer.next().unwrap().unwrap();
    let ws_span = Span::new(22..22, None);
    assert_eq!(tok, Token::new(TokenKind::Whitespace, ws_span.clone()));

    // open brace
    let tok = lexer.next().unwrap().unwrap();
    let open_brace_span = Span::new(23..23, None);
    assert_eq!(
        tok,
        Token::new(TokenKind::OpenBrace, open_brace_span.clone())
    );

    // close brace
    let tok = lexer.next().unwrap().unwrap();
    let close_brace_span = Span::new(24..24, None);
    assert_eq!(
        tok,
        Token::new(TokenKind::CloseBrace, close_brace_span.clone())
    );

    // eof
    let tok = lexer.next().unwrap().unwrap();
    let eof_span = Span::new(25..25, None);
    assert_eq!(tok, Token::new(TokenKind::Eof, eof_span.clone()));

    // We covered the whole source
    assert!(lexer.eof);
}
