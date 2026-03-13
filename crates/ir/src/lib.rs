//! Edge Language Egglog-based Intermediate Representation.
//!
//! This crate provides the IR layer between the Edge AST and EVM bytecode
//! generation. It uses [egglog](https://github.com/egraphs-good/egglog) for
//! equality-saturation-based optimization.
//!
//! # Pipeline
//!
//! ```text
//! edge_ast::Program
//!   -> AstToEgglog::lower_program()   (AST -> egglog Terms)
//!   -> equality saturation             (optimize via rewrite rules)
//!   -> sexp::sexp_to_expr()              (extract optimized IR)
//!   -> EvmProgram                      (ready for codegen)
//! ```

#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]

pub mod ast_helpers;
pub mod cleanup;
pub mod costs;
pub mod mem_region;
pub mod optimizations;
pub mod region_forward;
pub mod pretty;
pub mod schedule;
pub mod schema;
pub mod sexp;
pub mod storage_hoist;
pub mod to_egglog;
pub mod u256_sort;
pub mod var_opt;

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
};

pub use costs::OptimizeFor;
use schema::{EvmBaseType, EvmConstant, EvmType};
pub use schema::{EvmContract, EvmExpr, EvmProgram, RcExpr};

// ============================================================
// Hash-consing: re-establish Rc sharing after tree-rebuilding passes
// ============================================================
//
// var_opt reconstructs IR nodes, breaking Rc sharing. This pass walks
// bottom-up and deduplicates structurally identical subtrees into
// shared Rc pointers, restoring the compact DAG representation.
// After hash-consing, DAG-aware s-expression serialization can emit
// compact egglog programs.

/// Hash-cons an IR expression tree: deduplicate structurally identical
/// subtrees into shared `Rc` pointers.
pub fn hash_cons(expr: &RcExpr) -> RcExpr {
    let mut cache: HashMap<HashConsKey, RcExpr> = HashMap::new();
    hash_cons_rec(expr, &mut cache)
}

/// Hash-cons all expressions in a program.
pub fn hash_cons_program(program: &mut EvmProgram) {
    let mut cache: HashMap<HashConsKey, RcExpr> = HashMap::new();
    for contract in &mut program.contracts {
        contract.runtime = hash_cons_rec(&contract.runtime, &mut cache);
        for func in &mut contract.internal_functions {
            *func = hash_cons_rec(func, &mut cache);
        }
        contract.constructor = hash_cons_rec(&contract.constructor, &mut cache);
    }
    for func in &mut program.free_functions {
        *func = hash_cons_rec(func, &mut cache);
    }
}

/// Hash-cons a single expression tree, restoring Rc sharing for structurally identical subtrees.
pub fn hash_cons_expr(expr: &RcExpr) -> RcExpr {
    let mut cache: HashMap<HashConsKey, RcExpr> = HashMap::new();
    hash_cons_rec(expr, &mut cache)
}

/// A hash key that captures node identity by type + inline data + child Rc pointers.
/// Since children are hash-consed first, pointer equality <=> structural equality.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct HashConsKey {
    /// Compact byte representation of the node
    bytes: Vec<u8>,
}

impl Hash for HashConsKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl HashConsKey {
    pub(crate) fn new() -> Self {
        Self {
            bytes: Vec::with_capacity(64),
        }
    }

    pub(crate) fn tag(&mut self, tag: u8) {
        self.bytes.push(tag);
    }

    pub(crate) fn ptr(&mut self, rc: &RcExpr) {
        let p = Rc::as_ptr(rc) as usize;
        self.bytes.extend_from_slice(&p.to_le_bytes());
    }

    pub(crate) fn str(&mut self, s: &str) {
        self.bytes
            .extend_from_slice(&(s.len() as u32).to_le_bytes());
        self.bytes.extend_from_slice(s.as_bytes());
    }

