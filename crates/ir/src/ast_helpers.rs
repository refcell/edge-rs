//! Builder/convenience functions for constructing IR nodes.
//!
//! These helpers reduce boilerplate when building `EvmExpr` trees
//! during AST lowering.

use std::rc::Rc;

use crate::schema::{
    EvmBaseType, EvmBinaryOp, EvmConstant, EvmContext, EvmExpr, EvmTernaryOp, EvmType, EvmUnaryOp,
    RcExpr,
};

// ---- Constants ----

/// Create a small integer constant.
pub fn const_int(val: i64, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Const(
        EvmConstant::SmallInt(val),
        EvmType::Base(EvmBaseType::UIntT(256)),
        ctx,
    ))
}

/// Create a big integer constant from hex.
pub fn const_bigint(hex: String, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Const(
        EvmConstant::LargeInt(hex),
        EvmType::Base(EvmBaseType::UIntT(256)),
        ctx,
    ))
}

/// Create a boolean constant.
pub fn const_bool(val: bool, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Const(
        EvmConstant::Bool(val),
        EvmType::Base(EvmBaseType::BoolT),
        ctx,
    ))
}

/// Create an address constant.
pub fn const_addr(hex: String, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Const(
        EvmConstant::Addr(hex),
        EvmType::Base(EvmBaseType::AddrT),
        ctx,
    ))
}

/// Create a typed constant.
pub fn const_typed(val: EvmConstant, ty: EvmType, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Const(val, ty, ctx))
}

// ---- Leaf nodes ----

/// Create an argument reference.
pub fn arg(ty: EvmType, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Arg(ty, ctx))
}

/// Create an empty tuple.
pub fn empty(ty: EvmType, ctx: EvmContext) -> RcExpr {
    Rc::new(EvmExpr::Empty(ty, ctx))
}

// ---- Binary operations ----

/// Create a binary operation.
pub fn bop(op: EvmBinaryOp, lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Bop(op, lhs, rhs))
}

/// Shorthand: addition
pub fn add(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Add, lhs, rhs)
}

/// Shorthand: subtraction
pub fn sub(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Sub, lhs, rhs)
}

/// Shorthand: multiplication
pub fn mul(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Mul, lhs, rhs)
}

/// Shorthand: checked addition (reverts on overflow)
pub fn checked_add(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::CheckedAdd, lhs, rhs)
}

/// Shorthand: checked subtraction (reverts on underflow)
pub fn checked_sub(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::CheckedSub, lhs, rhs)
}

/// Shorthand: checked multiplication (reverts on overflow)
pub fn checked_mul(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::CheckedMul, lhs, rhs)
}

/// Shorthand: shift left (`SHL` `shift_amount`, `value` — EVM operand order)
pub fn shl(shift_amount: RcExpr, value: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Shl, shift_amount, value)
}

/// Shorthand: logical shift right (`SHR` `shift_amount`, `value` — EVM operand order)
pub fn shr(shift_amount: RcExpr, value: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Shr, shift_amount, value)
}

/// Shorthand: bitwise AND
pub fn bitand(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::And, lhs, rhs)
}

/// Shorthand: bitwise OR
pub fn bitor(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Or, lhs, rhs)
}

/// Shorthand: storage load
pub fn sload(slot: RcExpr, state: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::SLoad, slot, state)
}

/// Shorthand: transient storage load
pub fn tload(slot: RcExpr, state: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::TLoad, slot, state)
}

/// Shorthand: equality comparison
pub fn eq(lhs: RcExpr, rhs: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::Eq, lhs, rhs)
}

// ---- Unary operations ----

/// Create a unary operation.
pub fn uop(op: EvmUnaryOp, expr: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Uop(op, expr))
}

/// Shorthand: is zero check
pub fn iszero(expr: RcExpr) -> RcExpr {
    uop(EvmUnaryOp::IsZero, expr)
}

// ---- Ternary operations ----

/// Create a ternary operation.
pub fn top(op: EvmTernaryOp, a: RcExpr, b: RcExpr, c: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Top(op, a, b, c))
}

/// Shorthand: storage store
pub fn sstore(slot: RcExpr, val: RcExpr, state: RcExpr) -> RcExpr {
    top(EvmTernaryOp::SStore, slot, val, state)
}

