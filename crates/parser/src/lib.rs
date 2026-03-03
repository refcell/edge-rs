#![doc = include_str!("../README.md")]
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all,
)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod errors;
pub mod parser;

pub use errors::{ParseError, ParseResult};
pub use parser::Parser;

/// Parse source code into an AST
pub fn parse(source: &str) -> ParseResult<edge_ast::Program> {
    let mut parser = Parser::new(source)?;
    parser.parse()
}
