//! File source objects and utilities.

use crate::span::Span;
use crate::time;
use std::sync::Arc;
use uuid::Uuid;

/// Source File
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Source {
    /// File ID
    pub id: Uuid,
    /// File Path
    pub path: String,
    /// File Source
    pub source: Option<String>,
    /// Last File Access Time
    pub access: Option<time::Time>,
    /// An Ordered List of File Dependencies
    pub dependencies: Option<Vec<Arc<Source>>>,
}

/// Full File Source
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct FullFileSource<'a> {
    /// Flattened file source
    pub source: &'a str,
    /// The top level file source
    pub file: Option<Arc<Source>>,
    /// Files and their associated spans in the flattend file source
    pub spans: Vec<(Arc<Source>, Span)>,
}
