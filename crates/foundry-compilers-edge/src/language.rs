//! Edge language definition for the Foundry compilers framework.

use foundry_compilers::Language;
use std::fmt;

/// The Edge language identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EdgeLanguage;

impl Language for EdgeLanguage {
    const FILE_EXTENSIONS: &'static [&'static str] = &["edge"];
}

impl fmt::Display for EdgeLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Edge")
    }
}
