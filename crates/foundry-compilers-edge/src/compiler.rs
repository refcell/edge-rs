//! The Edge compiler implementation for the Foundry compilers framework.

use crate::EdgeCompilationError;
use crate::EdgeCompilerContract;
use crate::EdgeCompilerInput;
use crate::EdgeLanguage;
use crate::EdgeParser;
use crate::EdgeSettings;
use foundry_compilers::artifacts::sources::Source;
use foundry_compilers::artifacts::{BytecodeObject, SourceFile};
use foundry_compilers::compilers::CompilerOutput;
use foundry_compilers::error::Result;
use foundry_compilers::{Compiler, CompilerInput, CompilerVersion};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The Edge compiler, implementing the Foundry [`Compiler`] trait.
///
/// This invokes the Edge compilation pipeline in-process via
/// [`edge_driver::standard_json::compile_standard_json`].
#[derive(Debug, Clone)]
pub struct EdgeCompiler {
    /// The version reported by this compiler instance.
    version: semver::Version,
}

impl Default for EdgeCompiler {
    fn default() -> Self {
        Self {
            version: semver::Version::new(0, 1, 0),
        }
    }
}

impl EdgeCompiler {
    /// Create a new `EdgeCompiler` with the given version.
    pub const fn new(version: semver::Version) -> Self {
        Self { version }
    }
}

impl Compiler for EdgeCompiler {
    type Input = EdgeCompilerInput;
    type CompilationError = EdgeCompilationError;
    type CompilerContract = EdgeCompilerContract;
    type Parser = EdgeParser;
    type Settings = EdgeSettings;
    type Language = EdgeLanguage;

    fn compile(
        &self,
        input: &Self::Input,
    ) -> Result<CompilerOutput<Self::CompilationError, Self::CompilerContract>> {
        // Build a StandardJsonInput from EdgeCompilerInput.
        let sources: BTreeMap<String, edge_driver::standard_json::SourceFile> =
            CompilerInput::sources(input)
                .map(|(path, source): (&std::path::Path, &Source)| {
                    let path_str = path.to_string_lossy().into_owned();
                    let content = Some(AsRef::<str>::as_ref(source).to_string());
                    (
                        path_str,
                        edge_driver::standard_json::SourceFile { content },
                    )
                })
                .collect();

        let std_input = edge_driver::standard_json::StandardJsonInput {
            language: "Edge".to_string(),
            sources,
            settings: edge_driver::standard_json::Settings::default(),
        };

        let std_output = edge_driver::standard_json::compile_standard_json(std_input);

        // Convert errors.
        let errors: Vec<EdgeCompilationError> = std_output
            .errors
            .iter()
            .map(|e| EdgeCompilationError {
                message: e.formatted_message.clone(),
                is_warning: e.severity == "warning",
            })
            .collect();

        // Convert contracts: FileToContractsMap<C> = BTreeMap<PathBuf, BTreeMap<String, C>>
        let mut contracts: BTreeMap<PathBuf, BTreeMap<String, EdgeCompilerContract>> =
            BTreeMap::new();
        for (source_path, contract_map) in &std_output.contracts {
            let mut file_contracts = BTreeMap::new();
            for (contract_name, contract_output) in contract_map {
                // Parse ABI from serde_json::Value array into JsonAbi.
                let abi = serde_json::to_string(&contract_output.abi)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok());

                // Convert bytecode hex strings to BytecodeObject.
                let bytecode = hex_to_bytecode(&contract_output.evm.bytecode.object);
                let runtime_bytecode =
                    hex_to_bytecode(&contract_output.evm.deployed_bytecode.object);

                file_contracts.insert(
                    contract_name.clone(),
                    EdgeCompilerContract {
                        abi,
                        bytecode,
                        runtime_bytecode,
                    },
                );
            }
            contracts.insert(PathBuf::from(source_path), file_contracts);
        }

        // Convert sources.
        let output_sources: BTreeMap<PathBuf, SourceFile> = std_output
            .sources
            .iter()
            .map(|(path, src)| {
                (
                    PathBuf::from(path),
                    SourceFile {
                        id: src.id,
                        ast: None,
                    },
                )
            })
            .collect();

        Ok(CompilerOutput {
            errors,
            contracts,
            sources: output_sources,
            metadata: BTreeMap::new(),
        })
    }

    fn available_versions(&self, _language: &Self::Language) -> Vec<CompilerVersion> {
        vec![CompilerVersion::Installed(self.version.clone())]
    }
}

/// Convert a hex string (without `0x` prefix) into a [`BytecodeObject`].
///
/// Returns `None` if the hex string is empty.
fn hex_to_bytecode(hex: &str) -> Option<BytecodeObject> {
    if hex.is_empty() {
        return None;
    }
    let bytes = alloy_primitives::hex::decode(hex).ok()?;
    Some(BytecodeObject::Bytecode(bytes.into()))
}