/// Shorthand: transient storage store
pub fn tstore(slot: RcExpr, val: RcExpr, state: RcExpr) -> RcExpr {
    top(EvmTernaryOp::TStore, slot, val, state)
}

/// Shorthand: memory store
pub fn mstore(offset: RcExpr, val: RcExpr, state: RcExpr) -> RcExpr {
    top(EvmTernaryOp::MStore, offset, val, state)
}

/// Memory load at offset.
pub fn mload(offset: RcExpr, state: RcExpr) -> RcExpr {
    bop(EvmBinaryOp::MLoad, offset, state)
}

// ---- Tuple operations ----

/// Get element at index from a tuple.
pub fn get(expr: RcExpr, idx: usize) -> RcExpr {
    Rc::new(EvmExpr::Get(expr, idx))
}

/// Sequence two expressions (evaluate both, return second).
pub fn concat(a: RcExpr, b: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Concat(a, b))
}

// ---- Control flow ----

/// If-then-else.
pub fn if_then_else(pred: RcExpr, inputs: RcExpr, then_: RcExpr, else_: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::If(pred, inputs, then_, else_))
}

/// Do-while loop.
pub fn do_while(inputs: RcExpr, pred_and_body: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::DoWhile(inputs, pred_and_body))
}

// ---- EVM-specific ----

/// Internal function call.
pub fn call(name: String, args: Vec<RcExpr>) -> RcExpr {
    Rc::new(EvmExpr::Call(name, args))
}

/// Return from contract.
pub fn return_op(offset: RcExpr, size: RcExpr, state: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::ReturnOp(offset, size, state))
}

/// Revert.
pub fn revert(offset: RcExpr, size: RcExpr, state: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Revert(offset, size, state))
}

/// Function definition.
pub fn function(name: String, in_ty: EvmType, out_ty: EvmType, body: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::Function(name, in_ty, out_ty, body))
}

/// Function selector.
pub fn selector(sig: String) -> RcExpr {
    Rc::new(EvmExpr::Selector(sig))
}

/// Let binding: compute value once, reference via Var(name) in body.
pub fn let_bind(name: String, value: RcExpr, body: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::LetBind(name, value, body))
}

/// Variable reference to a `LetBind`.
pub fn var(name: String) -> RcExpr {
    Rc::new(EvmExpr::Var(name))
}

/// Write to a `LetBind` variable's memory slot. Pushes 0 values to stack.
pub fn var_store(name: String, value: RcExpr) -> RcExpr {
    Rc::new(EvmExpr::VarStore(name, value))
}

/// Drop a variable (marks end of lifetime for slot reclamation).
pub fn drop_var(name: String) -> RcExpr {
    Rc::new(EvmExpr::Drop(name))
}

/// Storage field definition.
pub fn storage_field(name: String, slot: usize, ty: EvmType) -> RcExpr {
    Rc::new(EvmExpr::StorageField(name, slot, ty))
}

/// Calldata copy: (`dest_offset`, `cd_offset`, `size`) -> state effect.
/// Copies `size` bytes from calldata at `cd_offset` to memory at `dest_offset`.
pub fn calldatacopy(dest: RcExpr, cd_offset: RcExpr, size: RcExpr) -> RcExpr {
    top(EvmTernaryOp::CalldataCopy, dest, cd_offset, size)
}

/// Memory copy: (`dest`, `src`, `size`) -> state effect.
/// Copies `size` bytes from memory at `src` to memory at `dest`.
pub fn mcopy(dest: RcExpr, src: RcExpr, size: RcExpr) -> RcExpr {
    top(EvmTernaryOp::Mcopy, dest, src, size)
}

/// Keccak256 hash: (offset, size, state) -> hash.
/// The state parameter captures the memory dependency so that
/// keccak256 calls with different memory contents are distinguishable.
pub fn keccak256(offset: RcExpr, size: RcExpr, state: RcExpr) -> RcExpr {
    top(EvmTernaryOp::Keccak256, offset, size, state)
}

/// Symbolic memory region allocation.
/// Returns an expression that evaluates to the base address of the region.
/// `region_id` must be unique per allocation site; `size_words` is the number of 32-byte words.
pub fn mem_region(region_id: i64, size_words: i64) -> RcExpr {
    Rc::new(EvmExpr::MemRegion(region_id, size_words))
}
