//! Type checking and name resolution pass
//!
//! This module implements the type checker that walks the AST, resolves type information,
//! computes storage layouts, and generates function selectors for the compiler driver.

use alloy_primitives::Selector;
use edge_ast::{
    expr::Expr,
    item::{ContractDecl, ContractFnDecl},
    lit::Lit,
    op::BinOp,
    stmt::Stmt,
    ty::{Location, PrimitiveType, TypeSig},
    Program,
};
use indexmap::IndexMap;
use tiny_keccak::{Hasher, Keccak};

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
    /// 4-byte ABI selector: keccak256("name(types...)")\[0:4\]
    pub selector: Selector,
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

        // If no contracts found, check for top-level functions and synthesize a virtual contract
        if contracts.is_empty() {
            let synthetic_contract = self.check_toplevel_functions(program)?;
            if let Some(contract_info) = synthetic_contract {
                contracts.push(contract_info);
            } else {
                return Err(crate::TypeCheckError::NoContract);
            }
        }

        Ok(CheckedProgram { contracts })
    }

    /// Extract top-level functions and synthesize a virtual contract
    fn check_toplevel_functions(
        &self,
        program: &Program,
    ) -> Result<Option<ContractInfo>, crate::TypeCheckError> {
        let mut functions = Vec::new();

        for stmt in &program.stmts {
            if let Stmt::FnAssign(fn_decl, body) = stmt {
                let name = fn_decl.name.name.clone();
                let selector = Self::compute_selector(&name, &fn_decl.params);

                let params = fn_decl
                    .params
                    .iter()
                    .map(|(ident, ty)| (ident.name.clone(), ty.clone()))
                    .collect();

                let returns = fn_decl.returns.clone();
                // Top-level functions are always publicly callable.
                functions.push(FnInfo {
                    name,
                    selector,
                    params,
                    returns,
                    is_pub: true,
                    body: Some(body.clone()),
                });
            }
        }

        if functions.is_empty() {
            return Ok(None);
        }

        Ok(Some(ContractInfo {
            name: "__module__".to_string(),
            storage: StorageLayout {
                slots: IndexMap::new(),
            },
            functions,
            consts: Vec::new(),
        }))
    }

    /// Check a single contract declaration
    fn check_contract(
        &self,
        contract: &ContractDecl,
    ) -> Result<ContractInfo, crate::TypeCheckError> {
        let name = contract.name.name.clone();

        // Build storage layout for &s fields
        let storage = self.build_storage_layout(&contract.fields);

        // Process functions
        let functions = contract
            .functions
            .iter()
            .map(|fn_decl| self.check_contract_fn(fn_decl))
            .collect();

        // Process constants (evaluate simple literal/arithmetic expressions)
        let mut const_env: IndexMap<String, u64> = IndexMap::new();
        let consts = contract
            .consts
            .iter()
            .map(|(decl, expr)| {
                let value = Self::eval_const_expr(expr, &const_env).unwrap_or(0);
                const_env.insert(decl.name.name.clone(), value);
                ConstValue {
                    name: decl.name.name.clone(),
                    value,
                }
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
            // Assign slots to &s (persistent) and &t (transient) storage fields.
            // Transient storage will use TSTORE/TLOAD once those opcodes are added
            // to the IR; for now the slot numbering is shared.
            if Self::is_stored_pointer(ty) {
                slots.insert(field_name.name.clone(), next_slot);
                next_slot += 1;
            }
        }

        StorageLayout { slots }
    }

    /// Check if a type uses contract storage (&s persistent or &t transient).
    const fn is_stored_pointer(ty: &TypeSig) -> bool {
        matches!(
            ty,
            TypeSig::Pointer(Location::Stack, _) | TypeSig::Pointer(Location::Transient, _)
        )
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

        let body = fn_decl.body.clone();
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
    fn compute_selector(name: &str, params: &[(edge_ast::Ident, TypeSig)]) -> Selector {
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

        Selector::from([output[0], output[1], output[2], output[3]])
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

    /// Evaluate a constant expression to a u64 value.
    /// Supports literals and simple arithmetic over previously-defined constants.
    fn eval_const_expr(expr: &Expr, env: &IndexMap<String, u64>) -> Option<u64> {
        match expr {
            Expr::Literal(lit) => match lit.as_ref() {
                Lit::Int(n, _, _) => Some(*n),
                Lit::Bool(b, _) => Some(if *b { 1 } else { 0 }),
                Lit::Hex(bytes, _) | Lit::Bin(bytes, _) => {
                    let mut v = 0u64;
                    for &b in bytes.iter().take(8) {
                        v = (v << 8) | (b as u64);
                    }
                    Some(v)
                }
                Lit::Str(_, _) => None,
            },
            Expr::Ident(id) => env.get(&id.name).copied(),
            Expr::Paren(inner, _) => Self::eval_const_expr(inner, env),
            Expr::Binary(lhs, op, rhs, _) => {
                let l = Self::eval_const_expr(lhs, env)?;
                let r = Self::eval_const_expr(rhs, env)?;
                match op {
                    BinOp::Add => Some(l.wrapping_add(r)),
                    BinOp::Sub => Some(l.wrapping_sub(r)),
                    BinOp::Mul => Some(l.wrapping_mul(r)),
                    BinOp::Div if r != 0 => Some(l / r),
                    BinOp::Mod if r != 0 => Some(l % r),
                    BinOp::BitwiseAnd => Some(l & r),
                    BinOp::BitwiseOr => Some(l | r),
                    BinOp::BitwiseXor => Some(l ^ r),
                    BinOp::Shl => Some(l << (r & 63)),
                    BinOp::Shr => Some(l >> (r & 63)),
                    BinOp::Eq => Some(if l == r { 1 } else { 0 }),
                    BinOp::Neq => Some(if l != r { 1 } else { 0 }),
                    BinOp::Lt => Some(if l < r { 1 } else { 0 }),
                    BinOp::Gt => Some(if l > r { 1 } else { 0 }),
                    BinOp::Lte => Some(if l <= r { 1 } else { 0 }),
                    BinOp::Gte => Some(if l >= r { 1 } else { 0 }),
                    _ => None,
                }
            }
            _ => None,
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

#[cfg(test)]
mod toplevel_tests {
    use edge_ast::{
        item::FnDecl,
        stmt::{CodeBlock, Stmt},
        ty::TypeSig,
        Ident, Program,
    };
    use edge_types::span::Span;

    use super::*;

    #[test]
    fn test_toplevel_functions() {
        // Create a simple top-level function: fn add(x: u256, y: u256) -> (u256) { return x; }
        let fn_decl = FnDecl {
            name: Ident {
                name: "add".to_string(),
                span: Span::EOF,
            },
            type_params: Vec::new(),
            params: vec![
                (
                    Ident {
                        name: "x".to_string(),
                        span: Span::EOF,
                    },
                    TypeSig::Primitive(PrimitiveType::UInt(256)),
                ),
                (
                    Ident {
                        name: "y".to_string(),
                        span: Span::EOF,
                    },
                    TypeSig::Primitive(PrimitiveType::UInt(256)),
                ),
            ],
            returns: vec![TypeSig::Primitive(PrimitiveType::UInt(256))],
            is_pub: true,
            is_ext: false,
            is_mut: false,
            span: Span::EOF,
        };

        let body = CodeBlock {
            stmts: Vec::new(),
            span: Span::EOF,
        };

        let program = Program {
            stmts: vec![Stmt::FnAssign(fn_decl, body)],
            span: Span::EOF,
        };

        let checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(
            result.is_ok(),
            "Typeck should succeed for top-level functions"
        );

        let checked = result.unwrap();
        assert_eq!(
            checked.contracts.len(),
            1,
            "Should have one synthetic contract"
        );
        assert_eq!(checked.contracts[0].name, "__module__");
        assert_eq!(checked.contracts[0].functions.len(), 1);
        assert_eq!(checked.contracts[0].functions[0].name, "add");
    }
}