    pub(crate) fn usize(&mut self, v: usize) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i64(&mut self, v: i64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn bool(&mut self, v: bool) {
        self.bytes.push(v as u8);
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    pub(crate) fn u16(&mut self, v: u16) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }
}

pub(crate) fn key_for_type(k: &mut HashConsKey, ty: &EvmType) {
    match ty {
        EvmType::Base(b) => {
            k.tag(0);
            key_for_basetype(k, b);
        }
        EvmType::TupleT(types) => {
            k.tag(1);
            k.usize(types.len());
            for t in types {
                key_for_basetype(k, t);
            }
        }
        EvmType::ArrayT(elem, len) => {
            k.tag(2);
            key_for_basetype(k, elem);
            k.usize(*len);
        }
    }
}

pub(crate) fn key_for_basetype(k: &mut HashConsKey, bt: &EvmBaseType) {
    match bt {
        EvmBaseType::UIntT(n) => {
            k.tag(0);
            k.u16(*n);
        }
        EvmBaseType::IntT(n) => {
            k.tag(1);
            k.u16(*n);
        }
        EvmBaseType::BytesT(n) => {
            k.tag(2);
            k.u8(*n);
        }
        EvmBaseType::AddrT => k.tag(3),
        EvmBaseType::BoolT => k.tag(4),
        EvmBaseType::UnitT => k.tag(5),
        EvmBaseType::StateT => k.tag(6),
    }
}

pub(crate) fn key_for_const(k: &mut HashConsKey, c: &EvmConstant) {
    match c {
        EvmConstant::SmallInt(i) => {
            k.tag(0);
            k.i64(*i);
        }
        EvmConstant::LargeInt(s) => {
            k.tag(1);
            k.str(s);
        }
        EvmConstant::Bool(b) => {
            k.tag(2);
            k.bool(*b);
        }
        EvmConstant::Addr(s) => {
            k.tag(3);
            k.str(s);
        }
    }
}

pub(crate) fn key_for_ctx(k: &mut HashConsKey, ctx: &schema::EvmContext) {
    match ctx {
        schema::EvmContext::InFunction(name) => {
            k.tag(0);
            k.str(name);
        }
        schema::EvmContext::InBranch(b, pred, input) => {
            k.tag(1);
            k.bool(*b);
            k.ptr(pred);
            k.ptr(input);
        }
        schema::EvmContext::InLoop(input, pred) => {
            k.tag(2);
            k.ptr(input);
            k.ptr(pred);
        }
    }
}

fn hash_cons_rec(expr: &RcExpr, cache: &mut HashMap<HashConsKey, RcExpr>) -> RcExpr {
    // Build key and hash-cons children first
    let mut k = HashConsKey::new();

    macro_rules! child {
        ($e:expr) => {
            hash_cons_rec($e, cache)
        };
    }

    let result: RcExpr = match expr.as_ref() {
        EvmExpr::Arg(ty, ctx) => {
            k.tag(0);
            key_for_type(&mut k, ty);
            key_for_ctx(&mut k, ctx);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::Const(c, ty, ctx) => {
            k.tag(1);
            key_for_const(&mut k, c);
            key_for_type(&mut k, ty);
            key_for_ctx(&mut k, ctx);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::Empty(ty, ctx) => {
            k.tag(2);
            key_for_type(&mut k, ty);
            key_for_ctx(&mut k, ctx);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::Bop(op, l, r) => {
            let nl = child!(l);
            let nr = child!(r);
            k.tag(3);
            k.u8(*op as u8);
            k.ptr(&nl);
            k.ptr(&nr);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nl, l) && Rc::ptr_eq(&nr, r) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Bop(*op, nl, nr))
            }
        }
        EvmExpr::Uop(op, a) => {
            let na = child!(a);
            k.tag(4);
            k.u8(*op as u8);
            k.ptr(&na);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Uop(*op, na))
            }
        }
        EvmExpr::Top(op, a, b, c) => {
            let na = child!(a);
            let nb = child!(b);
            let nc = child!(c);
            k.tag(5);
            k.u8(*op as u8);
            k.ptr(&na);
            k.ptr(&nb);
            k.ptr(&nc);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Top(*op, na, nb, nc))
            }
        }
        EvmExpr::Get(a, idx) => {
            let na = child!(a);
            k.tag(6);
            k.ptr(&na);
            k.usize(*idx);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Get(na, *idx))
            }
        }
        EvmExpr::Concat(a, b) => {
            let na = child!(a);
            let nb = child!(b);
            k.tag(7);
            k.ptr(&na);
            k.ptr(&nb);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Concat(na, nb))
            }
        }
        EvmExpr::If(cond, inputs, t, e) => {
            let nc = child!(cond);
            let ni = child!(inputs);
            let nt = child!(t);
            let ne = child!(e);
            k.tag(8);
            k.ptr(&nc);
            k.ptr(&ni);
            k.ptr(&nt);
            k.ptr(&ne);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nc, cond)
                && Rc::ptr_eq(&ni, inputs)
                && Rc::ptr_eq(&nt, t)
                && Rc::ptr_eq(&ne, e)
            {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::If(nc, ni, nt, ne))
            }
        }
        EvmExpr::DoWhile(a, b) => {
            let na = child!(a);
            let nb = child!(b);
            k.tag(9);
            k.ptr(&na);
            k.ptr(&nb);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::DoWhile(na, nb))
            }
        }
        EvmExpr::EnvRead(op, st) => {
            let ns = child!(st);
            k.tag(10);
            k.u8(*op as u8);
            k.ptr(&ns);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&ns, st) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::EnvRead(*op, ns))
            }
        }
        EvmExpr::EnvRead1(op, arg, st) => {
            let na = child!(arg);
            let ns = child!(st);
            k.tag(11);
            k.u8(*op as u8);
            k.ptr(&na);
            k.ptr(&ns);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, arg) && Rc::ptr_eq(&ns, st) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::EnvRead1(*op, na, ns))
            }
        }
        EvmExpr::Log(n, topics, doff, dsz, st) => {
            let new_topics: Vec<_> = topics.iter().map(|t| child!(t)).collect();
            let nd = child!(doff);
            let ns = child!(dsz);
            let nst = child!(st);
            k.tag(12);
            k.usize(*n);
            for t in &new_topics {
                k.ptr(t);
            }
            k.ptr(&nd);
            k.ptr(&ns);
            k.ptr(&nst);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::new(EvmExpr::Log(*n, new_topics, nd, ns, nst))
        }
        EvmExpr::Revert(a, b, c) => {
            let na = child!(a);
            let nb = child!(b);
            let nc = child!(c);
            k.tag(13);
            k.ptr(&na);
            k.ptr(&nb);
            k.ptr(&nc);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Revert(na, nb, nc))
            }
        }
        EvmExpr::ReturnOp(a, b, c) => {
            let na = child!(a);
            let nb = child!(b);
            let nc = child!(c);
            k.tag(14);
            k.ptr(&na);
            k.ptr(&nb);
            k.ptr(&nc);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&na, a) && Rc::ptr_eq(&nb, b) && Rc::ptr_eq(&nc, c) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::ReturnOp(na, nb, nc))
            }
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            let na = child!(a);
            let nb = child!(b);
            let nc = child!(c);
            let nd = child!(d);
            let ne = child!(e);
            let nf = child!(f);
            let ng = child!(g);
            k.tag(15);
            k.ptr(&na);
            k.ptr(&nb);
            k.ptr(&nc);
            k.ptr(&nd);
            k.ptr(&ne);
            k.ptr(&nf);
            k.ptr(&ng);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::new(EvmExpr::ExtCall(na, nb, nc, nd, ne, nf, ng))
        }
        EvmExpr::Call(name, args) => {
            let new_args: Vec<_> = args.iter().map(|a| child!(a)).collect();
            k.tag(16);
            k.str(name);
            for a in &new_args {
                k.ptr(a);
            }
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::new(EvmExpr::Call(name.clone(), new_args))
        }
        EvmExpr::Selector(s) => {
            k.tag(17);
            k.str(s);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::LetBind(name, value, body) => {
            let nv = child!(value);
            let nb = child!(body);
            k.tag(18);
            k.str(name);
            k.ptr(&nv);
            k.ptr(&nb);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nv, value) && Rc::ptr_eq(&nb, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::LetBind(name.clone(), nv, nb))
            }
        }
        EvmExpr::Var(name) => {
            k.tag(19);
            k.str(name);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::VarStore(name, val) => {
            let nv = child!(val);
            k.tag(20);
            k.str(name);
            k.ptr(&nv);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nv, val) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::VarStore(name.clone(), nv))
            }
        }
        EvmExpr::Drop(name) => {
            k.tag(21);
            k.str(name);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            let nb = child!(body);
            k.tag(22);
            k.str(name);
            key_for_type(&mut k, in_ty);
            key_for_type(&mut k, out_ty);
            k.ptr(&nb);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nb, body) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::Function(
                    name.clone(),
                    in_ty.clone(),
                    out_ty.clone(),
                    nb,
                ))
            }
        }
        EvmExpr::StorageField(name, slot, ty) => {
            k.tag(23);
            k.str(name);
            k.usize(*slot);
            key_for_type(&mut k, ty);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let new_inputs: Vec<_> = inputs.iter().map(|a| child!(a)).collect();
            k.tag(24);
            k.str(hex);
            k.i32(*num_outputs);
            for a in &new_inputs {
                k.ptr(a);
            }
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::new(EvmExpr::InlineAsm(new_inputs, hex.clone(), *num_outputs))
        }
        EvmExpr::MemRegion(id, size) => {
            k.tag(25);
            k.i64(*id);
            k.i64(*size);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            Rc::clone(expr)
        }
        EvmExpr::DynAlloc(size) => {
            let ns = child!(size);
            k.tag(26);
            k.ptr(&ns);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&ns, size) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::DynAlloc(ns))
            }
        }
        EvmExpr::AllocRegion(id, num_fields, is_dynamic) => {
            let nf = child!(num_fields);
            k.tag(27);
            k.i64(*id);
            k.ptr(&nf);
            k.u8(*is_dynamic as u8);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nf, num_fields) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::AllocRegion(*id, nf, *is_dynamic))
            }
        }
        EvmExpr::RegionStore(id, field_idx, val, state) => {
            let nv = child!(val);
            let ns = child!(state);
            k.tag(28);
            k.i64(*id);
            k.i64(*field_idx);
            k.ptr(&nv);
            k.ptr(&ns);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&nv, val) && Rc::ptr_eq(&ns, state) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::RegionStore(*id, *field_idx, nv, ns))
            }
        }
        EvmExpr::RegionLoad(id, field_idx, state) => {
            let ns = child!(state);
            k.tag(29);
            k.i64(*id);
            k.i64(*field_idx);
            k.ptr(&ns);
            if let Some(cached) = cache.get(&k) {
                return Rc::clone(cached);
            }
            if Rc::ptr_eq(&ns, state) {
                Rc::clone(expr)
            } else {
                Rc::new(EvmExpr::RegionLoad(*id, *field_idx, ns))
            }
        }
    };

    cache.insert(k, Rc::clone(&result));
    result
}

