//! AST to egglog IR lowering.
//!
//! Converts `edge_ast::Program` into `EvmProgram` by walking the AST
//! and producing IR nodes. This follows the pattern from eggcc's
//! `TreeToEgglog` but targets EVM-specific IR constructs.

mod calls;
mod composite;
mod control_flow;
mod dispatch;
mod expr;
mod function;
mod pattern;
mod storage;
mod types;

use std::{collections::HashSet, rc::Rc};

use indexmap::IndexMap;

use crate::{
    ast_helpers,
    schema::{
        DataLocation, EvmBaseType, EvmContext, EvmContract, EvmExpr, EvmProgram, EvmType, RcExpr,
    },
    IrError,
};

/// Check if an expression references any variable from a set of names.
///
/// Used during lowering to ensure a `LetBind` init expression doesn't reference
/// variables whose `LetBinds` are inner (not yet allocated).
pub(crate) fn references_any_var(expr: &RcExpr, names: &HashSet<&str>) -> bool {
    match expr.as_ref() {
        EvmExpr::Var(n) => names.contains(n.as_str()),
        EvmExpr::Const(..)
        | EvmExpr::Arg(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Selector(_)
        | EvmExpr::StorageField(..)
        | EvmExpr::Drop(_) => false,
        EvmExpr::InlineAsm(inputs, _, _) => inputs.iter().any(|inp| references_any_var(inp, names)),
        EvmExpr::Bop(_, a, b) | EvmExpr::Concat(a, b) | EvmExpr::DoWhile(a, b) => {
            references_any_var(a, names) || references_any_var(b, names)
        }
        EvmExpr::Uop(_, a) | EvmExpr::Get(a, _) => references_any_var(a, names),
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            references_any_var(a, names)
                || references_any_var(b, names)
                || references_any_var(c, names)
        }
        EvmExpr::If(c, i, t, e) => {
            references_any_var(c, names)
                || references_any_var(i, names)
                || references_any_var(t, names)
                || references_any_var(e, names)
        }
        EvmExpr::VarStore(_, v) => references_any_var(v, names),
        EvmExpr::LetBind(_, init, body) => {
            references_any_var(init, names) || references_any_var(body, names)
        }
        EvmExpr::EnvRead(_, s) => references_any_var(s, names),
        EvmExpr::EnvRead1(_, a, s) => references_any_var(a, names) || references_any_var(s, names),
        EvmExpr::Log(_, topics, data_offset, data_size, state) => {
            topics.iter().any(|t| references_any_var(t, names))
                || references_any_var(data_offset, names)
                || references_any_var(data_size, names)
                || references_any_var(state, names)
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => [a, b, c, d, e, f, g]
            .iter()
            .any(|x| references_any_var(x, names)),
        EvmExpr::Call(_, args) => args.iter().any(|a| references_any_var(a, names)),
        EvmExpr::Function(_, _, _, body) => references_any_var(body, names),
    }
}

/// Tracks a variable binding during lowering.
#[derive(Debug, Clone)]
pub(crate) struct VarBinding {
    /// The current value expression (for storage/transient: the IR tree; for memory-backed: ignored)
    pub value: RcExpr,
    /// Where this variable lives
    pub location: DataLocation,
    /// For storage variables, the slot index
    pub storage_slot: Option<usize>,
    /// The type
    pub _ty: EvmType,
    /// For memory-backed local variables, the `LetBind` variable name
    pub let_bind_name: Option<String>,
    /// For struct/array-typed variables: the type name (for field/index lookup)
    pub composite_type: Option<String>,
    /// For struct/array-typed variables: the memory base offset
    pub composite_base: Option<usize>,
}

/// Scope for variable resolution during lowering.
#[derive(Debug, Clone)]
pub(crate) struct Scope {
    /// Variable bindings: name -> binding
    pub bindings: IndexMap<String, VarBinding>,
}

impl Scope {
    pub(crate) fn new() -> Self {
        Self {
            bindings: IndexMap::new(),
        }
    }
}

/// A contract function: (name, params, body).
pub(crate) type ContractFunction = (
    String,
    Vec<(String, edge_ast::ty::TypeSig)>,
    edge_ast::CodeBlock,
);

/// A free/comptime function with metadata.
#[derive(Debug, Clone)]
pub(crate) struct FreeFnInfo {
    pub name: String,
    pub params: Vec<(String, edge_ast::ty::TypeSig)>,
    pub returns: Vec<edge_ast::ty::TypeSig>,
    pub body: edge_ast::CodeBlock,
    pub is_comptime: bool,
    pub type_params: Vec<edge_ast::ty::TypeParam>,
}

/// Info about a generic type template (struct/union with type params).
#[derive(Debug, Clone)]
pub(crate) struct GenericTypeTemplate {
    pub type_params: Vec<String>,
    pub type_sig: edge_ast::ty::TypeSig,
}

/// Packed layout for a single field within a packed struct.
#[derive(Debug, Clone)]
pub(crate) struct PackedFieldLayout {
    /// Bit offset from LSB within the word
    pub bit_offset: u16,
    /// Bit width of this field
    pub bit_width: u16,
    /// Which 256-bit word this field lives in (0 for single-word packed structs)
    pub word_index: usize,
}

/// Layout information for a packed struct.
#[derive(Debug, Clone)]
pub(crate) struct PackedLayout {
    /// Total bits used across all fields
    pub _total_bits: u16,
    /// Number of 256-bit words needed
    pub word_count: usize,
    /// Per-field layout information (same order as fields vec)
    pub field_layouts: Vec<PackedFieldLayout>,
}

/// Extended struct type info that tracks packed-ness.
#[derive(Debug, Clone)]
pub(crate) struct StructTypeInfo {
    /// Field definitions: (name, type)
    pub fields: Vec<(String, EvmType)>,
    /// Whether this is a packed struct
    pub is_packed: bool,
    /// Layout info for packed structs
    pub packed_layout: Option<PackedLayout>,
}

impl StructTypeInfo {
    /// Create an unpacked struct type info.
    pub(crate) const fn unpacked(fields: Vec<(String, EvmType)>) -> Self {
        Self {
            fields,
            is_packed: false,
            packed_layout: None,
        }
    }

    /// Create a packed struct type info, computing layout from field types.
    pub(crate) fn packed(fields: Vec<(String, EvmType)>) -> Self {
        let mut field_layouts = Vec::with_capacity(fields.len());

        // Compute total bits first (for MSB-first layout)
        let total_bits: u16 = fields
            .iter()
            .map(|(_, ty)| match ty {
                EvmType::Base(b) => b.bit_width(),
                _ => 256,
            })
            .sum();

        // Fields are packed MSB-first: first field at highest bits, last at lowest.
        // We compute bit_offset from LSB for each field.
        let mut remaining_bits = total_bits;
        for (_, ty) in &fields {
            let width = match ty {
                EvmType::Base(b) => b.bit_width(),
                _ => 256,
            };
            remaining_bits -= width;
            field_layouts.push(PackedFieldLayout {
                bit_offset: remaining_bits,
                bit_width: width,
                word_index: 0, // TODO: multi-word support
            });
        }

        let word_count = (total_bits as usize).div_ceil(256);

        Self {
            fields,
            is_packed: true,
            packed_layout: Some(PackedLayout {
                _total_bits: total_bits,
                word_count: word_count.max(1),
                field_layouts,
            }),
        }
    }
}

/// Trait definition info.
#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    pub _type_params: Vec<edge_ast::ty::TypeParam>,
    pub _supertraits: Vec<String>,
    /// Required methods (no default body)
    pub required_methods: Vec<(String, edge_ast::item::FnDecl)>,
    /// Default methods (have a body)
    pub _default_methods: Vec<(String, edge_ast::item::FnDecl, edge_ast::CodeBlock)>,
}

