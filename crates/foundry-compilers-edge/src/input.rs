//! Compiler input for Edge source files.

use crate::EdgeLanguage;
use crate::EdgeSettings;
use foundry_compilers::artifacts::sources::{Source, Sources};
use foundry_compilers::CompilerInput;
use semver::Version;
use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// Input for the Edge compiler, containing resolved source files and settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeCompilerInput {
    /// The sources to compile.
    pub sources: Sources,
    /// Compiler settings.
    pub settings: EdgeSettings,
    /// The compiler version.
    pub version: Version,
}

impl CompilerInput for EdgeCompilerInput {
    type Language = EdgeLanguage;
    type Settings = EdgeSettings;

    fn build(
        sources: Sources,
        settings: Self::Settings,
        _language: Self::Language,
        version: Version,
    ) -> Self {
        Self {
            sources,
            settings,
            version,
        }
    }

    fn language(&self) -> Self::Language {
        EdgeLanguage
    }

    fn version(&self) -> &Version {
        &self.version
    }

    fn sources(&self) -> impl Iterator<Item = (&Path, &Source)> {
        self.sources.0.iter().map(|(p, s)| (p.as_path(), s))
    }

    fn compiler_name(&self) -> Cow<'static, str> {
        Cow::Borrowed("Edge")
    }

    fn strip_prefix(&mut self, base: &Path) {
        let old: std::collections::BTreeMap<PathBuf, Source> =
            std::mem::take(&mut self.sources.0);
        self.sources.0 = old
            .into_iter()
            .map(|(path, source)| {
                let stripped = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_path_buf();
                (stripped, source)
            })
            .collect();
    }
}