/// Count unique IR DAG nodes by variant name.
pub fn ir_stats(expr: &RcExpr) -> IrStats {
    let mut stats = IrStats::default();
    let mut visited = std::collections::HashSet::new();
    ir_stats_dag(expr, &mut stats, 0, &mut visited);
    // Top-level Concat chain breakdown (not DAG-deduped — shows structural layout)
    collect_top_concat_sizes_dag(expr, &mut stats.top_concat_child_sizes, 0);
    stats
}

/// Count unique DAG nodes (Rc pointer identity).
pub fn dag_node_count(expr: &RcExpr) -> usize {
    let mut visited = std::collections::HashSet::new();
    dag_count_rec(expr, &mut visited)
}

fn dag_count_rec(expr: &RcExpr, visited: &mut std::collections::HashSet<usize>) -> usize {
    let ptr = Rc::as_ptr(expr) as usize;
    if !visited.insert(ptr) {
        return 0;
    }
    let mut count = 1usize;
    macro_rules! add {
        ($e:expr) => {
            count += dag_count_rec($e, visited);
        };
    }
    match expr.as_ref() {
        EvmExpr::Arg(..)
        | EvmExpr::Const(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(..)
        | EvmExpr::Drop(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..)
        | EvmExpr::Selector(..) => {}
        EvmExpr::Uop(_, a)
        | EvmExpr::VarStore(_, a)
        | EvmExpr::Get(a, _)
        | EvmExpr::EnvRead(_, a)
        | EvmExpr::DynAlloc(a)
        | EvmExpr::AllocRegion(_, a, _) => {
            add!(a);
        }
        EvmExpr::Bop(_, a, b)
        | EvmExpr::Concat(a, b)
        | EvmExpr::DoWhile(a, b)
        | EvmExpr::EnvRead1(_, a, b) => {
            add!(a);
            add!(b);
        }
        EvmExpr::RegionStore(_, _, a, b) => {
            add!(a);
            add!(b);
        }
        EvmExpr::RegionLoad(_, _, a) => {
            add!(a);
        }
        EvmExpr::LetBind(_, a, b) => {
            add!(a);
            add!(b);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            add!(a);
            add!(b);
            add!(c);
        }
        EvmExpr::If(a, b, c, d) => {
            add!(a);
            add!(b);
            add!(c);
            add!(d);
        }
        EvmExpr::Function(_, _, _, a) => {
            add!(a);
        }
        EvmExpr::Call(_, args) => {
            for a in args {
                add!(a);
            }
        }
        EvmExpr::Log(_, topics, a, b, c) => {
            for t in topics {
                add!(t);
            }
            add!(a);
            add!(b);
            add!(c);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            add!(a);
            add!(b);
            add!(c);
            add!(d);
            add!(e);
            add!(f);
            add!(g);
        }
        EvmExpr::InlineAsm(inputs, _, _) => {
            for a in inputs {
                add!(a);
            }
        }
    }
    count
}

/// Walk the top-level Concat spine and record each child's DAG size + label.
/// Recurses into LetBind bodies and If branches to break down the dispatcher.
fn collect_top_concat_sizes_dag(expr: &RcExpr, out: &mut Vec<(String, usize)>, depth: usize) {
    if depth > 6 {
        out.push((
            format!("{}...(depth limit)", "  ".repeat(depth)),
            dag_node_count(expr),
        ));
        return;
    }
    let indent = "  ".repeat(depth);
    match expr.as_ref() {
        EvmExpr::Concat(a, b) => {
            collect_top_concat_sizes_dag(a, out, depth);
            collect_top_concat_sizes_dag(b, out, depth);
        }
        EvmExpr::LetBind(name, init, body) => {
            out.push((
                format!("{indent}LetBind({name}) init"),
                dag_node_count(init),
            ));
            collect_top_concat_sizes_dag(body, out, depth + 1);
        }
        EvmExpr::If(pred, _inputs, then_body, else_body) => {
            out.push((format!("{indent}If pred"), dag_node_count(pred)));
            let then_count = dag_node_count(then_body);
            let else_count = dag_node_count(else_body);
            if then_count > 100 {
                out.push((format!("{indent}  then:"), then_count));
                collect_top_concat_sizes_dag(then_body, out, depth + 2);
            } else {
                out.push((format!("{indent}  then"), then_count));
            }
            if else_count > 100 {
                out.push((format!("{indent}  else:"), else_count));
                collect_top_concat_sizes_dag(else_body, out, depth + 2);
            } else {
                out.push((format!("{indent}  else"), else_count));
            }
        }
        _ => {
            let label = match expr.as_ref() {
                EvmExpr::ReturnOp(..) => format!("{indent}ReturnOp"),
                EvmExpr::Revert(..) => format!("{indent}Revert"),
                EvmExpr::VarStore(name, _) => format!("{indent}VarStore({name})"),
                EvmExpr::Drop(name) => format!("{indent}Drop({name})"),
                EvmExpr::Empty(..) => format!("{indent}Empty"),
                other => format!("{indent}{:?}", std::mem::discriminant(other)),
            };
            out.push((label, dag_node_count(expr)));
        }
    }
}

/// Accumulated IR statistics.
#[derive(Debug, Default)]
pub struct IrStats {
    /// Count of nodes per variant name
    pub node_counts: HashMap<&'static str, usize>,
    /// Total node count
    pub total_nodes: usize,
    /// Maximum tree depth
    pub max_depth: usize,
    /// Count of LetBind nodes (proxy for variable allocations)
    pub let_binds: usize,
    /// Count of Function nodes
    pub functions: usize,
    /// Count of Call nodes
    pub calls: usize,
    /// Count of Concat nodes (chaining)
    pub concats: usize,
    /// Count of If nodes
    pub ifs: usize,
    /// Count of VarStore nodes
    pub var_stores: usize,
    /// Count of Var nodes (reads)
    pub var_reads: usize,
    /// Count of DynAlloc nodes
    pub dyn_allocs: usize,
    /// Per-variable Var read counts
    pub var_read_names: HashMap<String, usize>,
    /// Per-variable LetBind counts
    pub let_bind_names: HashMap<String, usize>,
    /// Per-variable VarStore counts
    pub var_store_names: HashMap<String, usize>,
    /// Subtree sizes for top-level Concat children (to identify where bulk lives)
    pub top_concat_child_sizes: Vec<(String, usize)>,
}

impl std::fmt::Display for IrStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  total_nodes: {}", self.total_nodes)?;
        writeln!(f, "  max_depth:   {}", self.max_depth)?;
        writeln!(f, "  let_binds:   {}", self.let_binds)?;
        writeln!(f, "  functions:   {}", self.functions)?;
        writeln!(f, "  calls:       {}", self.calls)?;
        writeln!(f, "  concats:     {}", self.concats)?;
        writeln!(f, "  ifs:         {}", self.ifs)?;
        writeln!(f, "  var_stores:  {}", self.var_stores)?;
        writeln!(f, "  var_reads:   {}", self.var_reads)?;
        writeln!(f, "  dyn_allocs:  {}", self.dyn_allocs)?;
        // Top node types by count
        let mut sorted: Vec<_> = self.node_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        writeln!(f, "  top node types:")?;
        for (name, count) in sorted.iter().take(15) {
            writeln!(f, "    {name:20} {count}")?;
        }
        // Top Var reads by name
        let mut var_sorted: Vec<_> = self.var_read_names.iter().collect();
        var_sorted.sort_by(|a, b| b.1.cmp(a.1));
        writeln!(f, "  top Var reads by name:")?;
        for (name, count) in var_sorted.iter().take(20) {
            writeln!(f, "    {name:40} {count}")?;
        }
        // LetBind names
        let mut lb_sorted: Vec<_> = self.let_bind_names.iter().collect();
        lb_sorted.sort_by(|a, b| b.1.cmp(a.1));
        writeln!(f, "  LetBind names:")?;
        for (name, count) in lb_sorted.iter().take(20) {
            writeln!(f, "    {name:40} {count}")?;
        }
        // VarStore names
        let mut vs_sorted: Vec<_> = self.var_store_names.iter().collect();
        vs_sorted.sort_by(|a, b| b.1.cmp(a.1));
        if !vs_sorted.is_empty() {
            writeln!(f, "  VarStore names:")?;
            for (name, count) in vs_sorted.iter().take(20) {
                writeln!(f, "    {name:40} {count}")?;
            }
        }
        // Top concat child sizes
        if !self.top_concat_child_sizes.is_empty() {
            writeln!(f, "  top-level Concat children (label, nodes):")?;
            for (label, size) in &self.top_concat_child_sizes {
                writeln!(f, "    {label:40} {size}")?;
            }
        }
        Ok(())
    }
}