/// Trait implementation info.
#[derive(Debug, Clone)]
pub(crate) struct TraitImplInfo {
    pub _type_params: Vec<edge_ast::ty::TypeParam>,
    pub methods: IndexMap<String, (edge_ast::item::FnDecl, edge_ast::CodeBlock)>,
}

/// An inherent method (from impl block without trait).
#[derive(Debug, Clone)]
pub(crate) struct InherentMethod {
    pub fn_decl: edge_ast::item::FnDecl,
    pub body: edge_ast::CodeBlock,
}

/// Converts Edge AST to the egglog-based EVM IR.
#[derive(Debug)]
pub struct AstToEgglog {
    /// Scope stack (innermost last)
    pub(crate) scopes: Vec<Scope>,
    /// Current state expression (for threading side effects)
    pub(crate) current_state: RcExpr,
    /// Current context
    pub(crate) current_ctx: EvmContext,
    /// Persistent storage slot counter for the current contract
    pub(crate) next_storage_slot: usize,
    /// Transient storage slot counter for the current contract
    pub(crate) next_transient_slot: usize,
    /// Storage field IR nodes for the current contract
    pub(crate) storage_fields: Vec<RcExpr>,
    /// Internal functions available for inlining in the current contract
    /// Maps function name -> (`fn_decl` ref data, body)
    pub(crate) contract_functions: Vec<ContractFunction>,
    /// Free functions and comptime functions available for calling/inlining
    pub(crate) free_fn_bodies: Vec<FreeFnInfo>,
    /// Lowered Function IR nodes to attach to the contract runtime
    pub(crate) lowered_functions: Vec<RcExpr>,
    /// Events declared in the program (name -> (params with indexed info and type))
    pub(crate) events: IndexMap<String, Vec<(String, bool, edge_ast::ty::TypeSig)>>,
    /// Inline call depth — when > 0, `return` produces just the value (no RETURN opcode)
    pub(crate) inline_depth: usize,
    /// Counter for generating unique variable names during inlining
    pub(crate) inline_counter: usize,
    /// Prefix for variable names when inlining (empty at top level)
    pub(crate) inline_prefix: String,
    /// Union/enum type declarations: `type_name` -> `[(variant_name, has_data)]`
    /// Variant index is its position in the vector.
    pub(crate) union_types: IndexMap<String, Vec<(String, bool)>>,
    /// Struct type declarations: `type_name` -> struct type info (fields, packed layout)
    /// Field index is its position in the fields vector.
    pub(crate) struct_types: IndexMap<String, StructTypeInfo>,
    /// Type aliases: name -> `TypeSig` (for resolving named types like `FiveInts`)
    pub(crate) type_aliases: IndexMap<String, edge_ast::ty::TypeSig>,
    /// Storage array fields: `field_name` -> `(base_slot, array_length)`
    pub(crate) storage_array_fields: IndexMap<String, (usize, usize)>,
    /// Next available memory offset for composite value allocation (structs, arrays, data-unions).
    /// Starts at 128 to avoid conflict with mapping keccak scratch space (0..128).
    pub(crate) next_memory_offset: usize,
    /// Tracks the last composite allocation `(type_name, base_offset)` for wiring
    /// struct/array assignments to variable bindings.
    pub(crate) last_composite_alloc: Option<(String, usize)>,
    /// Module prefixes from `use std::math`-style whole-module imports.
    /// When set, `math::mul_div_down` resolves to `mul_div_down`.
    pub(crate) module_prefixes: HashSet<String>,

