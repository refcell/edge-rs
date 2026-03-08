//! Standard JSON I/O mode for the Edge compiler.
//!
//! Provides Solidity-compatible standard JSON input/output, allowing
//! build tools (e.g. Foundry) to drive `edgec` with a single JSON blob
//! on stdin and receive structured compilation results on stdout.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::compiler::Compiler;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Top-level standard JSON input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardJsonInput {
    /// Language identifier (expected: `"Edge"`).
    #[serde(default = "default_language")]
    pub language: String,

    /// Source files keyed by path.
    pub sources: BTreeMap<String, SourceFile>,

    /// Compiler settings (optional).
    #[serde(default)]
    pub settings: Settings,
}

/// A single source file entry.
#[derive(Debug, Deserialize)]
pub struct SourceFile {
    /// Inline source content.
    pub content: Option<String>,
}

/// Compiler settings from the standard JSON input.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Optimizer settings.
    #[serde(default)]
    pub optimizer: Optimizer,
}

/// Optimizer configuration.
#[derive(Debug, Default, Deserialize)]
pub struct Optimizer {
    /// Whether optimization is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Number of optimization runs (unused for now, kept for compat).
    #[serde(default)]
    pub runs: Option<u64>,
}

fn default_language() -> String {
    "Edge".to_string()
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Top-level standard JSON output.
#[derive(Debug, Default, Serialize)]
pub struct StandardJsonOutput {
    /// Compilation errors and warnings.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<OutputError>,

    /// Per-source compilation results.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub sources: BTreeMap<String, SourceOutput>,

    /// Per-source, per-contract outputs.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub contracts: BTreeMap<String, BTreeMap<String, ContractOutput>>,
}

/// A compilation error or warning.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputError {
    /// Error kind (e.g. `"Error"`, `"Warning"`).
    #[serde(rename = "type")]
    pub kind: String,

    /// Severity level.
    pub severity: String,

    /// Human-readable message.
    pub message: String,

    /// Formatted message with source context.
    pub formatted_message: String,
}

/// Per-source output (currently only an ID placeholder).
#[derive(Debug, Default, Serialize)]
pub struct SourceOutput {
    /// Numeric source index.
    pub id: u32,
}

/// Per-contract compilation output.
#[derive(Debug, Default, Serialize)]
pub struct ContractOutput {
    /// ABI entries.
    pub abi: Vec<serde_json::Value>,

    /// EVM-related output.
    pub evm: EvmOutput,
}

/// EVM output section.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvmOutput {
    /// Bytecode (deployment/creation code is the same as runtime for now).
    pub bytecode: BytecodeOutput,

    /// Deployed (runtime) bytecode.
    pub deployed_bytecode: BytecodeOutput,
}

/// Bytecode output with hex-encoded object.
#[derive(Debug, Default, Serialize)]
pub struct BytecodeOutput {
    /// Hex-encoded bytecode (no `0x` prefix).
    pub object: String,
}

// ---------------------------------------------------------------------------
// Compilation entry point
// ---------------------------------------------------------------------------

/// Compile one or more source files using the standard JSON interface.
///
/// Always succeeds at the Rust level; errors are reported inside the
/// returned [`StandardJsonOutput`].
pub fn compile_standard_json(input: StandardJsonInput) -> StandardJsonOutput {
    let mut output = StandardJsonOutput::default();

    for (idx, (source_path, source_file)) in input.sources.iter().enumerate() {
        // Extract source content.
        let content = match source_file.content {
            Some(ref c) => c.clone(),
            None => {
                output.errors.push(OutputError {
                    kind: "Error".into(),
                    severity: "error".into(),
                    message: format!("source file `{source_path}` has no content"),
                    formatted_message: format!("source file `{source_path}` has no content"),
                });
                continue;
            }
        };

        // Compile the source.
        let mut compiler = Compiler::from_source(content);

        // Apply optimizer settings: enabled → O1, disabled → O0.
        compiler.session_mut().config.optimization_level = if input.settings.optimizer.enabled {
            1
        } else {
            0
        };

        let result = match compiler.compile() {
            Ok(r) => r,
            Err(e) => {
                // Collect diagnostics as formatted messages.
                let diag_messages = compiler.diagnostic_messages();
                if diag_messages.is_empty() {
                    output.errors.push(OutputError {
                        kind: "Error".into(),
                        severity: "error".into(),
                        message: format!("{e}"),
                        formatted_message: format!("{e}"),
                    });
                } else {
                    for msg in diag_messages {
                        output.errors.push(OutputError {
                            kind: "Error".into(),
                            severity: "error".into(),
                            message: msg.clone(),
                            formatted_message: msg,
                        });
                    }
                }
                continue;
            }
        };

        // Record the source.
        output
            .sources
            .insert(source_path.clone(), SourceOutput { id: idx as u32 });

        // Serialize ABI entries.
        let abi_values: Vec<serde_json::Value> = result
            .abi
            .as_ref()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|e| serde_json::to_value(e).ok())
                    .collect()
            })
            .unwrap_or_default();

        // Build per-contract outputs.
        let mut contract_map: BTreeMap<String, ContractOutput> = BTreeMap::new();

        if let Some(ref bytecodes) = result.bytecodes {
            for (contract_name, bytecode) in bytecodes {
                let hex: String = bytecode.iter().map(|b| format!("{b:02x}")).collect();
                contract_map.insert(
                    contract_name.clone(),
                    ContractOutput {
                        abi: abi_values.clone(),
                        evm: EvmOutput {
                            bytecode: BytecodeOutput {
                                object: hex.clone(),
                            },
                            deployed_bytecode: BytecodeOutput { object: hex },
                        },
                    },
                );
            }
        } else if let Some(ref bytecode) = result.bytecode {
            // Single contract without a name — use the source filename stem.
            let hex: String = bytecode.iter().map(|b| format!("{b:02x}")).collect();
            let name = std::path::Path::new(source_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Contract")
                .to_string();
            contract_map.insert(
                name,
                ContractOutput {
                    abi: abi_values.clone(),
                    evm: EvmOutput {
                        bytecode: BytecodeOutput {
                            object: hex.clone(),
                        },
                        deployed_bytecode: BytecodeOutput { object: hex },
                    },
                },
            );
        }

        if !contract_map.is_empty() {
            output.contracts.insert(source_path.clone(), contract_map);
        }
    }

    output
}