fn ir_stats_dag(
    expr: &RcExpr,
    stats: &mut IrStats,
    depth: usize,
    visited: &mut std::collections::HashSet<usize>,
) {
    let ptr = Rc::as_ptr(expr) as usize;
    if !visited.insert(ptr) {
        return;
    }
    stats.total_nodes += 1;
    if depth > stats.max_depth {
        stats.max_depth = depth;
    }
    let variant_name = match expr.as_ref() {
        EvmExpr::Arg(..) => "Arg",
        EvmExpr::Const(..) => "Const",
        EvmExpr::Empty(..) => "Empty",
        EvmExpr::Bop(op, ..) => {
            let name = match op {
                schema::EvmBinaryOp::Add => "Bop::Add",
                schema::EvmBinaryOp::Sub => "Bop::Sub",
                schema::EvmBinaryOp::Mul => "Bop::Mul",
                schema::EvmBinaryOp::CheckedAdd => "Bop::CheckedAdd",
                schema::EvmBinaryOp::CheckedSub => "Bop::CheckedSub",
                schema::EvmBinaryOp::CheckedMul => "Bop::CheckedMul",
                schema::EvmBinaryOp::SLoad => "Bop::SLoad",
                schema::EvmBinaryOp::MLoad => "Bop::MLoad",
                schema::EvmBinaryOp::Lt => "Bop::Lt",
                schema::EvmBinaryOp::Gt => "Bop::Gt",
                schema::EvmBinaryOp::Eq => "Bop::Eq",
                _ => "Bop::Other",
            };
            *stats.node_counts.entry(name).or_default() += 1;
            "Bop"
        }
        EvmExpr::Uop(..) => "Uop",
        EvmExpr::Top(op, ..) => {
            let name = match op {
                schema::EvmTernaryOp::MStore => "Top::MStore",
                schema::EvmTernaryOp::SStore => "Top::SStore",
                schema::EvmTernaryOp::Keccak256 => "Top::Keccak256",
                schema::EvmTernaryOp::Mcopy => "Top::Mcopy",
                _ => "Top::Other",
            };
            *stats.node_counts.entry(name).or_default() += 1;
            "Top"
        }
        EvmExpr::Get(..) => "Get",
        EvmExpr::Concat(..) => {
            stats.concats += 1;
            "Concat"
        }
        EvmExpr::If(..) => {
            stats.ifs += 1;
            "If"
        }
        EvmExpr::DoWhile(..) => "DoWhile",
        EvmExpr::EnvRead(..) => "EnvRead",
        EvmExpr::EnvRead1(..) => "EnvRead1",
        EvmExpr::Log(..) => "Log",
        EvmExpr::Revert(..) => "Revert",
        EvmExpr::ReturnOp(..) => "ReturnOp",
        EvmExpr::ExtCall(..) => "ExtCall",
        EvmExpr::Call(..) => {
            stats.calls += 1;
            "Call"
        }
        EvmExpr::Selector(..) => "Selector",
        EvmExpr::LetBind(name, ..) => {
            stats.let_binds += 1;
            *stats.let_bind_names.entry(name.clone()).or_default() += 1;
            "LetBind"
        }
        EvmExpr::Var(name) => {
            stats.var_reads += 1;
            *stats.var_read_names.entry(name.clone()).or_default() += 1;
            "Var"
        }
        EvmExpr::VarStore(name, ..) => {
            stats.var_stores += 1;
            *stats.var_store_names.entry(name.clone()).or_default() += 1;
            "VarStore"
        }
        EvmExpr::Drop(..) => "Drop",
        EvmExpr::Function(..) => {
            stats.functions += 1;
            "Function"
        }
        EvmExpr::StorageField(..) => "StorageField",
        EvmExpr::InlineAsm(..) => "InlineAsm",
        EvmExpr::MemRegion(..) => "MemRegion",
        EvmExpr::DynAlloc(..) => {
            stats.dyn_allocs += 1;
            "DynAlloc"
        }
        EvmExpr::AllocRegion(..) => "AllocRegion",
        EvmExpr::RegionStore(..) => "RegionStore",
        EvmExpr::RegionLoad(..) => "RegionLoad",
    };
    *stats.node_counts.entry(variant_name).or_default() += 1;

    // Recurse into children
    let d = depth + 1;
    macro_rules! go {
        ($e:expr) => {
            ir_stats_dag($e, stats, d, visited)
        };
    }
    match expr.as_ref() {
        EvmExpr::Arg(..)
        | EvmExpr::Const(..)
        | EvmExpr::Empty(..)
        | EvmExpr::Var(..)
        | EvmExpr::Drop(..)
        | EvmExpr::StorageField(..)
        | EvmExpr::MemRegion(..)
        | EvmExpr::Selector(..) => {}
        EvmExpr::Uop(_, a)
        | EvmExpr::VarStore(_, a)
        | EvmExpr::Get(a, _)
        | EvmExpr::EnvRead(_, a)
        | EvmExpr::DynAlloc(a)
        | EvmExpr::AllocRegion(_, a, _) => go!(a),
        EvmExpr::Bop(_, a, b)
        | EvmExpr::Concat(a, b)
        | EvmExpr::DoWhile(a, b)
        | EvmExpr::EnvRead1(_, a, b) => {
            go!(a);
            go!(b);
        }
        EvmExpr::RegionStore(_, _, a, b) => {
            go!(a);
            go!(b);
        }
        EvmExpr::RegionLoad(_, _, a) => {
            go!(a);
        }
        EvmExpr::LetBind(_, a, b) => {
            go!(a);
            go!(b);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            go!(a);
            go!(b);
            go!(c);
        }
        EvmExpr::If(a, b, c, e) => {
            go!(a);
            go!(b);
            go!(c);
            go!(e);
        }
        EvmExpr::Function(_, _, _, body) => go!(body),
        EvmExpr::Call(_, args) => {
            for a in args {
                go!(a);
            }
        }
        EvmExpr::Log(_, topics, doff, dsz, state) => {
            for t in topics {
                go!(t);
            }
            go!(doff);
            go!(dsz);
            go!(state);
        }
        EvmExpr::ExtCall(a, b, c, e, f, g, h) => {
            go!(a);
            go!(b);
            go!(c);
            go!(e);
            go!(f);
            go!(g);
            go!(h);
        }
        EvmExpr::InlineAsm(inputs, _, _) => {
            for inp in inputs {
                go!(inp);
            }
        }
    }
}