    // ---- Generics & Traits ----
    /// Generic type templates: name -> template info (type params + original `TypeSig`)
    pub(crate) generic_type_templates: IndexMap<String, GenericTypeTemplate>,
    /// Cache of monomorphized types: (`generic_name`, `concrete_types`) -> `mangled_name`
    pub(crate) monomorphized_types: IndexMap<(String, Vec<EvmType>), String>,
    /// Generic function templates: name -> `FreeFnInfo` (with `type_params`)
    pub(crate) generic_fn_templates: IndexMap<String, FreeFnInfo>,
    /// Cache of monomorphized function bodies: `mangled_name` -> `FreeFnInfo`
    pub(crate) monomorphized_fns: IndexMap<String, FreeFnInfo>,
    /// Trait definitions: `trait_name` -> `TraitInfo`
    pub(crate) trait_registry: IndexMap<String, TraitInfo>,
    /// Trait implementations: (`type_name`, `trait_name`) -> `TraitImplInfo`
    pub(crate) trait_impls: IndexMap<(String, String), TraitImplInfo>,
    /// Inherent methods: `type_name` -> [methods]
    pub(crate) inherent_methods: IndexMap<String, Vec<InherentMethod>>,
    /// Current Self type (set when inside an impl block)
    pub(crate) _self_type: Option<String>,
    /// Operator traits imported from `std::ops` (e.g., "Add", "Sub").
    /// Only these traits are eligible for operator overloading dispatch.
    pub(crate) std_ops_traits: HashSet<String>,
    /// Type hint from assignment target, used for generic return-type inference.
    /// Set before lowering the RHS of a typed variable assignment, cleared after.
    pub(crate) type_hint: Option<EvmType>,
}

