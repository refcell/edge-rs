//! Type checking and name resolution pass
//!
//! This module implements the type checker that walks the AST, resolves type information,
//! computes storage layouts, and generates function selectors for the compiler driver.

use indexmap::IndexMap;
use tiny_keccak::{Hasher, Keccak};

use edge_ast::{
    item::{ContractDecl, ContractFnDecl},
    stmt::Stmt,
    ty::{Location, PrimitiveType, TypeSig},
    Program,
};

/// Storage slot assignment for contract fields
#[derive(Debug, Clone)]
pub struct StorageLayout {
    /// Map from field name to storage slot number (u256 slot index)
    pub slots: IndexMap<String, u32>,
}

/// A constant value resolved at compile time
#[derive(Debug, Clone)]
pub struct ConstValue {
    /// Constant name
    pub name: String,
    /// Resolved u256 value (fits in u64 for simple literals)
    pub value: u64,
}

/// A function's type information
#[derive(Debug, Clone)]
pub struct FnInfo {
    /// Function name
    pub name: String,
    /// 4-byte ABI selector: keccak256("name(types...)")[0:4]
    pub selector: [u8; 4],
    /// Parameter types with names
    pub params: Vec<(String, TypeSig)>,
    /// Return types
    pub returns: Vec<TypeSig>,
    /// Whether the function is public (callable externally)
    pub is_pub: bool,
    /// Function body (None for ABI-only declarations)
    pub body: Option<edge_ast::stmt::CodeBlock>,
}

/// Type-checked contract information
#[derive(Debug, Clone)]
pub struct ContractInfo {
    /// Contract name
    pub name: String,
    /// Storage layout for &s fields
    pub storage: StorageLayout,
    /// All functions (public and internal)
    pub functions: Vec<FnInfo>,
    /// Resolved constants
    pub consts: Vec<ConstValue>,
}

/// Output of the type-checking pass
#[derive(Debug, Clone)]
pub struct CheckedProgram {
    /// All contracts in the program
    pub contracts: Vec<ContractInfo>,
}

/// The type checker
#[derive(Debug, Default)]
pub struct TypeChecker;

impl TypeChecker {
    /// Create a new type checker
    pub const fn new() -> Self {
        Self
    }

    /// Check a parsed program and return structured type information
    pub fn check(&self, program: &Program) -> Result<CheckedProgram, crate::TypeCheckError> {
        let mut contracts = Vec::new();

        // Find all contract declarations in the program
        for stmt in &program.stmts {
            if let Stmt::ContractDecl(contract_decl) = stmt {
                let contract_info = self.check_contract(contract_decl)?;
                contracts.push(contract_info);
            }
        }

        if contracts.is_empty() {
            return Err(crate::TypeCheckError::NoContract);
        }

        Ok(CheckedProgram { contracts })
    }

    /// Check a single contract declaration
    fn check_contract(&self, contract: &ContractDecl) -> Result<ContractInfo, crate::TypeCheckError> {
        let name = contract.name.name.clone();

        // Build storage layout for &s fields
        let storage = self.build_storage_layout(&contract.fields);

        // Process functions
        let functions = contract
            .functions
            .iter()
            .map(|fn_decl| self.check_contract_fn(fn_decl))
            .collect();

        // Process constants
        let consts = contract
            .consts
            .iter()
            .map(|(decl, _expr)| ConstValue {
                name: decl.name.name.clone(),
                value: 0, // TODO: Evaluate simple expressions
            })
            .collect();

        Ok(ContractInfo {
            name,
            storage,
            functions,
            consts,
        })
    }

    /// Build storage layout from contract fields
    fn build_storage_layout(&self, fields: &[(edge_ast::Ident, TypeSig)]) -> StorageLayout {
        let mut slots = IndexMap::new();
        let mut next_slot = 0u32;

        for (field_name, ty) in fields {
            // Only assign storage slots to &s (Stack) pointers
            if Self::is_stack_pointer(ty) {
                slots.insert(field_name.name.clone(), next_slot);
                next_slot += 1;
            }
        }

        StorageLayout { slots }
    }

    /// Check if a type is a stack pointer (&s T)
    const fn is_stack_pointer(ty: &TypeSig) -> bool {
        matches!(ty, TypeSig::Pointer(Location::Stack, _))
    }

    /// Check a single contract function
    fn check_contract_fn(&self, fn_decl: &ContractFnDecl) -> FnInfo {
        let name = fn_decl.name.name.clone();
        let selector = Self::compute_selector(&name, &fn_decl.params);

        let params = fn_decl
            .params
            .iter()
            .map(|(ident, ty)| (ident.name.clone(), ty.clone()))
            .collect();

        let returns = fn_decl.returns.clone();

        let body = Some(fn_decl.body.clone());
        let is_pub = fn_decl.is_pub;

        FnInfo {
            name,
            selector,
            params,
            returns,
            is_pub,
            body,
        }
    }

    /// Compute the 4-byte ABI selector for a function
    fn compute_selector(name: &str, params: &[(edge_ast::Ident, TypeSig)]) -> [u8; 4] {
        let param_types = params
            .iter()
            .map(|(_, ty)| Self::type_to_abi_string(ty))
            .collect::<Vec<_>>()
            .join(",");

        let sig = format!("{name}({param_types})");

        let mut hasher = Keccak::v256();
        hasher.update(sig.as_bytes());
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);

        [output[0], output[1], output[2], output[3]]
    }

    /// Convert an Edge type to its ABI type string
    fn type_to_abi_string(ty: &TypeSig) -> String {
        match ty {
            TypeSig::Primitive(prim) => Self::primitive_to_abi_string(prim),
            TypeSig::Pointer(_loc, inner) => Self::type_to_abi_string(inner),
            TypeSig::Array(inner, _) | TypeSig::PackedArray(inner, _) => {
                format!("{}[]", Self::type_to_abi_string(inner))
            }
            TypeSig::Tuple(types) | TypeSig::PackedTuple(types) => {
                let inner = types
                    .iter()
                    .map(Self::type_to_abi_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({inner})")
            }
            _ => "uint256".to_string(), // Fallback for complex types
        }
    }

    /// Convert a primitive type to its ABI string
    fn primitive_to_abi_string(prim: &PrimitiveType) -> String {
        match prim {
            PrimitiveType::UInt(n) => format!("uint{n}"),
            PrimitiveType::Int(n) => format!("int{n}"),
            PrimitiveType::FixedBytes(n) => format!("bytes{n}"),
            PrimitiveType::Address => "address".to_string(),
            PrimitiveType::Bool | PrimitiveType::Bit => "bool".to_string(), // Map bit to bool in ABI
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_to_abi_string() {
        assert_eq!(
            TypeChecker::primitive_to_abi_string(&PrimitiveType::UInt(256)),
            "uint256"
        );
        assert_eq!(
            TypeChecker::primitive_to_abi_string(&PrimitiveType::Address),
            "address"
        );
        assert_eq!(
            TypeChecker::primitive_to_abi_string(&PrimitiveType::Bool),
            "bool"
        );
    }
}
