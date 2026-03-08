//! ABI JSON types and extraction for Edge contracts.
//!
//! Produces Ethereum-compatible ABI descriptors from type-checked contract info.

use edge_ast::item::EventDecl;
use serde::Serialize;

use crate::checker::{ContractInfo, TypeChecker};

/// State mutability of a function in the ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StateMutability {
    /// Function does not read or write state
    Pure,
    /// Function reads but does not write state
    View,
    /// Function may write state (no ETH value accepted)
    NonPayable,
    /// Function accepts ETH value
    Payable,
}

/// A single parameter in an ABI function or event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiParam {
    /// Parameter name (empty string for unnamed return values)
    pub name: String,
    /// Solidity ABI type string (e.g. "uint256", "address")
    #[serde(rename = "type")]
    pub ty: String,
}

/// A single parameter in an ABI event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiEventParam {
    /// Parameter name
    pub name: String,
    /// Solidity ABI type string
    #[serde(rename = "type")]
    pub ty: String,
    /// Whether this parameter is indexed
    pub indexed: bool,
}

/// A top-level ABI entry (function or event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AbiEntry {
    /// A function entry
    Function(AbiFunctionEntry),
    /// An event entry
    Event(AbiEventEntry),
}

/// ABI descriptor for a function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiFunctionEntry {
    /// Function name
    pub name: String,
    /// Input parameters
    pub inputs: Vec<AbiParam>,
    /// Output parameters
    pub outputs: Vec<AbiParam>,
    /// State mutability
    pub state_mutability: StateMutability,
}

/// ABI descriptor for an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbiEventEntry {
    /// Event name
    pub name: String,
    /// Event parameters
    pub inputs: Vec<AbiEventParam>,
    /// Whether the event is anonymous
    pub anonymous: bool,
}

/// Extract an Ethereum-compatible ABI from a type-checked contract and top-level events.
pub fn extract_abi(contract: &ContractInfo, events: &[EventDecl]) -> Vec<AbiEntry> {
    let mut entries = Vec::new();

    // Functions: only include public functions
    for func in &contract.functions {
        if !func.is_pub {
            continue;
        }

        let inputs = func
            .params
            .iter()
            .map(|(name, ty)| AbiParam {
                name: name.clone(),
                ty: TypeChecker::type_to_abi_string(ty),
            })
            .collect();

        let outputs = func
            .returns
            .iter()
            .map(|ty| AbiParam {
                name: String::new(),
                ty: TypeChecker::type_to_abi_string(ty),
            })
            .collect();

        let state_mutability = if func.is_mut {
            StateMutability::NonPayable
        } else {
            StateMutability::View
        };

        entries.push(AbiEntry::Function(AbiFunctionEntry {
            name: func.name.clone(),
            inputs,
            outputs,
            state_mutability,
        }));
    }

    // Events
    for event in events {
        let inputs = event
            .fields
            .iter()
            .map(|field| AbiEventParam {
                name: field.name.name.clone(),
                ty: TypeChecker::type_to_abi_string(&field.ty),
                indexed: field.indexed,
            })
            .collect();

        entries.push(AbiEntry::Event(AbiEventEntry {
            name: event.name.name.clone(),
            inputs,
            anonymous: event.is_anon,
        }));
    }

    entries
}