impl Default for AstToEgglog {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToEgglog {
    /// Create a new lowering context.
    pub fn new() -> Self {
        let dummy_ctx = EvmContext::InFunction("__init__".to_owned());
        Self {
            scopes: vec![Scope::new()],
            current_state: Rc::new(EvmExpr::Arg(
                EvmType::Base(EvmBaseType::StateT),
                dummy_ctx.clone(),
            )),
            current_ctx: dummy_ctx,
            next_storage_slot: 0,
            next_transient_slot: 0,
            storage_fields: Vec::new(),
            contract_functions: Vec::new(),
            free_fn_bodies: Vec::new(),
            lowered_functions: Vec::new(),
            events: IndexMap::new(),
            inline_depth: 0,
            inline_counter: 0,
            inline_prefix: String::new(),
            union_types: IndexMap::new(),
            struct_types: IndexMap::new(),
            type_aliases: IndexMap::new(),
            storage_array_fields: IndexMap::new(),
            next_memory_offset: 128,
            last_composite_alloc: None,
            module_prefixes: HashSet::new(),
            generic_type_templates: IndexMap::new(),
            monomorphized_types: IndexMap::new(),
            generic_fn_templates: IndexMap::new(),
            monomorphized_fns: IndexMap::new(),
            trait_registry: IndexMap::new(),
            trait_impls: IndexMap::new(),
            inherent_methods: IndexMap::new(),
            _self_type: None,
            std_ops_traits: HashSet::new(),
            type_hint: None,
        }
    }

