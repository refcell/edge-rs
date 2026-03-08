//! Compiler settings for the Edge language.

use std::path::PathBuf;

use foundry_compilers::{
    artifacts::output_selection::OutputSelection, CompilerSettings, CompilerSettingsRestrictions,
};

/// Settings for the Edge compiler.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EdgeSettings {
    /// Output directory for compiled artifacts.
    pub output_dir: Option<PathBuf>,
}

impl CompilerSettings for EdgeSettings {
    type Restrictions = EdgeSettingsRestrictions;

    fn update_output_selection(&mut self, _f: impl FnMut(&mut OutputSelection)) {
        // Edge does not use output selection.
    }

    fn can_use_cached(&self, other: &Self) -> bool {
        self == other
    }

    fn satisfies_restrictions(&self, _restrictions: &Self::Restrictions) -> bool {
        true
    }
}

/// Restrictions on Edge compiler settings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EdgeSettingsRestrictions;

impl CompilerSettingsRestrictions for EdgeSettingsRestrictions {
    fn merge(self, _other: Self) -> Option<Self> {
        Some(Self)
    }
}