/// Errors that can occur during IR lowering or optimization.
#[derive(Debug, thiserror::Error)]
pub enum IrError {
    /// Error during AST lowering with source span for diagnostics
    #[error("{message}")]
    LoweringSpanned {
        /// The error message
        message: String,
        /// Source location where the error occurred
        span: edge_types::span::Span,
    },
    /// Rich diagnostic error with multiple labels and notes
    #[error("{}", .0.message)]
    Diagnostic(
        /// The full diagnostic with labels, notes, and severity
        edge_diagnostics::Diagnostic,
    ),
    /// Error during egglog execution
    #[error("egglog error: {0}")]
    Egglog(String),
    /// Error during extraction
    #[error("extraction error: {0}")]
    Extraction(String),
    /// Unsupported feature
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// Build the egglog prologue: schema + all optimization rules.
///
/// This string is prepended to every egglog program before execution.
/// The `optimize_for` parameter controls the `:cost` annotations on the schema.
pub fn prologue(optimize_for: OptimizeFor) -> String {
    let schema = costs::schema_with_costs(include_str!("schema.egg"), optimize_for);
    [
        schema.as_str(),
        include_str!("optimizations/peepholes.egg"),
        include_str!("optimizations/arithmetic.egg"),
        include_str!("optimizations/storage.egg"),
        include_str!("optimizations/memory.egg"),
        include_str!("optimizations/region_memory.egg"),
        include_str!("optimizations/dead_code.egg"),
        include_str!("optimizations/range_analysis.egg"),
        include_str!("optimizations/u256_const_fold.egg"),
        include_str!("optimizations/type_propagation.egg"),
        include_str!("optimizations/checked_arithmetic.egg"),
        include_str!("optimizations/cse.egg"),
        include_str!("optimizations/inline.egg"),
        include_str!("optimizations/const_prop.egg"),
        &schedule::rulesets(),
    ]
    .join("\n")
}

/// Create an egglog `EGraph` with the U256 sort registered.
pub fn create_egraph() -> egglog::EGraph {
    let mut egraph = egglog::EGraph::default();
    egraph
        .add_arcsort(
            std::sync::Arc::new(u256_sort::U256Sort),
            egglog::ast::Span::Panic,
        )
        .expect("U256 sort registration");
    egraph
}

/// Lower an AST program to the egglog IR, optionally optimize, and extract.
///
/// This is the main entry point for the IR crate.
///
/// - `optimization_level == 0`: no egglog pass, direct lowering and extraction
/// - `optimization_level >= 1`: run equality saturation with appropriate schedule
pub fn lower_and_optimize(
    program: &edge_ast::Program,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<EvmProgram, IrError> {
    let pipeline_start = std::time::Instant::now();

    // 1. Lower AST -> IR structs
    let t = std::time::Instant::now();
    let mut lowering = to_egglog::AstToEgglog::new();
    let mut ir_program = lowering.lower_program(program)?;
    tracing::debug!("  lowering: {:?}", t.elapsed());
    for c in &ir_program.contracts {
        let dag = dag_node_count(&c.runtime);
        tracing::debug!(
            "    [{}] after lowering: {} DAG nodes, {} fns",
            c.name,
            dag,
            c.internal_functions.len()
        );
        if tracing::enabled!(tracing::Level::TRACE) {
            let stats = ir_stats(&c.runtime);
            tracing::trace!("    [{}] IR stats after lowering:\n{stats}", c.name);
        }
    }

    // 2. Variable optimizations (store-forwarding, dead elim, inlining, const prop)
    // Runs at ALL optimization levels since these are cheap deterministic transforms.
    // Monomorphization only at O1+ (egglog decides inlining); at O0 keep original calls.
    let t = std::time::Instant::now();
    var_opt::optimize_program(&mut ir_program, optimization_level);
    tracing::debug!("  var_opt: {:?}", t.elapsed());
    for c in &ir_program.contracts {
        let dag = dag_node_count(&c.runtime);
        tracing::debug!(
            "    [{}] after var_opt: {} DAG nodes, {} fns",
            c.name,
            dag,
            c.internal_functions.len()
        );
        if tracing::enabled!(tracing::Level::TRACE) {
            let stats = ir_stats(&c.runtime);
            tracing::trace!("    [{}] IR stats after var_opt:\n{stats}", c.name);
        }
    }

    // 3. Storage optimizations:
    //    a) Hoist storage ops out of loops (LICM) — egglog can't model iteration
    let t = std::time::Instant::now();
    storage_hoist::hoist_program(&mut ir_program);
    tracing::debug!("  storage_hoist: {:?}", t.elapsed());

    // 4. Forward RegionStore → RegionLoad in straight-line code.
    // Walks IR in program order, forwarding known field values through
    // struct field access. Enables compile-time resolution of Vec len/cap.
    let t = std::time::Instant::now();
    region_forward::forward_region_stores_program(&mut ir_program, &lowering.region_var_map);
    tracing::debug!("  region_forward: {:?}", t.elapsed());

    // 5. Resolve symbolic MemRegion nodes to concrete offsets.
    // Runs before egglog so that Add(Const, Const) patterns from
    // region+field offsets get folded by egglog's constant folding.
    let t = std::time::Instant::now();
    mem_region::assign_program_offsets(&mut ir_program, &lowering.region_var_map);
    tracing::debug!("  mem_region: {:?}", t.elapsed());

    if optimization_level == 0 {
        // Resolve RegionStore/RegionLoad → MStore/MLoad (no egglog to forward through)
        let t = std::time::Instant::now();
        mem_region::resolve_regions_post_egglog(&mut ir_program, &lowering.region_var_map);
        tracing::debug!("  resolve_regions: {:?}", t.elapsed());

        //    b) At O0 only: forward SStore→SLoad in straight-line code (no egglog)
        let t = std::time::Instant::now();
        storage_hoist::forward_stores_program(&mut ir_program);
        tracing::debug!("  forward_stores: {:?}", t.elapsed());

        let t = std::time::Instant::now();
        var_opt::tighten_drops_program(&mut ir_program);
        tracing::debug!("  tighten_drops: {:?}", t.elapsed());

        let t = std::time::Instant::now();
        var_opt::dead_store_elim_program(&mut ir_program);
        tracing::debug!("  dead_store_elim: {:?}", t.elapsed());

        tracing::debug!("  total IR pipeline: {:?}", pipeline_start.elapsed());
        return Ok(ir_program);
    }
    // At O1+, egglog handles SStore→SLoad forwarding via cross-slot rules in storage.egg

    // 2. Optimize each contract's runtime through egglog equality saturation
    let t_egglog_total = std::time::Instant::now();
    let schedule = schedule::make_schedule(optimization_level);
    let mut optimized_contracts = Vec::new();

    // TODO: trying to remove this by fixing `replace_regions` w memoization instead
    // Re-establish Rc sharing broken by var_opt/storage_hoist/mem_region.
    // Must run right before serialization, after all IR transform passes.
    let t = std::time::Instant::now();
    hash_cons_program(&mut ir_program);
    tracing::debug!("  hash_cons: {:?}", t.elapsed());
    for c in &ir_program.contracts {
        let dag = dag_node_count(&c.runtime);
        tracing::debug!("    [{}] after hash_cons: {} DAG nodes", c.name, dag);
    }

    for contract in &ir_program.contracts {
        let t_contract = std::time::Instant::now();

        // DAG-aware serialization: emit shared sub-expressions as egglog let-bindings
        let (shared_lets, runtime_sexp, mut next_id) = sexp::expr_to_sexp_dag(&contract.runtime, 0);

        // Collect immutable variable names for bound propagation in egglog
        let immutable_vars = var_opt::collect_immutable_vars(&contract.runtime);
        let immutable_facts: String = immutable_vars
            .iter()
            .map(|name| format!("(ImmutableVar \"{name}\")\n"))
            .collect();

        // Include internal function definitions in the same egraph so that
        // the inline rule (Call + Function → body) can fire.
        let mut func_lets = String::new();
        for (i, func) in contract.internal_functions.iter().enumerate() {
            let (func_shared, func_sexp, new_next_id) = sexp::expr_to_sexp_dag(func, next_id);
            next_id = new_next_id;
            if !func_shared.is_empty() {
                func_lets.push_str(&func_shared);
                func_lets.push('\n');
            }
            func_lets.push_str(&format!("(let __fn_{i} {func_sexp})\n"));
        }

        let egglog_program = format!(
            "{}\n\n{}\n(let __runtime {})\n{}\n{}\n{}\n\n(extract __runtime)\n",
            prologue(optimize_for),
            shared_lets,
            runtime_sexp,
            func_lets,
            immutable_facts,
            schedule
        );
        let prologue_len = prologue(optimize_for).len();
        tracing::debug!(
            "    [{}] egglog input: {} bytes (prologue: {}, shared_lets: {}, runtime_sexp: {}, func_lets: {}, immutable: {}, schedule: {})",
            contract.name,
            egglog_program.len(),
            prologue_len,
            shared_lets.len(),
            runtime_sexp.len(),
            func_lets.len(),
            immutable_facts.len(),
            schedule.len(),
        );

        let t_egg = std::time::Instant::now();
        let mut egraph = create_egraph();
        egraph.disable_messages(); // skip 32MB string generation
        let _ = egraph
            .parse_and_run_program(None, &egglog_program)
            .map_err(|e| IrError::Egglog(format!("{e}")))?;
        tracing::debug!("    egglog run ({}): {:?}", contract.name, t_egg.elapsed());

        tracing::info!(
            "Optimized contract {} at -O{}",
            contract.name,
            optimization_level
        );

        // Extract directly from egglog's hash-consed TermDag (no string round-trip)
        let t_phase = std::time::Instant::now();
        let report = egraph
            .get_extract_report()
            .as_ref()
            .ok_or_else(|| IrError::Extraction("no extract report from egglog".to_owned()))?;
        let mut optimized_runtime = sexp::extract_report_to_expr(report)?;
        tracing::debug!("      extract_report_to_expr: {:?}", t_phase.elapsed());

        // Check for compile-time-detectable constant overflows in narrow types.
        // This catches overflow revealed by egglog const-folding (e.g. through
        // inlined constants). The lowering-time check catches literal cases with
        // source spans; this is the fallback for optimization-revealed cases.
        let t_phase = std::time::Instant::now();
        let overflow_errors = check_const_overflow(&optimized_runtime);
        if !overflow_errors.is_empty() {
            let mut diag = edge_diagnostics::Diagnostic::error(
                "arithmetic overflow detected after optimization",
            );
            for err in &overflow_errors {
                diag = diag.with_note(err.clone());
            }
            return Err(IrError::Diagnostic(diag));
        }
        tracing::debug!("      check_const_overflow: {:?}", t_phase.elapsed());

        // Post-egglog cleanup: simplify state params and remove dead code
        let t_phase = std::time::Instant::now();
        optimized_runtime = cleanup::cleanup_expr_pub(&optimized_runtime);
        tracing::debug!("      cleanup: {:?}", t_phase.elapsed());

        let t_phase = std::time::Instant::now();
        optimized_runtime = hash_cons_expr(&optimized_runtime);
        tracing::debug!(
            "      post-egglog hash_cons: {:?} (dag={})",
            t_phase.elapsed(),
            dag_node_count(&optimized_runtime)
        );

        // Only keep internal functions still referenced (directly or transitively)
        // by Call nodes in the optimized runtime. Monomorphized functions that
        // were inlined by egglog are no longer needed.
        let t_phase = std::time::Instant::now();
        let mut referenced = collect_call_names(&optimized_runtime);
        // Transitively collect: if a kept function calls another, keep that too
        loop {
            let mut new_names = std::collections::HashSet::new();
            for func in &contract.internal_functions {
                if let EvmExpr::Function(name, _, _, body) = func.as_ref() {
                    if referenced.contains(name.as_str()) {
                        for n in collect_call_names(body) {
                            if !referenced.contains(&n) {
                                new_names.insert(n);
                            }
                        }
                    }
                }
            }
            if new_names.is_empty() {
                break;
            }
            referenced.extend(new_names);
        }
        let mut optimized_functions = Vec::new();
        for func in &contract.internal_functions {
            let name = match func.as_ref() {
                EvmExpr::Function(n, ..) => n,
                _ => continue,
            };
            if !referenced.contains(name.as_str()) {
                continue;
            }
            let (func_shared, func_sexp, _) = sexp::expr_to_sexp_dag(func, 0);
            let func_program = format!(
                "{}\n\n{}\n(let __func {})\n\n{}\n\n(extract __func)\n",
                prologue(optimize_for),
                func_shared,
                func_sexp,
                schedule
            );
            let mut func_egraph = create_egraph();
            func_egraph.disable_messages();
            let _ = func_egraph
                .parse_and_run_program(None, &func_program)
                .map_err(|e| IrError::Egglog(format!("{e}")))?;
            let func_report = func_egraph
                .get_extract_report()
                .as_ref()
                .ok_or_else(|| IrError::Extraction("no extract report from func egglog".to_owned()))?;
            let optimized_func = sexp::extract_report_to_expr(func_report)?;
            let optimized_func = cleanup::cleanup_expr_pub(&optimized_func);
            optimized_functions.push(optimized_func);
        }
        tracing::debug!(
            "      collect+optimize fns: {:?} ({} kept)",
            t_phase.elapsed(),
            optimized_functions.len()
        );

        tracing::debug!(
            "    contract {} total: {:?}",
            contract.name,
            t_contract.elapsed()
        );
        optimized_contracts.push(EvmContract {
            name: contract.name.clone(),
            storage_fields: contract.storage_fields.clone(),
            constructor: Rc::clone(&contract.constructor),
            runtime: optimized_runtime,
            internal_functions: optimized_functions,
            memory_high_water: contract.memory_high_water,
        });
    }
    tracing::debug!("  egglog total: {:?}", t_egglog_total.elapsed());

    let mut result = EvmProgram {
        contracts: optimized_contracts,
        free_functions: ir_program.free_functions,
        warnings: ir_program.warnings,
    };

    // Post-egglog: resolve RegionStore/RegionLoad → MStore/MLoad.
    // These survived into egglog for symbolic forwarding; now lower to concrete memory ops.
    let t = std::time::Instant::now();
    mem_region::resolve_regions_post_egglog(&mut result, &lowering.region_var_map);
    tracing::debug!("  resolve_regions: {:?}", t.elapsed());

    // Post-egglog: forward SStore→SLoad and eliminate dead stores in straight-line code.
    // Egglog's storage-opt rules only handle state-threaded SStore chains, not Concat-chained
    // SStores (which use Arg(StateT) as state). This pass handles the Concat case.
    storage_hoist::forward_stores_program(&mut result);

    let t = std::time::Instant::now();
    var_opt::tighten_drops_program(&mut result);
    tracing::debug!("  tighten_drops: {:?}", t.elapsed());

    // Eliminate dead stores (write-before-write with no intervening read)
    let t = std::time::Instant::now();
    var_opt::dead_store_elim_program(&mut result);
    tracing::debug!("  dead_store_elim: {:?}", t.elapsed());

    // Deduplicate CalldataLoad nodes (hoist repeated loads into LetBind vars)
    let t = std::time::Instant::now();
    var_opt::calldataload_cse_program(&mut result);
    tracing::debug!("  calldataload_cse: {:?}", t.elapsed());

    tracing::debug!("  total IR pipeline: {:?}", pipeline_start.elapsed());

    Ok(result)
}

/// Collect all function names referenced by `Call` nodes in an expression.
fn collect_call_names(expr: &schema::RcExpr) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    collect_call_names_rec(expr, &mut names);
    names
}

fn collect_call_names_rec(expr: &schema::RcExpr, names: &mut std::collections::HashSet<String>) {
    match expr.as_ref() {
        EvmExpr::Call(name, args) => {
            names.insert(name.clone());
            for a in args {
                collect_call_names_rec(a, names);
            }
        }
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
        }
        EvmExpr::Uop(_, a) | EvmExpr::VarStore(_, a) => {
            collect_call_names_rec(a, names);
        }
        EvmExpr::Get(a, _) => collect_call_names_rec(a, names),
        EvmExpr::If(c, i, t, e) => {
            collect_call_names_rec(c, names);
            collect_call_names_rec(i, names);
            collect_call_names_rec(t, names);
            collect_call_names_rec(e, names);
        }
        EvmExpr::LetBind(_, init, body) => {
            collect_call_names_rec(init, names);
            collect_call_names_rec(body, names);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
            collect_call_names_rec(c, names);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                collect_call_names_rec(t, names);
            }
            collect_call_names_rec(d, names);
            collect_call_names_rec(s, names);
            collect_call_names_rec(st, names);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            collect_call_names_rec(a, names);
            collect_call_names_rec(b, names);
            collect_call_names_rec(c, names);
            collect_call_names_rec(d, names);
            collect_call_names_rec(e, names);
            collect_call_names_rec(f, names);
            collect_call_names_rec(g, names);
        }
        EvmExpr::Function(_, _, _, body) => collect_call_names_rec(body, names),
        _ => {}
    }
}