    /// Lower an entire program.
    pub fn lower_program(&mut self, program: &edge_ast::Program) -> Result<EvmProgram, IrError> {
        let mut contracts = Vec::new();
        let mut free_functions = Vec::new();

        // Collect module prefixes from `use std::math`-style imports (whole module).
        // These allow `math::mul_div_down(...)` to resolve to `mul_div_down`.
        for stmt in &program.stmts {
            if let edge_ast::Stmt::ModuleImport(import) = stmt {
                // A whole-module import has no path (e.g., `use std::math;`)
                // or imports all (`use std::math::*;`).
                // In these cases, the last segment is the module name prefix.
                let is_whole_module =
                    matches!(&import.path, None | Some(edge_ast::ImportPath::All));
                if is_whole_module {
                    // The module prefix is the last segment.
                    // For `use std::math;`, segments=["math"], path=None → prefix="math"
                    // For `use std::tokens::erc20;`, segments=["tokens","erc20"], path=None → prefix="erc20"
                    let prefix = if !import.segments.is_empty() {
                        import.segments.last().unwrap().name.clone()
                    } else {
                        import.root.name.clone()
                    };
                    self.module_prefixes.insert(prefix);
                }
            }
        }

        // Detect `use std::ops::X;` imports to register operator traits.
        // Only traits imported from std::ops are eligible for operator overloading.
        let known_ops = ["Add", "Sub", "Mul", "Div", "Mod", "Eq", "Ord"];
        for stmt in &program.stmts {
            if let edge_ast::Stmt::ModuleImport(import) = stmt {
                if import.root.name == "std" {
                    let is_ops_module =
                        import.segments.len() == 1 && import.segments[0].name == "ops";
                    if is_ops_module {
                        match &import.path {
                            // `use std::ops::Add;` — single symbol
                            Some(edge_ast::ImportPath::Ident(ident)) => {
                                if known_ops.contains(&ident.name.as_str()) {
                                    self.std_ops_traits.insert(ident.name.clone());
                                }
                            }
                            // `use std::ops::*;` or `use std::ops;` — all ops
                            None | Some(edge_ast::ImportPath::All) => {
                                for name in &known_ops {
                                    self.std_ops_traits.insert((*name).to_string());
                                }
                            }
                            // `use std::ops::{Add, Sub};` — nested
                            Some(edge_ast::ImportPath::Nested(items)) => {
                                for item in items {
                                    if let edge_ast::ImportPath::Ident(ident) = item {
                                        if known_ops.contains(&ident.name.as_str()) {
                                            self.std_ops_traits.insert(ident.name.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // First pass: collect event declarations and free/comptime function bodies.
        // Free/comptime functions must be collected before const evaluation
        // because constants may call them (e.g. `const BASE_FEE = base_fee()`).
        for stmt in &program.stmts {
            match stmt {
                edge_ast::Stmt::EventDecl(event) => {
                    let params = event
                        .fields
                        .iter()
                        .map(|f| (f.name.name.clone(), f.indexed, f.ty.clone()))
                        .collect();
                    self.events.insert(event.name.name.clone(), params);
                }
                edge_ast::Stmt::FnAssign(fn_decl, body) => {
                    let params = fn_decl
                        .params
                        .iter()
                        .map(|(id, ty)| (id.name.clone(), ty.clone()))
                        .collect();
                    let info = FreeFnInfo {
                        name: fn_decl.name.name.clone(),
                        params,
                        returns: fn_decl.returns.clone(),
                        body: body.clone(),
                        is_comptime: false,
                        type_params: fn_decl.type_params.clone(),
                    };
                    if fn_decl.type_params.is_empty() {
                        self.free_fn_bodies.push(info);
                    } else {
                        // Generic function — store as template, don't add to free_fn_bodies yet
                        self.generic_fn_templates
                            .insert(fn_decl.name.name.clone(), info);
                    }
                }
                edge_ast::Stmt::ComptimeFn(fn_decl, body) => {
                    let params = fn_decl
                        .params
                        .iter()
                        .map(|(id, ty)| (id.name.clone(), ty.clone()))
                        .collect();
                    self.free_fn_bodies.push(FreeFnInfo {
                        name: fn_decl.name.name.clone(),
                        params,
                        returns: fn_decl.returns.clone(),
                        body: body.clone(),
                        is_comptime: true,
                        type_params: fn_decl.type_params.clone(),
                    });
                }
                _ => {}
            }
        }

        // Second pass: collect top-level constants into scope
        // so they're visible to free functions and contracts
        for stmt in &program.stmts {
            if let edge_ast::Stmt::ConstAssign(const_decl, expr, _span) = stmt {
                let val = self.lower_expr(expr)?;
                let ty = const_decl
                    .ty
                    .as_ref()
                    .map(|ts| self.lower_type_sig(ts))
                    .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
                let binding = VarBinding {
                    value: Rc::clone(&val),
                    location: DataLocation::Stack,
                    storage_slot: None,
                    _ty: ty,
                    let_bind_name: None,
                    composite_type: None,
                    composite_base: None,
                };
                self.scopes
                    .last_mut()
                    .expect("scope stack empty")
                    .bindings
                    .insert(const_decl.name.name.clone(), binding);
            }
        }

        // Third pass: collect type aliases and union/enum/struct type declarations
        for stmt in &program.stmts {
            if let edge_ast::Stmt::TypeAssign(_type_decl, type_sig, _span) = stmt {
                // Store ALL type aliases for resolution (Named → concrete type)
                self.type_aliases
                    .insert(_type_decl.name.name.clone(), type_sig.clone());

                // If the type has type params, store as generic template
                if !_type_decl.type_params.is_empty() {
                    let template = GenericTypeTemplate {
                        type_params: _type_decl
                            .type_params
                            .iter()
                            .map(|tp| tp.name.name.clone())
                            .collect(),
                        type_sig: type_sig.clone(),
                    };
                    self.generic_type_templates
                        .insert(_type_decl.name.name.clone(), template);
                    // Don't register in struct_types/union_types yet — will be monomorphized on use
                    continue;
                }

                if let edge_ast::ty::TypeSig::Union(members) = type_sig {
                    let variants: Vec<(String, bool)> = members
                        .iter()
                        .map(|m| (m.name.name.clone(), m.inner.is_some()))
                        .collect();
                    self.union_types
                        .insert(_type_decl.name.name.clone(), variants);
                } else if let edge_ast::ty::TypeSig::Struct(fields) = type_sig {
                    let field_info: Vec<(String, EvmType)> = fields
                        .iter()
                        .map(|f| (f.name.name.clone(), self.lower_type_sig(&f.ty)))
                        .collect();
                    self.struct_types.insert(
                        _type_decl.name.name.clone(),
                        StructTypeInfo::unpacked(field_info),
                    );
                } else if let edge_ast::ty::TypeSig::PackedStruct(fields) = type_sig {
                    let field_info: Vec<(String, EvmType)> = fields
                        .iter()
                        .map(|f| (f.name.name.clone(), self.lower_type_sig(&f.ty)))
                        .collect();
                    self.struct_types.insert(
                        _type_decl.name.name.clone(),
                        StructTypeInfo::packed(field_info),
                    );
                }
            }
        }

        // Fourth pass: collect trait declarations and impl blocks
        for stmt in &program.stmts {
            match stmt {
                edge_ast::Stmt::TraitDecl(decl, _span) => {
                    let mut required_methods = Vec::new();
                    let mut default_methods = Vec::new();
                    for item in &decl.items {
                        match item {
                            edge_ast::item::TraitItem::FnDecl(fn_decl) => {
                                required_methods.push((fn_decl.name.name.clone(), fn_decl.clone()));
                            }
                            edge_ast::item::TraitItem::FnAssign(fn_decl, body) => {
                                default_methods.push((
                                    fn_decl.name.name.clone(),
                                    fn_decl.clone(),
                                    body.clone(),
                                ));
                            }
                            _ => {}
                        }
                    }
                    self.trait_registry.insert(
                        decl.name.name.clone(),
                        TraitInfo {
                            _type_params: decl.type_params.clone(),
                            _supertraits: decl.supertraits.iter().map(|s| s.name.clone()).collect(),
                            required_methods,
                            _default_methods: default_methods,
                        },
                    );
                }
                edge_ast::Stmt::ImplBlock(impl_block) => {
                    let type_name = impl_block.ty_name.name.clone();
                    if let Some((ref trait_name, _)) = impl_block.trait_impl {
                        // Trait impl — collect methods and validate against trait definition
                        let mut methods = IndexMap::new();
                        for item in &impl_block.items {
                            if let edge_ast::item::ImplItem::FnAssign(fn_decl, body) = item {
                                methods.insert(
                                    fn_decl.name.name.clone(),
                                    (fn_decl.clone(), body.clone()),
                                );
                            }
                        }

                        // Validate: all required trait methods must be provided
                        if let Some(trait_info) = self.trait_registry.get(&trait_name.name) {
                            let missing: Vec<&(String, edge_ast::item::FnDecl)> = trait_info
                                .required_methods
                                .iter()
                                .filter(|(name, _)| !methods.contains_key(name))
                                .collect();
                            if !missing.is_empty() {
                                let missing_names: Vec<&str> =
                                    missing.iter().map(|(n, _)| n.as_str()).collect();
                                let mut diag = edge_diagnostics::Diagnostic::error(format!(
                                    "not all trait items implemented, missing: `{}`",
                                    missing_names.join("`, `"),
                                ));
                                // Add trait method labels first (earlier in file)
                                for (name, fn_decl) in &missing {
                                    diag = diag.with_help_label(
                                        fn_decl.span.clone(),
                                        format!("`{name}` from trait"),
                                    );
                                }
                                // Then the impl block label (the primary error site)
                                diag = diag.with_label(
                                    impl_block.span.clone(),
                                    format!(
                                        "missing `{}` in implementation",
                                        missing_names.join("`, `"),
                                    ),
                                );
                                return Err(IrError::Diagnostic(diag));
                            }
                        }

                        self.trait_impls.insert(
                            (type_name, trait_name.name.clone()),
                            TraitImplInfo {
                                _type_params: impl_block.type_params.clone(),
                                methods,
                            },
                        );
                    } else {
                        // Inherent impl
                        let methods: Vec<InherentMethod> = impl_block
                            .items
                            .iter()
                            .filter_map(|item| {
                                if let edge_ast::item::ImplItem::FnAssign(fn_decl, body) = item {
                                    Some(InherentMethod {
                                        fn_decl: fn_decl.clone(),
                                        body: body.clone(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();
                        self.inherent_methods
                            .entry(type_name)
                            .or_default()
                            .extend(methods);
                    }
                }
                _ => {}
            }
        }

        // Fifth pass: eagerly monomorphize generic types used with concrete type args
        // anywhere in the program (function params, return types, variable decls, etc.)
        self.monomorphize_all_type_usages(program)?;

        // Save top-level const bindings to inject into each contract scope
        let toplevel_consts: IndexMap<String, VarBinding> = self
            .scopes
            .last()
            .map(|s| s.bindings.clone())
            .unwrap_or_default();

        // Collect free function declarations for potential synthetic contract
        let mut fn_stmts: Vec<(&edge_ast::FnDecl, &edge_ast::CodeBlock)> = Vec::new();

        for stmt in &program.stmts {
            match stmt {
                edge_ast::Stmt::ContractDecl(contract) => {
                    let ir_contract = self.lower_contract(contract, &toplevel_consts)?;
                    contracts.push(ir_contract);
                }
                edge_ast::Stmt::FnAssign(fn_decl, body) => {
                    fn_stmts.push((fn_decl, body));
                }
                // Skip other top-level items (type aliases, traits, events already collected, consts already collected)
                _ => {}
            }
        }

        // If there are free functions but no contracts, wrap them in a synthetic contract
        // so the dispatcher/deployment pipeline can generate deployable bytecode.
        if contracts.is_empty() && !fn_stmts.is_empty() {
            let synthetic = self.create_synthetic_contract(&fn_stmts, &toplevel_consts)?;
            contracts.push(synthetic);
        } else {
            // Otherwise, lower free functions standalone
            for (fn_decl, body) in &fn_stmts {
                let ir_fn = self.lower_function(fn_decl, body)?;
                free_functions.push(ir_fn);
            }
        }

        Ok(EvmProgram {
            contracts,
            free_functions,
        })
    }

    /// Lower a contract declaration.
    fn lower_contract(
        &mut self,
        contract: &edge_ast::ContractDecl,
        toplevel_consts: &IndexMap<String, VarBinding>,
    ) -> Result<EvmContract, IrError> {
        // Reset storage layout for this contract
        self.next_storage_slot = 0;
        self.next_transient_slot = 0;
        self.storage_fields.clear();
        // Start with a fresh scope, inheriting top-level consts
        let mut base_scope = Scope::new();
        for (name, binding) in toplevel_consts {
            base_scope.bindings.insert(name.clone(), binding.clone());
        }
        self.scopes = vec![base_scope];

        let contract_name = contract.name.name.clone();

        // Assign storage slots to fields
        for (ident, type_sig) in &contract.fields {
            let location = Self::extract_data_location(type_sig);

            // Check if this is a fixed-size array type (allocate N slots)
            let resolved_ty = self.resolve_type_alias(type_sig).clone();
            let array_len = match &resolved_ty {
                edge_ast::ty::TypeSig::Array(_, len_expr)
                | edge_ast::ty::TypeSig::PackedArray(_, len_expr) => {
                    Self::extract_array_length(len_expr)
                }
                edge_ast::ty::TypeSig::Pointer(_, inner) => {
                    let inner_resolved = self.resolve_type_alias(inner).clone();
                    match &inner_resolved {
                        edge_ast::ty::TypeSig::Array(_, len_expr)
                        | edge_ast::ty::TypeSig::PackedArray(_, len_expr) => {
                            Self::extract_array_length(len_expr)
                        }
                        _ => None,
                    }
                }
                _ => None,
            };

            let slot_count = array_len.unwrap_or(1);
            let slot = match location {
                DataLocation::Transient => {
                    let s = self.next_transient_slot;
                    self.next_transient_slot += slot_count;
                    s
                }
                _ => {
                    let s = self.next_storage_slot;
                    self.next_storage_slot += slot_count;
                    s
                }
            };
            let ty = self.lower_type_sig(type_sig);

            // Register storage array fields for direct slot-based access
            if let Some(n) = array_len {
                self.storage_array_fields
                    .insert(ident.name.clone(), (slot, n));
            }

            // Create storage field IR node
            let field_ir = ast_helpers::storage_field(ident.name.clone(), slot, ty.clone());
            self.storage_fields.push(field_ir);

            // Check if the field type resolves to a packed struct
            let composite_type = self.resolve_storage_packed_struct_type(type_sig);

            // Register in scope with the correct location
            let binding = VarBinding {
                value: ast_helpers::const_int(
                    slot as i64,
                    EvmContext::InFunction(contract_name.clone()),
                ),
                location,
                storage_slot: Some(slot),
                _ty: ty,
                let_bind_name: None,
                composite_type,
                composite_base: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(ident.name.clone(), binding);
        }

        // Process contract-level constants
        for (const_decl, expr) in &contract.consts {
            let val = self.lower_expr(expr)?;
            let ty = const_decl
                .ty
                .as_ref()
                .map(|ts| self.lower_type_sig(ts))
                .unwrap_or(EvmType::Base(EvmBaseType::UIntT(256)));
            let binding = VarBinding {
                value: Rc::clone(&val),
                location: DataLocation::Stack,
                storage_slot: None,
                _ty: ty,
                let_bind_name: None,
                composite_type: None,
                composite_base: None,
            };
            self.scopes
                .last_mut()
                .expect("scope stack empty")
                .bindings
                .insert(const_decl.name.name.clone(), binding);
        }

        // Collect internal functions for call resolution
        self.contract_functions.clear();
        self.lowered_functions.clear();
        for fn_decl in &contract.functions {
            if let Some(body) = &fn_decl.body {
                let params = fn_decl
                    .params
                    .iter()
                    .map(|(id, ty)| (id.name.clone(), ty.clone()))
                    .collect();
                self.contract_functions
                    .push((fn_decl.name.name.clone(), params, body.clone()));
            }
        }

        // Lower contract function bodies
        let mut fn_bodies: Vec<(&edge_ast::ContractFnDecl, Option<RcExpr>)> = Vec::new();
        for fn_decl in &contract.functions {
            if let Some(body) = &fn_decl.body {
                let body_ir = self.lower_contract_fn_body(&contract_name, fn_decl, body)?;
                fn_bodies.push((fn_decl, Some(body_ir)));
            } else {
                fn_bodies.push((fn_decl, None));
            }
        }

        // Build dispatcher (runtime entry point) with inlined function bodies
        let runtime = self.build_dispatcher(&contract_name, &fn_bodies)?;

        // Internal functions are stored separately (not Concat'd to runtime)
        // so they survive halting-DCE in the cleanup pass.
        let internal_functions: Vec<RcExpr> = self.lowered_functions.clone();

        // Constructor: EVM storage is zero-initialized, so no SSTOREs needed.
        // Transient fields are also auto-zeroed per EIP-1153 at the start of each tx.
        let constructor_ctx = EvmContext::InFunction(format!("{contract_name}::constructor"));
        let constructor =
            ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), constructor_ctx);

        Ok(EvmContract {
            name: contract_name,
            storage_fields: self.storage_fields.clone(),
            constructor,
            runtime,
            internal_functions,
            memory_high_water: self.next_memory_offset,
        })
    }

    /// Create a synthetic contract from free functions.
    ///
    /// When a file has only free functions and no explicit contract declaration,
    /// this wraps them in a synthetic contract so the dispatcher/deployment pipeline
    /// can generate deployable bytecode.
    fn create_synthetic_contract(
        &mut self,
        fn_stmts: &[(&edge_ast::FnDecl, &edge_ast::CodeBlock)],
        toplevel_consts: &IndexMap<String, VarBinding>,
    ) -> Result<EvmContract, IrError> {
        use edge_ast::Ident;

        // Build a synthetic ContractDecl with free functions as public contract functions
        let contract_functions: Vec<edge_ast::ContractFnDecl> = fn_stmts
            .iter()
            .map(|(fn_decl, body)| edge_ast::ContractFnDecl {
                name: fn_decl.name.clone(),
                params: fn_decl.params.clone(),
                returns: fn_decl.returns.clone(),
                is_pub: true,
                is_ext: false,
                is_mut: fn_decl.is_mut,
                body: Some((*body).clone()),
                span: fn_decl.span.clone(),
            })
            .collect();

        let synthetic_contract = edge_ast::ContractDecl {
            name: Ident {
                name: "__Module__".to_owned(),
                span: edge_types::span::Span::default(),
            },
            fields: Vec::new(),
            consts: Vec::new(),
            functions: contract_functions,
            span: edge_types::span::Span::default(),
        };

        self.lower_contract(&synthetic_contract, toplevel_consts)
    }
}
