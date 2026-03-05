# Edge Language Spec Coverage Report

> Produced by spec-analyst. Covers every construct defined in `book/specs/` against existing examples and compiler implementation.

---

## 1. Feature Matrix

| Feature | Spec File | Example File(s) | Lexer | Parser | AST | Typeck | IR | Codegen | Status |
|---------|-----------|-----------------|-------|--------|-----|--------|----|---------|--------|
| **Literals** | | | | | | | | | |
| Decimal literals | syntax/compile/literals.md | types.edge, math.edge | Yes | Yes | `Lit::Int` | -- | Yes | Yes | Full |
| Hex literals (0x) | syntax/compile/literals.md | tokens/erc20.edge (MAX_UINT) | Yes | Yes | `Lit::Hex` | -- | Yes | Yes | Full |
| Binary literals (0b) | syntax/compile/literals.md | -- | Yes | Yes | `Lit::Bin` | -- | Yes | Yes | Partial (no example) |
| String literals | syntax/compile/literals.md | -- | Yes | Yes | `Lit::Str` | -- | Push(0) stub | -- | Partial (lexer+parser only, no codegen) |
| Boolean literals (true/false) | syntax/compile/literals.md | -- | Yes (sugar to 0x01/0x00) | Yes | `Lit::Bool` | -- | Yes | Yes | Full |
| Numeric type suffix (1u8) | syntax/compile/literals.md | -- | Yes | Yes | -- | -- | -- | -- | Partial (no example) |
| Underscore separators (1_000) | syntax/compile/literals.md | -- | Yes | Yes | -- | -- | -- | -- | Partial (no example) |
| **Identifiers** | | | | | | | | | |
| C-style identifiers | syntax/identifiers.md | all examples | Yes | Yes | `Ident` | Yes | Yes | Yes | Full |
| **Comments** | | | | | | | | | |
| Line comments (//) | syntax/comments.md | all examples | Yes | Yes (skip) | -- | -- | -- | -- | Full |
| Block comments (/* */) | syntax/comments.md | -- | Yes (nested) | Yes (skip) | -- | -- | -- | -- | Partial (no example) |
| Item devdoc (///) | syntax/comments.md | -- | No (treated as //) | No | -- | -- | -- | -- | Missing |
| Module devdoc (//!) | syntax/comments.md | -- | No | No | -- | -- | -- | -- | Missing |
| **Data Locations** | | | | | | | | | |
| &s (persistent storage) | syntax/locations.md | types.edge, erc20.edge | Yes | Yes | `Location::Stack` | Yes | Yes (SLOAD/SSTORE) | Yes | Full |
| &t (transient storage) | syntax/locations.md | types.edge | Yes | Yes | `Location::Transient` | -- | No | No | Partial (lex+parse only) |
| &m (memory) | syntax/locations.md | types.edge | Yes | Yes | `Location::Memory` | -- | No | No | Partial (lex+parse only) |
| &cd (calldata) | syntax/locations.md | -- | Yes (bug: consumes extra chars) | Yes | `Location::Calldata` | -- | No | No | Partial (lexer bug) |
| &rd (returndata) | syntax/locations.md | -- | Yes (bug) | Yes | `Location::Returndata` | -- | No | No | Partial (lexer bug) |
| &ic (internal code) | syntax/locations.md | -- | Yes (bug) | Yes | `Location::ImmutableCode` | -- | No | No | Partial (lexer bug) |
| &ec (external code) | syntax/locations.md | -- | Yes (bug) | Yes | `Location::ExternalCode` | -- | No | No | Partial (lexer bug) |
| Pointer type (&s ptr) | syntax/types/primitives.md | -- | No | No | -- | -- | -- | -- | Missing |
| **Primitive Types** | | | | | | | | | |
| u8..u256 (unsigned int) | syntax/types/primitives.md | types.edge | Yes | Yes | `PrimitiveType::UInt` | Yes | Yes | Yes | Full |
| i8..i256 (signed int) | syntax/types/primitives.md | types.edge | Yes | Yes | `PrimitiveType::Int` | Yes | -- | -- | Partial (no signed IR ops) |
| b1..b32 (fixed bytes) | syntax/types/primitives.md | types.edge, erc721.edge | Yes | Yes | `PrimitiveType::FixedBytes` | Yes | -- | -- | Partial |
| addr (address) | syntax/types/primitives.md | erc20.edge | Yes | Yes | `PrimitiveType::Address` | Yes | Yes | Yes | Full |
| bool | syntax/types/primitives.md | types.edge, expressions.edge | Yes | Yes | `PrimitiveType::Bool` | Yes | Yes | Yes | Full |
| bit | syntax/types/primitives.md | types.edge | Yes | Yes | `PrimitiveType::Bit` | Yes | -- | -- | Partial (no example usage) |
| **Operators** | | | | | | | | | |
| Arithmetic (+, -, *, /, %, **) | syntax/operators.md | expressions.edge, math.edge | Yes | Yes | `BinOp::*` | -- | Yes | Yes | Full |
| Compound assign (+=, -=, etc.) | syntax/operators.md | -- | Yes | No (not parsed as stmt) | -- | -- | -- | -- | Partial (lex only) |
| Bitwise (&, \|, ^, ~, <<, >>) | syntax/operators.md | expressions.edge | Yes | Yes | `BinOp::Bitwise*` | -- | Yes | Yes | Full |
| Comparison (==, !=, <, <=, >, >=) | syntax/operators.md | expressions.edge, math.edge | Yes | Yes | `BinOp::Eq/Neq/Lt/Gt/Lte/Gte` | -- | Yes | Yes | Full |
| Logical (&&, \|\|, !) | syntax/operators.md | auth.edge | Yes | Yes | `BinOp::Logical*` | -- | Yes | Yes | Full |
| Unary negation (-) | syntax/operators.md | -- | Yes | Yes | `UnaryOp::Neg` | -- | No | No | Partial (no IR) |
| Unary bitwise NOT (~) | syntax/operators.md | -- | Yes | Yes | `UnaryOp::BitwiseNot` | -- | No | No | Partial (no IR) |
| Unary logical NOT (!) | syntax/operators.md | -- | Yes | Yes | `UnaryOp::LogicalNot` | -- | No | No | Partial (no IR) |
| Operator precedence (Pratt) | syntax/operators.md | -- | -- | Yes (precedence climbing) | -- | -- | -- | -- | Full |
| **Expressions** | | | | | | | | | |
| Binary operations | syntax/expressions.md | expressions.edge | Yes | Yes | `Expr::Binary` | -- | Yes | Yes | Full |
| Unary operations | syntax/expressions.md | -- | Yes | Yes | `Expr::Unary` | -- | No | No | Partial |
| Ternary (? :) | syntax/expressions.md, syntax/control/branching.md | -- | Yes (? token) | No | `Expr::Ternary` (AST exists) | -- | No | No | Missing (not parsed) |
| Function calls | syntax/types/function.md | math.edge, erc20.edge | Yes | Yes | `Expr::FunctionCall` | -- | Stub (push 0) | -- | Partial (no real call) |
| Field access (expr.field) | syntax/types/products.md | erc4626.edge | Yes | Yes | `Expr::FieldAccess` | -- | No | No | Partial |
| Tuple field access (expr.0) | syntax/types/products.md | -- | No | No | `Expr::TupleFieldAccess` (AST exists) | -- | No | No | Missing |
| Array element access (arr[i]) | syntax/types/arrays.md | erc20.edge (balances[account]) | Yes | Yes | `Expr::ArrayIndex` | -- | Stub | Stub | Partial |
| Array range access (arr[i:j]) | syntax/types/arrays.md | -- | Yes | Yes | `Expr::ArrayIndex` (3rd arg) | -- | No | No | Missing |
| Struct instantiation | syntax/types/products.md | -- | -- | No | `Expr::StructInstantiation` (AST exists) | -- | No | No | Missing |
| Tuple instantiation | syntax/types/products.md | -- | -- | No | `Expr::TupleInstantiation` (AST exists) | -- | No | No | Missing |
| Array instantiation | syntax/types/arrays.md | -- | -- | No | `Expr::ArrayInstantiation` (AST exists) | -- | No | No | Missing |
| Union instantiation (T::V()) | syntax/types/sum.md | -- | -- | No | `Expr::UnionInstantiation` (AST exists) | -- | No | No | Missing |
| Pattern match (matches) | syntax/types/sum.md | -- | Yes (keyword) | No | `Expr::PatternMatch` (AST exists) | -- | No | No | Missing |
| Arrow function (=>) | syntax/types/function.md | -- | Yes (=> token) | No | `Expr::ArrowFunction` (AST exists) | -- | No | No | Missing |
| Path expressions (a::b::c) | -- | safe_transfer.edge | Yes | Yes | `Expr::Path` | -- | No | No | Partial |
| Parenthesized expr | syntax/expressions.md | expressions.edge | Yes | Yes | `Expr::Paren` | -- | Yes | Yes | Full |
| Builtin calls (@name()) | builtins.md | erc20.edge (@caller) | Yes (@ token) | Yes | `Expr::At` | -- | Partial (caller,value,timestamp,number) | Yes | Partial |
| Assignment (lhs = rhs) | syntax/variables.md | all examples | Yes | Yes | `Expr::Assign` | -- | Yes | Yes | Full |
| **Variables** | | | | | | | | | |
| Variable declaration (let) | syntax/variables.md | all examples | Yes | Yes | `Stmt::VarDecl` | -- | Yes (alloc_local) | -- | Full |
| Variable assignment | syntax/variables.md | all examples | Yes | Yes | `Stmt::VarAssign` / `Expr::Assign` | -- | Yes | Yes | Full |
| Mutability (mut) | syntax/variables.md | -- | Yes (keyword) | Partial (fn args) | -- | -- | No | No | Partial |
| **Constants** | | | | | | | | | |
| Constant declaration | syntax/compile/constants.md | types.edge, math.edge | Yes | Yes | `Stmt::ConstAssign` | Stub (value=0) | Skip | -- | Partial (no const eval) |
| Constant inference | syntax/compile/constants.md | -- | -- | -- | -- | No | -- | -- | Missing |
| **Type System** | | | | | | | | | |
| Type declaration | syntax/types/assignment.md | types.edge | Yes | Yes | `Stmt::TypeAssign` | -- | -- | -- | Partial (no type params) |
| Type alias | syntax/types/assignment.md | types.edge | Yes | Yes | -- | -- | -- | -- | Partial |
| Struct signature | syntax/types/products.md | -- | -- | No | `TypeSig::Struct` (AST exists) | -- | -- | -- | Missing |
| Packed struct | syntax/types/products.md | -- | Yes (keyword) | No | -- | -- | -- | -- | Missing |
| Tuple signature | syntax/types/products.md | -- | -- | Yes | `TypeSig::Tuple` | -- | -- | -- | Partial |
| Packed tuple | syntax/types/products.md | -- | Yes (keyword) | No | `TypeSig::PackedTuple` (AST exists) | -- | -- | -- | Missing |
| Union/sum type signature | syntax/types/sum.md | -- | -- | No | `TypeSig::Union` (AST exists) | -- | -- | -- | Missing |
| Array type signature | syntax/types/arrays.md | -- | -- | No | `TypeSig::Array` (AST exists) | -- | -- | -- | Missing |
| Packed array | syntax/types/arrays.md | -- | Yes (keyword) | No | `TypeSig::PackedArray` (AST exists) | -- | -- | -- | Missing |
| Function type signature | syntax/types/function.md | -- | -- | No | `TypeSig::Function` (AST exists) | -- | -- | -- | Missing |
| Generics/type params (<T>) | syntax/types/generics.md | -- | -- | Partial (parse_type_sig only) | `TypeDecl::type_params` | -- | -- | -- | Partial |
| Named type w/ params (Map<K,V>) | syntax/types/generics.md | erc20.edge (map<addr,u256>) | -- | Yes | `TypeSig::Named(ident, params)` | -- | -- | -- | Partial |
| **Traits** | | | | | | | | | |
| Trait declaration | syntax/types/traits.md | auth.edge, safe_transfer.edge | Yes | Yes | `Stmt::TraitDecl` | -- | -- | -- | Partial (no solving) |
| Supertraits (: A & B) | syntax/types/traits.md | -- | -- | Yes (uses + not &) | -- | -- | -- | -- | Partial (wrong separator) |
| Trait items (fn, const, type) | syntax/types/traits.md | auth.edge | -- | Yes | `TraitItem::*` | -- | -- | -- | Partial |
| Default implementations | syntax/types/traits.md | -- | -- | Yes | -- | -- | -- | -- | Partial |
| Trait constraints on generics | syntax/types/traits.md | -- | -- | No | -- | -- | -- | -- | Missing |
| **Implementation Blocks** | | | | | | | | | |
| impl Type { } | syntax/types/implementation.md | -- | Yes | Yes | `Stmt::ImplBlock` | -- | -- | -- | Partial (no codegen) |
| impl Type: Trait { } | syntax/types/implementation.md | -- | -- | No (trait_impl=None) | -- | -- | -- | -- | Missing |
| Self keyword | syntax/types/implementation.md | -- | Yes (keyword) | No (not resolved) | -- | -- | -- | -- | Missing |
| Associated types | syntax/types/implementation.md | -- | -- | Yes (in impl items) | -- | -- | -- | -- | Partial |
| Associated constants | syntax/types/implementation.md | -- | -- | Yes (in impl items) | -- | -- | -- | -- | Partial |
| **Functions** | | | | | | | | | |
| Function declaration | syntax/types/function.md | all examples | Yes | Yes | `FnDecl` | Yes | Yes | Yes | Full |
| Function assignment (body) | syntax/types/function.md | all examples | Yes | Yes | `Stmt::FnAssign` | Yes | Yes | Yes | Full |
| Return types | syntax/types/function.md | all examples | Yes | Yes | `FnDecl::returns` | Yes | Yes | Yes | Full |
| Arrow functions (=>) | syntax/types/function.md | -- | Yes (=> token) | No | `Expr::ArrowFunction` (AST) | -- | -- | -- | Missing |
| Function type params | syntax/types/function.md | -- | -- | No | `FnDecl::type_params` (exists, empty) | -- | -- | -- | Missing |
| Internal function calls | syntax/types/function.md | erc20.edge (_transfer) | Yes | Yes | `Expr::FunctionCall` | -- | Stub | -- | Partial (no actual call) |
| **Events** | | | | | | | | | |
| Event declaration | syntax/types/events.md | erc20.edge, erc721.edge | Yes | Yes | `Stmt::EventDecl` | -- | -- | -- | Partial (no LOG codegen) |
| Indexed fields | syntax/types/events.md | erc20.edge | Yes | Yes | `EventField::indexed` | -- | -- | -- | Partial |
| Anonymous events (anon) | syntax/types/events.md | -- | Yes (keyword) | No | `EventDecl::is_anon` (exists) | -- | -- | -- | Missing |
| Emit statement | -- | erc20.edge | Yes | Yes | `Stmt::Emit` | -- | Stub (pop args) | -- | Partial |
| **ABI** | | | | | | | | | |
| ABI declaration | syntax/types/abi.md | all token examples | Yes | Yes | `Stmt::AbiDecl` | -- | -- | -- | Partial (no codegen) |
| ABI supertypes (: A & B) | syntax/types/abi.md | -- | -- | No (superabis=empty) | -- | -- | -- | -- | Missing |
| mut function modifier | syntax/types/abi.md | -- | Yes (keyword) | No (in abi body) | `AbiFnDecl::is_mut` (exists) | -- | -- | -- | Missing |
| **Contracts** | | | | | | | | | |
| Contract declaration | syntax/types/contracts.md | counter.edge, erc20.edge | Yes | Yes | `Stmt::ContractDecl` | Yes | Yes | Yes | Full |
| Contract fields (storage) | syntax/types/contracts.md | erc20.edge | Yes | Yes | `ContractDecl::fields` | Yes (layout) | Yes | Yes | Full |
| Contract functions | syntax/types/contracts.md | erc20.edge | Yes | Yes | `ContractFnDecl` | Yes | Yes | Yes | Full |
| Contract impl (impl C: ABI) | syntax/types/contracts.md | -- | -- | No | `Stmt::ContractImpl` (AST exists) | -- | -- | -- | Missing |
| ext modifier | syntax/types/contracts.md | -- | Yes | Yes (in contract body) | `ContractFnDecl::is_ext` | -- | -- | -- | Partial |
| mut modifier on fn | syntax/types/contracts.md | -- | Yes | Yes | `ContractFnDecl::is_mut` | -- | -- | -- | Partial |
| pub modifier on fn | syntax/types/contracts.md | all contract examples | Yes | Yes | `ContractFnDecl::is_pub` | Yes | Yes | Yes | Full |
| Contract constants | -- | tokens/erc20.edge | Yes | Yes | `ContractDecl::consts` | Stub | -- | -- | Partial |
| Dispatcher generation | -- | -- | -- | -- | -- | -- | -- | Yes | Full |
| **Modules** | | | | | | | | | |
| Module declaration (mod) | syntax/modules.md | safe_transfer.edge, tokens/*.edge | Yes | Yes | `Stmt::ModuleDecl` | -- | -- | -- | Partial (no body parsing) |
| Module import (use) | syntax/modules.md | erc4626.edge, erc721.edge | Yes | Yes | `Stmt::ModuleImport` | -- | -- | -- | Partial (no resolution) |
| Nested paths (a::b::c) | syntax/modules.md | tokens/erc20.edge | Yes | Yes | -- | -- | -- | -- | Partial |
| pub use re-export | syntax/modules.md | -- | -- | No | -- | -- | -- | -- | Missing |
| super keyword | syntax/modules.md | -- | Yes (keyword) | No (not resolved) | -- | -- | -- | -- | Missing |
| File-as-module semantics | syntax/modules.md | -- | -- | -- | -- | -- | -- | -- | Missing |
| **Control Flow - Branching** | | | | | | | | | |
| if/else if/else | syntax/control/branching.md | auth.edge, math.edge | Yes | Yes | `Stmt::IfElse` | -- | Yes (labels) | Yes | Full |
| if matches (pattern) | syntax/control/branching.md | -- | Yes (keyword) | No | `Stmt::IfMatch` (AST exists) | -- | -- | -- | Missing |
| match statement | syntax/control/branching.md | -- | Yes | Yes | `Stmt::Match` | -- | No | No | Partial (parse only) |
| Match arm (union pattern) | syntax/control/branching.md | -- | -- | Yes | `MatchPattern::Union` | -- | No | No | Partial |
| Wildcard pattern (_) | syntax/control/branching.md | -- | -- | Yes | `MatchPattern::Wildcard` | -- | No | No | Partial |
| Ternary (? :) | syntax/control/branching.md | -- | Yes | No | `Expr::Ternary` (AST) | -- | No | No | Missing |
| Short-circuit eval | syntax/control/branching.md | -- | -- | -- | -- | -- | No | No | Missing |
| **Control Flow - Loops** | | | | | | | | | |
| Core loop (loop {}) | syntax/control/loops.md | -- | Yes | Yes | `Stmt::Loop` | -- | No | No | Partial (no example, no IR) |
| For loop | syntax/control/loops.md | -- | Yes | Yes | `Stmt::ForLoop` | -- | No | No | Partial (no example, no IR) |
| While loop | syntax/control/loops.md | math.edge (wad_sqrt) | Yes | Yes | `Stmt::WhileLoop` | -- | No | No | Partial (no IR) |
| Do-while loop | syntax/control/loops.md | -- | Yes | Yes | `Stmt::DoWhile` | -- | No | No | Partial (no example, no IR) |
| Break | syntax/control/loops.md | -- | Yes | Yes | `Stmt::Break` | -- | No | No | Partial |
| Continue | syntax/control/loops.md | -- | Yes | Yes | `Stmt::Continue` | -- | No | No | Partial |
| **Control Flow - Code Blocks** | | | | | | | | | |
| Code block { } | syntax/control/code.md | all examples | Yes | Yes | `CodeBlock` | -- | Yes | Yes | Full |
| Scoped blocks | syntax/control/code.md | -- | -- | Yes | -- | -- | -- | -- | Partial |
| **Compile Time** | | | | | | | | | |
| comptime fn | syntax/compile/functions.md | -- | Yes | Yes | `Stmt::ComptimeFn` | -- | No | No | Partial (parse only) |
| comptime branch | syntax/compile/branching.md | -- | Yes | Yes | `Stmt::ComptimeBranch` | -- | No | No | Partial (parse only) |
| Constant evaluation | syntax/compile/constants.md | -- | -- | -- | -- | Stub (hardcoded 0) | -- | -- | Missing |
| **Builtins** | | | | | | | | | |
| @typeInfo | builtins.md | -- | Yes (@ token) | Yes (generic @) | `Expr::At` | No | No | No | Missing |
| @bitsize | builtins.md | -- | Yes | Yes | `Expr::At` | No | No | No | Missing |
| @fields | builtins.md | -- | Yes | Yes | `Expr::At` | No | No | No | Missing |
| @compilerError | builtins.md | -- | Yes | Yes | `Expr::At` | No | No | No | Missing |
| @hardFork | builtins.md | -- | Yes | Yes | `Expr::At` | No | No | No | Missing |
| @bytecode | builtins.md | -- | Yes | Yes | `Expr::At` | No | No | No | Missing |
| @caller (non-spec) | -- | erc20.edge | Yes | Yes | `Expr::At` | -- | Yes | Yes | Example only |
| HardFork enum type | builtins.md | -- | No | No | -- | No | No | No | Missing |
| TypeInfo union type | builtins.md | -- | No | No | -- | No | No | No | Missing |
| **Inline Assembly** | | | | | | | | | |
| asm block | inline.md | -- | No (no `asm` keyword) | No | No | No | No | No | Missing |
| Opcode mnemonics | inline.md | -- | No | No | No | No | No | No | Missing |
| Assembly inputs/outputs | inline.md | -- | No | No | No | No | No | No | Missing |
| **Semantics** | | | | | | | | | |
| Codesize optimization | semantics/codesize.md | -- | -- | -- | -- | -- | No | No | Missing |
| Function inlining | semantics/codesize.md | -- | -- | -- | -- | -- | No | No | Missing |
| Dead code elimination | semantics/codesize.md | -- | -- | -- | -- | -- | No | No | Missing |
| Namespace resolution | semantics/namespaces.md | -- | -- | -- | -- | No | No | No | Missing |
| Scoping rules | semantics/scoping.md | -- | -- | -- | -- | No | No | No | Missing |
| Visibility (pub) | semantics/visibility.md | erc20.edge | Yes | Yes | -- | -- | -- | -- | Partial |

---

## 2. Missing Examples Needed

The following spec features lack any `.edge` example file demonstrating them:

### Literals & Basic Syntax
- **binary_literals.edge** -- Binary literal syntax (0b1100), underscore separators (1_000_000), type suffixes (42u8)
- **string_literals.edge** -- String literal usage, escape sequences, string as packed u8 array
- **comments.edge** -- All four comment forms: line (//), block (/* */), item devdoc (///), module devdoc (//!)

### Type System
- **structs.edge** -- Struct declaration, instantiation, field access, packed structs, nested structs
- **tuples.edge** -- Tuple declaration, instantiation, field access (.0, .1), packed tuples
- **unions.edge** -- Sum type / union / enum declaration, instantiation (T::Variant()), pattern matching
- **arrays.edge** -- Array type signature, instantiation, element access, range access [i:j]
- **generics.edge** -- Generic type parameters, generic functions, monomorphization
- **function_types.edge** -- Function type signatures (T -> U), arrow functions (x => { })
- **type_aliases.edge** -- Type aliases, newtype wrappers ((MyType))

### Traits & Implementation
- **traits.edge** -- Trait declaration with associated types/constants/functions, supertraits, default impls
- **impl_blocks.edge** -- Implementation blocks, trait impl (impl T: Trait), Self keyword, associated items

### Control Flow
- **loops.edge** -- All loop variants: loop {}, for (;;), while (), do {} while (), break, continue
- **branching.edge** -- if/else if/else, ternary (?:), match with union patterns, if-matches, wildcards

### Compile Time
- **comptime.edge** -- comptime fn, comptime branches, constant evaluation

### Builtins & Assembly
- **builtins.edge** -- @typeInfo, @bitsize, @fields, @compilerError, @hardFork, @bytecode
- **inline_asm.edge** -- asm blocks with inputs, outputs, EVM opcodes

### Modules
- **modules.edge** -- Module declaration (mod name {}), nested modules, use/import, pub use, super keyword

### Advanced Contracts
- **abi_inheritance.edge** -- ABI supertypes (abi A: B & C), mut function modifier
- **contract_impl.edge** -- Contract implementation block for ABI (impl Contract: ABI)

---

## 3. Compiler Gaps

### Critical (Blocks compilation of spec-compliant programs)

1. **No struct/tuple/union/array type signatures in parser** -- The parser cannot parse struct `{}`, packed struct, union `|`, or array `[T; N]` type signatures. Only primitive types, named types with generic params, pointer types, and tuple types are handled in `parse_type_sig()`.
   - Crate: `parser` (`parse_type_sig` method, ~line 1377)
   - Need: Add cases for `OpenBrace` (struct), `|` (union), `OpenBracket` (array), `Keyword::Packed` prefix

2. **No struct/tuple/union instantiation in parser** -- No parsing rules for `MyStruct { field: value }`, `MyTuple(a, b)`, or `Type::Variant()`.
   - Crate: `parser` (`parse_primary_expr` method)
   - Need: Detect `Ident { ... }` for struct, handle path + `(` for union instantiation

3. **No ternary expression parsing** -- The `?` and `:` tokens are lexed but the parser has no ternary rule.
   - Crate: `parser` -- need to add ternary as a binary precedence level or special case

4. **No arrow function parsing** -- The `=>` token (FatArrow) is lexed but only used for match arms, not lambda expressions.
   - Crate: `parser`

5. **No unary expression lowering to IR** -- `UnaryOp::Neg`, `UnaryOp::BitwiseNot`, `UnaryOp::LogicalNot` are parsed but not lowered.
   - Crate: `ir` (`lower_expr`)
   - Need: Add `Expr::Unary` match case. Neg = `PUSH 0; SUB`, BitwiseNot = `NOT`, LogicalNot = `ISZERO`

6. **No loop lowering to IR** -- While, for, do-while, and core loops parse but are never lowered to IR instructions (jump/jumpdest labels).
   - Crate: `ir` (`lower_stmt`)
   - Need: Add `Stmt::WhileLoop`, `Stmt::ForLoop`, `Stmt::Loop`, `Stmt::DoWhile` match cases with label generation

7. **No match lowering to IR** -- Match statements parse but are not lowered.
   - Crate: `ir` (`lower_stmt`)

8. **No internal function call codegen** -- Function calls inside a contract body emit `Push(0)` stub instead of a JUMP to the function's code.
   - Crate: `ir` (`lower_expr` for `Expr::FunctionCall`)
   - Need: Emit JUMP to function label and handle return values

9. **No event/emit codegen** -- Emit statements pop all args but never generate LOG0..LOG4 instructions.
   - Crate: `ir` (`lower_stmt` for `Stmt::Emit`)
   - Need: Compute topic hashes, store data in memory, emit LOG instruction

10. **Constant evaluation hardcoded to 0** -- `ConstValue::value` is always 0 regardless of the expression.
    - Crate: `typeck` (`check_contract` line 172)
    - Need: Implement a simple constant evaluator for literals and basic arithmetic

11. **No trait impl parsing** -- `impl Type: Trait { }` is not parsed (trait_impl is always None).
    - Crate: `parser` (`parse_impl_block` line 1019)
    - Need: Parse `:` after type name, then trait identifier

12. **No module body parsing** -- `mod name { ... }` with body items is not parsed (only `mod name;` is supported).
    - Crate: `parser` (`parse_module`)
    - Need: Check for `{` after ident and parse body items

### High Priority (Functional completeness)

13. **No inline assembly support** -- No `asm` keyword, no opcode mnemonic parsing, no assembly block.
    - Crate: `lexer`, `parser`, `ast`, `ir`, `codegen` (all need new constructs)

14. **No builtin function implementation** -- @typeInfo, @bitsize, @fields, @compilerError, @hardFork, @bytecode are all unimplemented.
    - Crate: `typeck` and `ir` (need builtin dispatch)

15. **No item/module devdoc comments** -- `///` and `//!` treated as regular line comments.
    - Crate: `lexer` (need new token kinds), `parser`, `ast`

16. **No comptime evaluation** -- `comptime fn` and `comptime` branches are parsed but never evaluated.
    - Crate: Need a new `comptime` evaluation pass between typeck and IR

17. **No field access lowering** -- `Expr::FieldAccess` is parsed but not lowered to IR.
    - Crate: `ir` (`lower_expr`)

18. **No storage mapping codegen** -- Map reads/writes (e.g., `balances[account]`) produce dummy values.
    - Crate: `ir` (need keccak256 slot computation)

19. **Trait supertrait separator** -- Spec says `&` but parser uses `+`. Need to align with spec.
    - Crate: `parser` (`parse_trait_stub` line 513)

20. **Data location lexer bug** -- When &cd, &rd, &ic, &ec fail to match (e.g., `&c` without `d`), the consumed character is lost and a bitwise AND is emitted at wrong position.
    - Crate: `lexer` (& handling, lines 621-668)

### Medium Priority (Robustness)

21. **No error recovery in parser** -- Parser stops on first error.
22. **No type validation beyond contract existence** -- Typeck only checks contracts, not expressions.
23. **No selector collision detection** -- Multiple functions with same 4-byte selector are not caught.
24. **No pub visibility enforcement** -- `pub` is parsed but has no semantic effect outside contract functions.
25. **No signed integer IR instructions** -- SDIV, SMOD, SLT, SGT, SAR are not used for signed types.
26. **No deploy (initcode) bytecode generation** -- Only runtime bytecode is emitted.
27. **No file output** -- CompileOutput is computed but never written to disk.
28. **Single contract per file** -- Driver only compiles the last contract found.
29. **No HashMap/map type support** -- The `map<K, V>` type is parsed as a named generic but has no semantic backing.

---

## 4. Parser Grammar Gaps

The following spec grammar rules have no corresponding parser implementation:

| Spec Rule | Spec File | Status |
|-----------|-----------|--------|
| `<struct_signature>` | syntax/types/products.md | Not parsed |
| `<packed>` prefix on struct/tuple/array | syntax/types/products.md, arrays.md | Keyword lexed, not parsed |
| `<tuple_signature>` (in type position) | syntax/types/products.md | Partially parsed (no packed) |
| `<union_signature>` | syntax/types/sum.md | Not parsed |
| `<union_instantiation>` | syntax/types/sum.md | Not parsed |
| `<union_pattern>` in if-match | syntax/types/sum.md | Not parsed as expression |
| `<pattern_match>` (expr matches pattern) | syntax/types/sum.md | Not parsed |
| `<arrow_function>` | syntax/types/function.md | Not parsed |
| `<function_signature>` (as type) | syntax/types/function.md | Not parsed |
| `<array_signature>` `[T; N]` | syntax/types/arrays.md | Not parsed |
| `<array_instantiation>` `[expr, ...]` | syntax/types/arrays.md | Not parsed |
| `<ternary>` `? :` | syntax/control/branching.md | Not parsed |
| `<if_match_branch>` | syntax/control/branching.md | Not parsed |
| `<inline_assembly>` asm block | inline.md | Not parsed |
| `<module_declaration>` with body `{ }` | syntax/modules.md | Not parsed (only `mod name;`) |
| `<module_import>` with tree (`{A, B}`) | syntax/modules.md | Not parsed (only linear path) |
| `pub use` re-export | syntax/modules.md | Not parsed |
| `<event_signature>` (as type in struct/contract) | syntax/types/events.md | Not parsed |
| `<abi_declaration>` with supertypes | syntax/types/abi.md | Not parsed |
| `<contract_impl_block>` (impl C: ABI) | syntax/types/contracts.md | Not parsed |
| `<trait_constraints>` on type params | syntax/types/traits.md | Not parsed |
| `<item_devdoc>` `///` | syntax/comments.md | Not distinguished from `//` |
| `<module_devdoc>` `//!` | syntax/comments.md | Not lexed |
| `<data_location>` prefix on instantiation | syntax/types/arrays.md, products.md | Not parsed |
| `<bit>` type usage | syntax/types/primitives.md | Lexed/parsed but no example or semantics |

---

## 5. Recommended Example Files

A complete list of every example file that should exist to cover the full Edge specification:

### Core Language (`examples/`)
| File | Description |
|------|-------------|
| `examples/counter.edge` | Simple on-chain counter (EXISTS) |
| `examples/erc20.edge` | Minimal ERC-20 token (EXISTS) |
| `examples/expressions.edge` | Arithmetic, comparison, bitwise operators (EXISTS) |
| `examples/types.edge` | Primitive types and data locations (EXISTS) |
| `examples/literals.edge` | All literal forms: decimal, hex, binary, string, bool, suffixed, underscore |
| `examples/comments.edge` | All comment forms: //, /* */, ///, //! |
| `examples/variables.edge` | let, mut, const, variable assignment, type inference |
| `examples/structs.edge` | Struct declaration, packed struct, instantiation, field access, nested structs |
| `examples/tuples.edge` | Tuple declaration, packed tuple, instantiation, field access (.0) |
| `examples/unions.edge` | Sum types, enum, union with data, pattern matching, Option<T> |
| `examples/arrays.edge` | Array types [T; N], packed arrays, element access, range access |
| `examples/generics.edge` | Type parameters, generic functions, generic types, monomorphization |
| `examples/function_types.edge` | Function type signatures, arrow functions, higher-order functions |
| `examples/traits.edge` | Trait declaration, supertraits, default impls, trait constraints |
| `examples/impl_blocks.edge` | impl Type { }, impl Type: Trait { }, Self, associated types/consts |
| `examples/branching.edge` | if/else if/else, ternary, match, if-matches, wildcards |
| `examples/loops.edge` | loop, for, while, do-while, break, continue |
| `examples/modules.edge` | mod declaration with body, use imports, pub use, super, nested modules |
| `examples/comptime.edge` | comptime fn, comptime branches, constant evaluation |
| `examples/builtins.edge` | @typeInfo, @bitsize, @fields, @compilerError, @hardFork, @bytecode |
| `examples/inline_asm.edge` | Inline assembly blocks with inputs, outputs, EVM opcodes |
| `examples/events.edge` | Event declarations, indexed fields, anonymous events, emit |
| `examples/abi.edge` | ABI declarations, supertypes, mut modifier |
| `examples/contracts.edge` | Contract with impl for ABI, ext/mut modifiers, storage layout |
| `examples/data_locations.edge` | All 7 data locations, pointer semantics, location transitions |

### Libraries (`examples/lib/`)
| File | Description |
|------|-------------|
| `examples/lib/auth.edge` | Ownership and access control (EXISTS) |
| `examples/lib/math.edge` | Fixed-point and safe arithmetic (EXISTS) |
| `examples/lib/safe_transfer.edge` | Safe ERC-20 and ETH transfer utilities (EXISTS) |
| `examples/lib/reentrancy.edge` | Reentrancy guard using comptime for transient/persistent storage |
| `examples/lib/merkle.edge` | Merkle proof verification using keccak256 |

### Tokens (`examples/tokens/`)
| File | Description |
|------|-------------|
| `examples/tokens/erc20.edge` | Full ERC-20 fungible token (EXISTS) |
| `examples/tokens/erc721.edge` | Full ERC-721 non-fungible token (EXISTS) |
| `examples/tokens/erc4626.edge` | ERC-4626 tokenized vault (EXISTS) |
| `examples/tokens/erc1155.edge` | Multi-token standard (demonstrates array/mapping complexity) |

### Advanced (`examples/advanced/`)
| File | Description |
|------|-------------|
| `examples/advanced/proxy.edge` | Minimal proxy / delegatecall pattern |
| `examples/advanced/multisig.edge` | Multi-signature wallet |
| `examples/advanced/auction.edge` | English auction with time-based logic |
| `examples/advanced/amm.edge` | Constant product AMM (x*y=k) |

---

## Summary Statistics

| Category | Spec Features | Fully Implemented | Partially Implemented | Missing |
|----------|--------------|-------------------|-----------------------|---------|
| Literals & Comments | 9 | 3 | 4 | 2 |
| Data Locations | 8 | 1 | 6 | 1 |
| Primitive Types | 6 | 4 | 2 | 0 |
| Operators | 7 | 4 | 3 | 0 |
| Expressions | 15 | 4 | 4 | 7 |
| Variables & Constants | 4 | 2 | 1 | 1 |
| Type System | 12 | 0 | 3 | 9 |
| Traits & Impl | 7 | 0 | 5 | 2 |
| Functions | 6 | 3 | 1 | 2 |
| Events | 4 | 0 | 3 | 1 |
| ABI & Contracts | 9 | 4 | 3 | 2 |
| Modules | 6 | 0 | 3 | 3 |
| Control Flow | 12 | 2 | 7 | 3 |
| Compile Time | 3 | 0 | 2 | 1 |
| Builtins | 8 | 0 | 1 | 7 |
| Inline Assembly | 3 | 0 | 0 | 3 |
| Semantics | 5 | 0 | 1 | 4 |
| **TOTAL** | **124** | **27 (22%)** | **49 (40%)** | **48 (39%)** |

### Key Findings

1. **The lexer is ~90% complete** -- It handles all keywords, operators, literals, data locations, and special tokens. Gaps: devdoc comments, `asm` keyword, data location position bugs.

2. **The parser is ~55% complete** -- It handles the core statement/expression grammar well (functions, contracts, events, abi, basic control flow, operators with precedence) but is missing complex type signatures (struct, union, array), instantiation expressions, ternary, arrow functions, trait impls, module bodies, and inline assembly.

3. **The AST is ~95% complete** -- Almost all spec constructs have AST node definitions, even if the parser cannot produce them yet. This is excellent forward-looking design.

4. **The type checker is ~30% complete** -- It only checks contract existence, builds storage layouts, and computes selectors. No type validation, no expression typing, no constant evaluation, no trait solving.

5. **The IR lowerer is ~40% complete** -- It handles basic expressions (arithmetic, comparisons, idents, assigns) and if/else branching. Missing: loops, match, unary ops, function calls, events, field access, mapping slot computation.

6. **The codegen is ~50% complete** -- The Assembler with label resolution is well-designed and functional. The dispatcher is correctly generated. It emits valid EVM bytecode for simple contracts. Missing: internal function calls, event LOG instructions, deploy bytecode wrapper.

7. **22% of spec features have full end-to-end support** (lexer through codegen with examples).
8. **39% of spec features are entirely missing** from the compiler.
9. **10 example files exist** out of the ~30 recommended for full coverage.
