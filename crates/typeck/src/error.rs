//! Type checking errors

/// Errors that can occur during type checking
#[derive(Debug, thiserror::Error)]
pub enum TypeCheckError {
    /// An undefined symbol was referenced
    #[error("undefined symbol: {0}")]
    UndefinedSymbol(String),
    /// A type mismatch was encountered
    #[error("type mismatch: expected {expected}, got {got}")]
    TypeMismatch {
        /// Expected type
        expected: String,
        /// Got type
        got: String,
    },
    /// No contract found in the program
    #[error("no contract declaration found")]
    NoContract,
}
