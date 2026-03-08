//! Source parser for Edge files within the Foundry compilers framework.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use foundry_compilers::{
    artifacts::sources::Sources, error::Result, resolver::Node, ParsedSource, ProjectPathsConfig,
    SourceParser,
};
use semver::VersionReq;

use crate::EdgeLanguage;

/// A parsed Edge source file.
#[derive(Debug, Clone)]
pub struct EdgeParsedSource {
    /// The contract names parsed from the source.
    pub contract_names: Vec<String>,
    /// Version requirement extracted from the source, if any.
    pub version_req: Option<VersionReq>,
}

impl ParsedSource for EdgeParsedSource {
    type Language = EdgeLanguage;

    fn parse(content: &str, _file: &Path) -> Result<Self> {
        // Extract contract names by scanning for `contract <Name>` patterns.
        let mut contract_names = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("contract ") {
                let name = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('{')
                    .trim();
                if !name.is_empty() {
                    contract_names.push(name.to_string());
                }
            }
        }

        // Extract version pragma: `// @version X.Y.Z`
        let version_req = content.lines().find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("// @version ")
                .and_then(|rest| VersionReq::parse(rest.trim()).ok())
        });

        Ok(Self {
            contract_names,
            version_req,
        })
    }

    fn version_req(&self) -> Option<&VersionReq> {
        self.version_req.as_ref()
    }

    fn contract_names(&self) -> &[String] {
        &self.contract_names
    }

    fn language(&self) -> Self::Language {
        EdgeLanguage
    }

    fn resolve_imports<C>(
        &self,
        _paths: &ProjectPathsConfig<C>,
        _include_paths: &mut BTreeSet<PathBuf>,
    ) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
}

/// The Edge source parser.
#[derive(Debug, Clone, Default)]
pub struct EdgeParser;

impl SourceParser for EdgeParser {
    type ParsedSource = EdgeParsedSource;

    fn new(_config: &ProjectPathsConfig) -> Self {
        Self
    }

    fn parse_sources(
        &mut self,
        sources: &mut Sources,
    ) -> Result<Vec<(PathBuf, Node<Self::ParsedSource>)>> {
        sources
            .0
            .iter()
            .map(|(path, source)| {
                let data = EdgeParsedSource::parse(source.as_ref(), path)?;
                Ok((path.clone(), Node::new(path.clone(), source.clone(), data)))
            })
            .collect::<Result<_>>()
    }
}
