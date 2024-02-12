use edge_lexer::core::Lexer;
use edge_types::prelude::*;

#[test]
fn parses_function_type() {
    let source = "contract EmptyContract {}";
    let flattened_source =  { source, file: None, spans: vec![] };
    let mut lexer = Lexer::new(flattened_source.source.clone());

    // contract
    let tok = lexer.next().unwrap().unwrap();
    let type_span = Span::new(0..7, None);
    assert_eq!(tok, Token::new(TokenKind::Contract, type_span.clone()));

    let _ = lexer.next(); // whitespace
    let _ = lexer.next(); // Identifier "EmptyContract"
    let _ = lexer.next(); // whitespace
    let _ = lexer.next(); // open parenthesis
    let _ = lexer.next(); // close parenthesis
    let _ = lexer.next(); // eof

    // We covered the whole source
    assert!(lexer.eof);
}