/// Check for constant values that overflow their declared narrow type.
///
/// After egglog optimization + extraction, walk the IR tree and look for
/// `Const(val, UIntT(N))` where `N < 256` and `val >= 2^N`. This detects
/// compile-time-provable overflows like `250u8 + 250u8`.
fn check_const_overflow(expr: &schema::RcExpr) -> Vec<String> {
    let mut errors = Vec::new();
    check_const_overflow_rec(expr, &mut errors);
    errors
}

fn check_const_overflow_rec(expr: &schema::RcExpr, errors: &mut Vec<String>) {
    match expr.as_ref() {
        EvmExpr::Const(val, EvmType::Base(EvmBaseType::UIntT(width)), _) if *width < 256 => {
            let max_val = if *width == 0 {
                0u128
            } else {
                (1u128 << *width) - 1
            };
            let exceeds = match val {
                EvmConstant::SmallInt(n) => {
                    if *n < 0 {
                        true // negative value in unsigned type
                    } else {
                        *n as u128 > max_val
                    }
                }
                EvmConstant::LargeInt(hex) => {
                    // LargeInt always exceeds narrow types (it's > i64::MAX)
                    // unless the hex happens to be small. Parse and check.
                    ruint::aliases::U256::from_str_radix(hex, 16)
                        .map(|v| v > ruint::aliases::U256::from(max_val))
                        .unwrap_or(false)
                }
                _ => false,
            };
            if exceeds {
                errors.push(format!(
                    "constant value {} overflows u{} (max {})",
                    match val {
                        EvmConstant::SmallInt(n) => format!("{n}"),
                        EvmConstant::LargeInt(hex) => format!("0x{hex}"),
                        _ => "?".to_string(),
                    },
                    width,
                    max_val
                ));
            }
        }
        EvmExpr::Const(val, EvmType::Base(EvmBaseType::IntT(width)), _) if *width < 256 => {
            let half = if *width <= 1 {
                1i128
            } else {
                1i128 << (*width - 1)
            };
            let min_val = -half;
            let max_val = half - 1;
            let exceeds = match val {
                EvmConstant::SmallInt(n) => (*n as i128) < min_val || (*n as i128) > max_val,
                _ => false,
            };
            if exceeds {
                errors.push(format!(
                    "constant value {} overflows i{} (range {}..={})",
                    match val {
                        EvmConstant::SmallInt(n) => format!("{n}"),
                        _ => "?".to_string(),
                    },
                    width,
                    min_val,
                    max_val
                ));
            }
        }
        _ => {}
    }

    // Recurse into children
    match expr.as_ref() {
        EvmExpr::Concat(a, b) | EvmExpr::Bop(_, a, b) | EvmExpr::DoWhile(a, b) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
        }
        EvmExpr::Uop(_, a) | EvmExpr::VarStore(_, a) | EvmExpr::Get(a, _) => {
            check_const_overflow_rec(a, errors);
        }
        EvmExpr::If(c, i, t, e) => {
            check_const_overflow_rec(c, errors);
            check_const_overflow_rec(i, errors);
            check_const_overflow_rec(t, errors);
            check_const_overflow_rec(e, errors);
        }
        EvmExpr::LetBind(_, init, body) => {
            check_const_overflow_rec(init, errors);
            check_const_overflow_rec(body, errors);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
            check_const_overflow_rec(c, errors);
        }
        EvmExpr::Log(_, topics, d, s, st) => {
            for t in topics {
                check_const_overflow_rec(t, errors);
            }
            check_const_overflow_rec(d, errors);
            check_const_overflow_rec(s, errors);
            check_const_overflow_rec(st, errors);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(b, errors);
            check_const_overflow_rec(c, errors);
            check_const_overflow_rec(d, errors);
            check_const_overflow_rec(e, errors);
            check_const_overflow_rec(f, errors);
            check_const_overflow_rec(g, errors);
        }
        EvmExpr::EnvRead(_, s) => check_const_overflow_rec(s, errors),
        EvmExpr::EnvRead1(_, a, s) => {
            check_const_overflow_rec(a, errors);
            check_const_overflow_rec(s, errors);
        }
        EvmExpr::Function(_, _, _, body) => check_const_overflow_rec(body, errors),
        EvmExpr::Call(_, args) => {
            for a in args {
                check_const_overflow_rec(a, errors);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prologue_parses_gas() {
        let prologue = prologue(OptimizeFor::Gas);
        assert!(!prologue.is_empty(), "Prologue should not be empty");
        assert!(prologue.contains("EvmExpr"));
        assert!(prologue.contains("EvmBinaryOp"));
        assert!(prologue.contains("peepholes"));
        assert!(prologue.contains(":cost"));
    }

    #[test]
    fn test_prologue_parses_size() {
        let prologue = prologue(OptimizeFor::Size);
        assert!(!prologue.is_empty());
        assert!(prologue.contains(":cost 1)"));
    }

    #[test]
    fn test_egglog_roundtrip_erc20() {
        let source = std::fs::read_to_string("../../examples/erc20.edge").unwrap();
        let mut parser = edge_parser::Parser::new(&source).unwrap();
        let mut ast = parser.parse().unwrap();

        // Import globals (ops, map, etc.) the same way the driver does
        let global_files = [
            "globals/ops",
            "globals/option",
            "globals/result",
            "globals/map",
        ];
        for key in &global_files {
            let path = format!("../../std/{key}.edge");
            if let Ok(src) = std::fs::read_to_string(&path) {
                if let Ok(mut p) = edge_parser::Parser::new(&src) {
                    if let Ok(globals_ast) = p.parse() {
                        // Prepend globals statements
                        let mut new_stmts = globals_ast.stmts;
                        new_stmts.append(&mut ast.stmts);
                        ast.stmts = new_stmts;
                    }
                }
            }
        }

        let mut lowering = to_egglog::AstToEgglog::new();
        let ir_program = lowering.lower_program(&ast).unwrap();

        let contract = &ir_program.contracts[0];
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);
        assert!(!runtime_sexp.is_empty());

        let schedule = schedule::make_schedule(1);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(OptimizeFor::Gas),
            runtime_sexp,
            schedule
        );

        let mut egraph = create_egraph();
        let result = egraph.parse_and_run_program(None, &egglog_program);
        match &result {
            Err(e) => eprintln!("EGGLOG ERROR: {e}"),
            Ok(outputs) => {
                eprintln!("SUCCESS: {} outputs", outputs.len());
                if let Some(last) = outputs.last() {
                    eprintln!("extracted (first 200): {}", &last[..last.len().min(200)]);
                }
            }
        }
        result.unwrap();
    }

    #[test]
    fn test_checked_elision_via_range_analysis() {
        // Verify that range analysis elides CheckedAdd→Add when bounds prove safety.
        // Input: (calldataload(4) & 0xFF) + 1 — max value 256, no overflow possible.
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (Bop (OpCheckedAdd)
                (Bop (OpAnd)
                    (Bop (OpCalldataLoad)
                        (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                        (Arg (Base (StateT)) (InFunction "test")))
                    (Const (SmallInt 255) (Base (UIntT 256)) (InFunction "test")))
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            !extracted.contains("OpCheckedAdd"),
            "CheckedAdd should have been elided to Add, got: {extracted}"
        );
    }

    #[test]
    fn test_checked_not_elided_without_bounds() {
        // Verify CheckedAdd is NOT elided when operands have no tight bounds.
        // Input: calldataload(4) + calldataload(36) — both are full u256 range.
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (Bop (OpCheckedAdd)
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 36) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            extracted.contains("OpCheckedAdd"),
            "CheckedAdd should NOT be elided without tight bounds, got: {extracted}"
        );
    }

    #[test]
    fn test_checked_elision_cascading() {
        // Verify cascading elision: (x & 0xff) + 1 + 2
        // First add: max(255+1)=256, no overflow → elided
        // Second add needs bounds from first result: max(256+2)=258 → elided
        let program = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __masked (Bop (OpAnd)
                (Bop (OpCalldataLoad)
                    (Const (SmallInt 4) (Base (UIntT 256)) (InFunction "test"))
                    (Arg (Base (StateT)) (InFunction "test")))
                (Const (SmallInt 255) (Base (UIntT 256)) (InFunction "test"))))"#,
            r#"(let __plus1 (Bop (OpCheckedAdd) __masked
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))))"#,
            r#"(let __plus3 (Bop (OpCheckedAdd) __plus1
                (Const (SmallInt 2) (Base (UIntT 256)) (InFunction "test"))))"#,
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))\n(extract __plus3)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        assert!(
            !extracted.contains("OpCheckedAdd"),
            "Both CheckedAdds should be elided via cascading bounds, got: {extracted}"
        );
    }

    #[test]
    fn test_const_prop_simple() {
        // LetBind("x", Const(1), LetBind("y", Var("x"), Var("y")))
        // Should flatten to just Const(1)
        let program = format!(
            "{}\n\n{}\n{}\n\n{}\n\n{}\n",
            prologue(OptimizeFor::Gas),
            r#"(let __test (LetBind "x"
                (Const (SmallInt 1) (Base (UIntT 256)) (InFunction "test"))
                (LetBind "y"
                    (Var "x")
                    (Var "y"))))"#,
            "(ImmutableVar \"x\")\n(ImmutableVar \"y\")",
            "(run-schedule
                (saturate (seq (run dead-code) (run range-analysis) (run type-propagation)))
                (repeat 3
                    (seq
                        (run peepholes)
                        (run u256-const-fold)
                        (saturate (seq (run const-prop) (run u256-const-fold)))
                        (saturate (seq (run dead-code) (run range-analysis) (run type-propagation))))))",
            "(extract __test)"
        );

        let mut egraph = create_egraph();
        let outputs = egraph.parse_and_run_program(None, &program).unwrap();
        let extracted = outputs.last().unwrap();

        eprintln!("const_prop result: {extracted}");
        // Should not contain LetBind or Var — everything propagated to Const(1)
        assert!(
            !extracted.contains("LetBind"),
            "LetBinds should be eliminated by const-prop, got: {extracted}"
        );
        assert!(
            !extracted.contains("Var"),
            "Vars should be replaced by const, got: {extracted}"
        );
    }

    #[test]
    fn test_egglog_roundtrip_counter() {
        let source = std::fs::read_to_string("../../examples/counter.edge").unwrap();
        let mut parser = edge_parser::Parser::new(&source).unwrap();
        let ast = parser.parse().unwrap();

        let mut lowering = to_egglog::AstToEgglog::new();
        let ir_program = lowering.lower_program(&ast).unwrap();

        let contract = &ir_program.contracts[0];
        let runtime_sexp = sexp::expr_to_sexp(&contract.runtime);
        eprintln!("sexp length: {}", runtime_sexp.len());
        eprintln!(
            "sexp (first 500): {}",
            &runtime_sexp[..runtime_sexp.len().min(500)]
        );

        let schedule = schedule::make_schedule(1);
        let egglog_program = format!(
            "{}\n\n(let __runtime {})\n\n{}\n\n(extract __runtime)\n",
            prologue(OptimizeFor::Gas),
            runtime_sexp,
            schedule
        );

        let mut egraph = create_egraph();
        let result = egraph.parse_and_run_program(None, &egglog_program);
        match &result {
            Err(e) => eprintln!("EGGLOG ERROR: {e}"),
            Ok(outputs) => {
                eprintln!("SUCCESS: {} outputs", outputs.len());
                if let Some(last) = outputs.last() {
                    eprintln!("extracted (first 200): {}", &last[..last.len().min(200)]);
                }
            }
        }
        result.unwrap();
    }
}
