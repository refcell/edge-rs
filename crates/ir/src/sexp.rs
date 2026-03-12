//! S-expression conversion for the egglog round-trip.
//!
//! Converts between `EvmExpr` and egglog-compatible s-expression strings.
//! Used to insert IR into an egglog `EGraph` and extract optimized results.

use std::{collections::HashMap, rc::Rc};

use crate::{
    schema::{
        EvmBaseType, EvmBinaryOp, EvmConstant, EvmContext, EvmEnvOp, EvmExpr, EvmTernaryOp,
        EvmType, EvmUnaryOp, RcExpr,
    },
    IrError,
};

// ============================================================
// EvmExpr → S-expression string
// ============================================================

/// Convert an `EvmExpr` to an egglog-compatible s-expression string.
pub fn expr_to_sexp(expr: &EvmExpr) -> String {
    match expr {
        EvmExpr::Arg(ty, ctx) => format!("(Arg {} {})", type_sexp(ty), ctx_sexp(ctx)),
        EvmExpr::Const(c, ty, ctx) => {
            format!(
                "(Const {} {} {})",
                const_sexp(c),
                type_sexp(ty),
                ctx_sexp(ctx)
            )
        }
        EvmExpr::Empty(ty, ctx) => format!("(Empty {} {})", type_sexp(ty), ctx_sexp(ctx)),
        EvmExpr::Bop(op, l, r) => {
            format!(
                "(Bop {} {} {})",
                binop_sexp(op),
                expr_to_sexp(l),
                expr_to_sexp(r)
            )
        }
        EvmExpr::Uop(op, e) => format!("(Uop {} {})", unop_sexp(op), expr_to_sexp(e)),
        EvmExpr::Top(op, a, b, c) => {
            format!(
                "(Top {} {} {} {})",
                ternop_sexp(op),
                expr_to_sexp(a),
                expr_to_sexp(b),
                expr_to_sexp(c)
            )
        }
        EvmExpr::Get(e, idx) => format!("(Get {} {})", expr_to_sexp(e), idx),
        EvmExpr::Concat(a, b) => {
            format!("(Concat {} {})", expr_to_sexp(a), expr_to_sexp(b))
        }
        EvmExpr::If(cond, inputs, t, e) => {
            format!(
                "(If {} {} {} {})",
                expr_to_sexp(cond),
                expr_to_sexp(inputs),
                expr_to_sexp(t),
                expr_to_sexp(e)
            )
        }
        EvmExpr::DoWhile(inputs, body) => {
            format!("(DoWhile {} {})", expr_to_sexp(inputs), expr_to_sexp(body))
        }
        EvmExpr::EnvRead(op, st) => {
            format!("(EnvRead {} {})", envop_sexp(op), expr_to_sexp(st))
        }
        EvmExpr::EnvRead1(op, arg, st) => {
            format!(
                "(EnvRead1 {} {} {})",
                envop_sexp(op),
                expr_to_sexp(arg),
                expr_to_sexp(st)
            )
        }
        EvmExpr::Log(n, topics, data_offset, data_size, st) => {
            let topics_s = list_to_sexp(topics);
            format!(
                "(Log {} {} {} {} {})",
                n,
                topics_s,
                expr_to_sexp(data_offset),
                expr_to_sexp(data_size),
                expr_to_sexp(st)
            )
        }
        EvmExpr::Revert(off, sz, st) => {
            format!(
                "(Revert {} {} {})",
                expr_to_sexp(off),
                expr_to_sexp(sz),
                expr_to_sexp(st)
            )
        }
        EvmExpr::ReturnOp(off, sz, st) => {
            format!(
                "(ReturnOp {} {} {})",
                expr_to_sexp(off),
                expr_to_sexp(sz),
                expr_to_sexp(st)
            )
        }
        EvmExpr::ExtCall(tgt, val, ao, al, ro, rl, st) => {
            format!(
                "(ExtCall {} {} {} {} {} {} {})",
                expr_to_sexp(tgt),
                expr_to_sexp(val),
                expr_to_sexp(ao),
                expr_to_sexp(al),
                expr_to_sexp(ro),
                expr_to_sexp(rl),
                expr_to_sexp(st)
            )
        }
        EvmExpr::Call(name, args) => {
            let mut list = "(Nil)".to_string();
            for arg in args.iter().rev() {
                list = format!("(Cons {} {})", expr_to_sexp(arg), list);
            }
            format!("(Call \"{name}\" {list})")
        }
        EvmExpr::Selector(sig) => format!("(Selector \"{sig}\")"),
        EvmExpr::LetBind(name, value, body) => {
            format!(
                "(LetBind \"{}\" {} {})",
                name,
                expr_to_sexp(value),
                expr_to_sexp(body)
            )
        }
        EvmExpr::Var(name) => format!("(Var \"{name}\")"),
        EvmExpr::VarStore(name, value) => {
            format!("(VarStore \"{}\" {})", name, expr_to_sexp(value))
        }
        EvmExpr::Drop(name) => format!("(Drop \"{name}\")"),
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            format!(
                "(Function \"{}\" {} {} {})",
                name,
                type_sexp(in_ty),
                type_sexp(out_ty),
                expr_to_sexp(body)
            )
        }
        EvmExpr::StorageField(name, slot, ty) => {
            format!("(StorageField \"{}\" {} {})", name, slot, type_sexp(ty))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let mut list = "(Nil)".to_string();
            for arg in inputs.iter().rev() {
                list = format!("(Cons {} {})", expr_to_sexp(arg), list);
            }
            format!("(InlineAsm {list} \"{hex}\" {num_outputs})")
        }
        EvmExpr::MemRegion(id, size) => format!("(MemRegion {id} {size})"),
        EvmExpr::DynAlloc(size) => format!("(DynAlloc {})", expr_to_sexp(size)),
    }
}

fn const_sexp(c: &EvmConstant) -> String {
    match c {
        EvmConstant::SmallInt(i) => format!("(SmallInt {i})"),
        EvmConstant::LargeInt(s) => format!("(LargeInt \"{s}\")"),
        EvmConstant::Bool(b) => format!("(ConstBool {b})"),
        EvmConstant::Addr(s) => format!("(ConstAddr \"{s}\")"),
    }
}

fn type_sexp(ty: &EvmType) -> String {
    match ty {
        EvmType::Base(b) => format!("(Base {})", basetype_sexp(b)),
        EvmType::TupleT(types) => {
            let list = types.iter().rev().fold("(TLNil)".to_owned(), |acc, t| {
                format!("(TLCons {} {})", basetype_sexp(t), acc)
            });
            format!("(TupleT {list})")
        }
        EvmType::ArrayT(elem, len) => {
            format!("(ArrayT {} {})", basetype_sexp(elem), len)
        }
    }
}

fn basetype_sexp(bt: &EvmBaseType) -> String {
    match bt {
        EvmBaseType::UIntT(n) => format!("(UIntT {n})"),
        EvmBaseType::IntT(n) => format!("(IntT {n})"),
        EvmBaseType::BytesT(n) => format!("(BytesT {n})"),
        EvmBaseType::AddrT => "(AddrT)".to_owned(),
        EvmBaseType::BoolT => "(BoolT)".to_owned(),
        EvmBaseType::UnitT => "(UnitT)".to_owned(),
        EvmBaseType::StateT => "(StateT)".to_owned(),
    }
}

fn ctx_sexp(ctx: &EvmContext) -> String {
    match ctx {
        EvmContext::InFunction(name) => format!("(InFunction \"{name}\")"),
        EvmContext::InBranch(b, pred, input) => {
            format!(
                "(InBranch {} {} {})",
                b,
                expr_to_sexp(pred),
                expr_to_sexp(input)
            )
        }
        EvmContext::InLoop(input, pred) => {
            format!("(InLoop {} {})", expr_to_sexp(input), expr_to_sexp(pred))
        }
    }
}

const fn binop_sexp(op: &EvmBinaryOp) -> &'static str {
    match op {
        EvmBinaryOp::Add => "(OpAdd)",
        EvmBinaryOp::Sub => "(OpSub)",
        EvmBinaryOp::Mul => "(OpMul)",
        EvmBinaryOp::Div => "(OpDiv)",
        EvmBinaryOp::SDiv => "(OpSDiv)",
        EvmBinaryOp::Mod => "(OpMod)",
        EvmBinaryOp::SMod => "(OpSMod)",
        EvmBinaryOp::Exp => "(OpExp)",
        EvmBinaryOp::CheckedAdd => "(OpCheckedAdd)",
        EvmBinaryOp::CheckedSub => "(OpCheckedSub)",
        EvmBinaryOp::CheckedMul => "(OpCheckedMul)",
        EvmBinaryOp::Lt => "(OpLt)",
        EvmBinaryOp::Gt => "(OpGt)",
        EvmBinaryOp::SLt => "(OpSLt)",
        EvmBinaryOp::SGt => "(OpSGt)",
        EvmBinaryOp::Eq => "(OpEq)",
        EvmBinaryOp::And => "(OpAnd)",
        EvmBinaryOp::Or => "(OpOr)",
        EvmBinaryOp::Xor => "(OpXor)",
        EvmBinaryOp::Shl => "(OpShl)",
        EvmBinaryOp::Shr => "(OpShr)",
        EvmBinaryOp::Sar => "(OpSar)",
        EvmBinaryOp::Byte => "(OpByte)",
        EvmBinaryOp::LogAnd => "(OpLogAnd)",
        EvmBinaryOp::LogOr => "(OpLogOr)",
        EvmBinaryOp::SLoad => "(OpSLoad)",
        EvmBinaryOp::TLoad => "(OpTLoad)",
        EvmBinaryOp::MLoad => "(OpMLoad)",
        EvmBinaryOp::CalldataLoad => "(OpCalldataLoad)",
    }
}

const fn unop_sexp(op: &EvmUnaryOp) -> &'static str {
    match op {
        EvmUnaryOp::IsZero => "(OpIsZero)",
        EvmUnaryOp::Not => "(OpNot)",
        EvmUnaryOp::Neg => "(OpNeg)",
        EvmUnaryOp::SignExtend => "(OpSignExtend)",
        EvmUnaryOp::Clz => "(OpClz)",
    }
}

const fn ternop_sexp(op: &EvmTernaryOp) -> &'static str {
    match op {
        EvmTernaryOp::SStore => "(OpSStore)",
        EvmTernaryOp::TStore => "(OpTStore)",
        EvmTernaryOp::MStore => "(OpMStore)",
        EvmTernaryOp::MStore8 => "(OpMStore8)",
        EvmTernaryOp::Keccak256 => "(OpKeccak256)",
        EvmTernaryOp::Select => "(OpSelect)",
        EvmTernaryOp::CalldataCopy => "(OpCalldataCopy)",
        EvmTernaryOp::Mcopy => "(OpMcopy)",
    }
}

const fn envop_sexp(op: &EvmEnvOp) -> &'static str {
    match op {
        EvmEnvOp::Caller => "(EnvCaller)",
        EvmEnvOp::CallValue => "(EnvCallValue)",
        EvmEnvOp::CallDataSize => "(EnvCallDataSize)",
        EvmEnvOp::Origin => "(EnvOrigin)",
        EvmEnvOp::GasPrice => "(EnvGasPrice)",
        EvmEnvOp::BlockHash => "(EnvBlockHash)",
        EvmEnvOp::Coinbase => "(EnvCoinbase)",
        EvmEnvOp::Timestamp => "(EnvTimestamp)",
        EvmEnvOp::Number => "(EnvNumber)",
        EvmEnvOp::GasLimit => "(EnvGasLimit)",
        EvmEnvOp::ChainId => "(EnvChainId)",
        EvmEnvOp::SelfBalance => "(EnvSelfBalance)",
        EvmEnvOp::BaseFee => "(EnvBaseFee)",
        EvmEnvOp::Gas => "(EnvGas)",
        EvmEnvOp::Address => "(EnvAddress)",
        EvmEnvOp::Balance => "(EnvBalance)",
        EvmEnvOp::CodeSize => "(EnvCodeSize)",
        EvmEnvOp::ReturnDataSize => "(EnvReturnDataSize)",
    }
}

fn list_to_sexp(exprs: &[RcExpr]) -> String {
    exprs.iter().rev().fold("(Nil)".to_owned(), |acc, e| {
        format!("(Cons {} {})", expr_to_sexp(e), acc)
    })
}

// ============================================================
// DAG-aware S-expression conversion
// ============================================================
//
// The IR is a DAG (via Rc sharing), but expr_to_sexp expands it into a tree.
// For Vec<T> with 5 push calls, this blows up from ~1,500 DAG nodes to
// ~867,000 expanded nodes (33 MB of s-expression text).
//
// This module detects shared Rc nodes and emits egglog `(let __sN ...)` bindings
// for them. Subsequent references use the binding name instead of re-expanding.
// This keeps the s-expression size proportional to the DAG, not the expanded tree.

use std::collections::HashSet;

/// Convert an `RcExpr` DAG to egglog s-expressions with `let`-bindings for shared nodes.
///
/// Returns `(let_bindings, main_expr)` where `let_bindings` contains
/// `(let __sN <expr>)` declarations in dependency order.
///
/// `id_offset` ensures unique names across multiple calls within the same
/// egglog program (e.g., runtime + internal functions). Returns the next
/// available ID after this call.
pub fn expr_to_sexp_dag(expr: &RcExpr, id_offset: usize) -> (String, String, usize) {
    // Pass 1: count how many parent edges each Rc node has (DAG-aware traversal)
    let mut ref_counts: HashMap<usize, usize> = HashMap::new();
    let mut visited: HashSet<usize> = HashSet::new();
    count_refs_dag(expr, &mut ref_counts, &mut visited);

    // Check if any node is referenced more than once
    let has_sharing = ref_counts.values().any(|&c| c > 1);
    if !has_sharing {
        return (String::new(), expr_to_sexp(expr), id_offset);
    }

    // Pass 2: serialize with let-bindings for shared nodes
    let mut ctx = DagSexpCtx {
        ref_counts,
        named: HashMap::new(),
        let_bindings: Vec::new(),
        next_id: id_offset,
    };
    let main = dag_sexp_rec(expr, &mut ctx);
    (ctx.let_bindings.join("\n"), main, ctx.next_id)
}

struct DagSexpCtx {
    ref_counts: HashMap<usize, usize>,
    named: HashMap<usize, String>,
    let_bindings: Vec<String>,
    next_id: usize,
}

fn ptr_id(expr: &RcExpr) -> usize {
    Rc::as_ptr(expr) as usize
}

/// Count references to each Rc node. Recurses into children only once per node.
fn count_refs_dag(expr: &RcExpr, counts: &mut HashMap<usize, usize>, visited: &mut HashSet<usize>) {
    let id = ptr_id(expr);
    *counts.entry(id).or_default() += 1;
    if !visited.insert(id) {
        return;
    }
    macro_rules! visit {
        ($e:expr) => {
            count_refs_dag($e, counts, visited);
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
        | EvmExpr::DynAlloc(a) => {
            visit!(a);
        }
        EvmExpr::Bop(_, a, b)
        | EvmExpr::Concat(a, b)
        | EvmExpr::DoWhile(a, b)
        | EvmExpr::EnvRead1(_, a, b) => {
            visit!(a);
            visit!(b);
        }
        EvmExpr::LetBind(_, a, b) => {
            visit!(a);
            visit!(b);
        }
        EvmExpr::Top(_, a, b, c) | EvmExpr::Revert(a, b, c) | EvmExpr::ReturnOp(a, b, c) => {
            visit!(a);
            visit!(b);
            visit!(c);
        }
        EvmExpr::If(a, b, c, d) => {
            visit!(a);
            visit!(b);
            visit!(c);
            visit!(d);
        }
        EvmExpr::Function(_, _, _, a) => {
            visit!(a);
        }
        EvmExpr::Call(_, args) => {
            for a in args {
                visit!(a);
            }
        }
        EvmExpr::Log(_, topics, a, b, c) => {
            for t in topics {
                visit!(t);
            }
            visit!(a);
            visit!(b);
            visit!(c);
        }
        EvmExpr::ExtCall(a, b, c, d, e, f, g) => {
            visit!(a);
            visit!(b);
            visit!(c);
            visit!(d);
            visit!(e);
            visit!(f);
            visit!(g);
        }
        EvmExpr::InlineAsm(inputs, _, _) => {
            for a in inputs {
                visit!(a);
            }
        }
    }
}

/// Serialize a node, emitting let-bindings for shared sub-expressions.
fn dag_sexp_rec(expr: &RcExpr, ctx: &mut DagSexpCtx) -> String {
    let id = ptr_id(expr);

    // If this shared node was already serialized, return its name
    if let Some(name) = ctx.named.get(&id) {
        return name.clone();
    }

    // Serialize the node itself (recurse into children via dag_sexp_rec)
    let sexp = dag_sexp_node(expr, ctx);

    // If multiply-referenced, emit a let-binding and return the name
    let is_shared = ctx.ref_counts.get(&id).copied().unwrap_or(0) > 1;
    if is_shared {
        // Don't share trivial leaf nodes (saves let-binding overhead)
        let is_leaf = matches!(
            expr.as_ref(),
            EvmExpr::Arg(..)
                | EvmExpr::Const(..)
                | EvmExpr::Empty(..)
                | EvmExpr::Var(..)
                | EvmExpr::Drop(..)
                | EvmExpr::Selector(..)
                | EvmExpr::MemRegion(..)
                | EvmExpr::StorageField(..)
        );
        if !is_leaf {
            let name = format!("__s{}", ctx.next_id);
            ctx.next_id += 1;
            ctx.named.insert(id, name.clone());
            ctx.let_bindings.push(format!("(let {name} {sexp})"));
            return name;
        }
    }

    sexp
}

/// Serialize a single node's s-expression, using dag_sexp_rec for children.
fn dag_sexp_node(expr: &RcExpr, ctx: &mut DagSexpCtx) -> String {
    match expr.as_ref() {
        EvmExpr::Arg(ty, c) => format!("(Arg {} {})", type_sexp(ty), ctx_sexp(c)),
        EvmExpr::Const(c, ty, cx) => {
            format!(
                "(Const {} {} {})",
                const_sexp(c),
                type_sexp(ty),
                ctx_sexp(cx)
            )
        }
        EvmExpr::Empty(ty, c) => format!("(Empty {} {})", type_sexp(ty), ctx_sexp(c)),
        EvmExpr::Bop(op, l, r) => {
            format!(
                "(Bop {} {} {})",
                binop_sexp(op),
                dag_sexp_rec(l, ctx),
                dag_sexp_rec(r, ctx)
            )
        }
        EvmExpr::Uop(op, e) => format!("(Uop {} {})", unop_sexp(op), dag_sexp_rec(e, ctx)),
        EvmExpr::Top(op, a, b, c) => {
            format!(
                "(Top {} {} {} {})",
                ternop_sexp(op),
                dag_sexp_rec(a, ctx),
                dag_sexp_rec(b, ctx),
                dag_sexp_rec(c, ctx)
            )
        }
        EvmExpr::Get(e, idx) => format!("(Get {} {})", dag_sexp_rec(e, ctx), idx),
        EvmExpr::Concat(a, b) => {
            format!("(Concat {} {})", dag_sexp_rec(a, ctx), dag_sexp_rec(b, ctx))
        }
        EvmExpr::If(cond, inputs, t, e) => {
            format!(
                "(If {} {} {} {})",
                dag_sexp_rec(cond, ctx),
                dag_sexp_rec(inputs, ctx),
                dag_sexp_rec(t, ctx),
                dag_sexp_rec(e, ctx)
            )
        }
        EvmExpr::DoWhile(inputs, body) => {
            format!(
                "(DoWhile {} {})",
                dag_sexp_rec(inputs, ctx),
                dag_sexp_rec(body, ctx)
            )
        }
        EvmExpr::EnvRead(op, st) => {
            format!("(EnvRead {} {})", envop_sexp(op), dag_sexp_rec(st, ctx))
        }
        EvmExpr::EnvRead1(op, arg, st) => {
            format!(
                "(EnvRead1 {} {} {})",
                envop_sexp(op),
                dag_sexp_rec(arg, ctx),
                dag_sexp_rec(st, ctx)
            )
        }
        EvmExpr::Log(n, topics, data_offset, data_size, st) => {
            let topics_s = dag_list_to_sexp(topics, ctx);
            format!(
                "(Log {} {} {} {} {})",
                n,
                topics_s,
                dag_sexp_rec(data_offset, ctx),
                dag_sexp_rec(data_size, ctx),
                dag_sexp_rec(st, ctx)
            )
        }
        EvmExpr::Revert(off, sz, st) => {
            format!(
                "(Revert {} {} {})",
                dag_sexp_rec(off, ctx),
                dag_sexp_rec(sz, ctx),
                dag_sexp_rec(st, ctx)
            )
        }
        EvmExpr::ReturnOp(off, sz, st) => {
            format!(
                "(ReturnOp {} {} {})",
                dag_sexp_rec(off, ctx),
                dag_sexp_rec(sz, ctx),
                dag_sexp_rec(st, ctx)
            )
        }
        EvmExpr::ExtCall(tgt, val, ao, al, ro, rl, st) => {
            format!(
                "(ExtCall {} {} {} {} {} {} {})",
                dag_sexp_rec(tgt, ctx),
                dag_sexp_rec(val, ctx),
                dag_sexp_rec(ao, ctx),
                dag_sexp_rec(al, ctx),
                dag_sexp_rec(ro, ctx),
                dag_sexp_rec(rl, ctx),
                dag_sexp_rec(st, ctx)
            )
        }
        EvmExpr::Call(name, args) => {
            let list = dag_list_to_sexp(args, ctx);
            format!("(Call \"{name}\" {list})")
        }
        EvmExpr::Selector(sig) => format!("(Selector \"{sig}\")"),
        EvmExpr::LetBind(name, value, body) => {
            format!(
                "(LetBind \"{}\" {} {})",
                name,
                dag_sexp_rec(value, ctx),
                dag_sexp_rec(body, ctx)
            )
        }
        EvmExpr::Var(name) => format!("(Var \"{name}\")"),
        EvmExpr::VarStore(name, value) => {
            format!("(VarStore \"{}\" {})", name, dag_sexp_rec(value, ctx))
        }
        EvmExpr::Drop(name) => format!("(Drop \"{name}\")"),
        EvmExpr::Function(name, in_ty, out_ty, body) => {
            format!(
                "(Function \"{}\" {} {} {})",
                name,
                type_sexp(in_ty),
                type_sexp(out_ty),
                dag_sexp_rec(body, ctx)
            )
        }
        EvmExpr::StorageField(name, slot, ty) => {
            format!("(StorageField \"{}\" {} {})", name, slot, type_sexp(ty))
        }
        EvmExpr::InlineAsm(inputs, hex, num_outputs) => {
            let list = dag_list_to_sexp(inputs, ctx);
            format!("(InlineAsm {list} \"{hex}\" {num_outputs})")
        }
        EvmExpr::MemRegion(id, size) => format!("(MemRegion {id} {size})"),
        EvmExpr::DynAlloc(size) => format!("(DynAlloc {})", dag_sexp_rec(size, ctx)),
    }
}

fn dag_list_to_sexp(exprs: &[RcExpr], ctx: &mut DagSexpCtx) -> String {
    exprs.iter().rev().fold("(Nil)".to_owned(), |acc, e| {
        format!("(Cons {} {})", dag_sexp_rec(e, ctx), acc)
    })
}

// ============================================================
// S-expression string → EvmExpr
// ============================================================

/// A parsed s-expression token.
#[derive(Debug, Clone, PartialEq)]
enum Sexp {
    Atom(String),
    List(Vec<Self>),
}

/// Tokenize and parse an s-expression string.
fn parse_sexp(input: &str) -> Result<Sexp, IrError> {
    let tokens = tokenize(input)?;
    let (sexp, rest) = parse_tokens(&tokens)?;
    if !rest.is_empty() {
        return Err(IrError::Extraction(format!(
            "trailing tokens after s-expression: {rest:?}"
        )));
    }
    Ok(sexp)
}

fn tokenize(input: &str) -> Result<Vec<String>, IrError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '(' => {
                tokens.push("(".to_owned());
                chars.next();
            }
            ')' => {
                tokens.push(")".to_owned());
                chars.next();
            }
            '"' => {
                chars.next(); // skip opening quote
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => {
                            if let Some(escaped) = chars.next() {
                                s.push(escaped);
                            }
                        }
                        Some(ch) => s.push(ch),
                        None => return Err(IrError::Extraction("unterminated string".to_owned())),
                    }
                }
                // Store with quotes to distinguish from identifiers
                tokens.push(format!("\"{s}\""));
            }
            _ => {
                let mut atom = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '(' || ch == ')' || ch.is_whitespace() {
                        break;
                    }
                    atom.push(ch);
                    chars.next();
                }
                tokens.push(atom);
            }
        }
    }
    Ok(tokens)
}

fn parse_tokens(tokens: &[String]) -> Result<(Sexp, &[String]), IrError> {
    if tokens.is_empty() {
        return Err(IrError::Extraction("unexpected end of input".to_owned()));
    }
    if tokens[0] == "(" {
        let mut items = Vec::new();
        let mut rest = &tokens[1..];
        while !rest.is_empty() && rest[0] != ")" {
            let (item, r) = parse_tokens(rest)?;
            items.push(item);
            rest = r;
        }
        if rest.is_empty() {
            return Err(IrError::Extraction("unclosed parenthesis".to_owned()));
        }
        Ok((Sexp::List(items), &rest[1..]))
    } else {
        Ok((Sexp::Atom(tokens[0].clone()), &tokens[1..]))
    }
}

/// Convert a parsed s-expression back to an `EvmExpr`.
pub fn sexp_to_expr(input: &str) -> Result<RcExpr, IrError> {
    let sexp = parse_sexp(input)?;
    sexp_to_evm_expr(&sexp)
}

fn sexp_to_evm_expr(sexp: &Sexp) -> Result<RcExpr, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "Arg" => {
                    let ty = sexp_to_type(&items[1])?;
                    let ctx = sexp_to_ctx(&items[2])?;
                    Ok(Rc::new(EvmExpr::Arg(ty, ctx)))
                }
                "Const" => {
                    let c = sexp_to_const(&items[1])?;
                    let ty = sexp_to_type(&items[2])?;
                    let ctx = sexp_to_ctx(&items[3])?;
                    Ok(Rc::new(EvmExpr::Const(c, ty, ctx)))
                }
                "Empty" => {
                    let ty = sexp_to_type(&items[1])?;
                    let ctx = sexp_to_ctx(&items[2])?;
                    Ok(Rc::new(EvmExpr::Empty(ty, ctx)))
                }
                "Bop" => {
                    let op = sexp_to_binop(&items[1])?;
                    let l = sexp_to_evm_expr(&items[2])?;
                    let r = sexp_to_evm_expr(&items[3])?;
                    Ok(Rc::new(EvmExpr::Bop(op, l, r)))
                }
                "Uop" => {
                    let op = sexp_to_unop(&items[1])?;
                    let e = sexp_to_evm_expr(&items[2])?;
                    Ok(Rc::new(EvmExpr::Uop(op, e)))
                }
                "Top" => {
                    let op = sexp_to_ternop(&items[1])?;
                    let a = sexp_to_evm_expr(&items[2])?;
                    let b = sexp_to_evm_expr(&items[3])?;
                    let c = sexp_to_evm_expr(&items[4])?;
                    Ok(Rc::new(EvmExpr::Top(op, a, b, c)))
                }
                "Get" => {
                    let e = sexp_to_evm_expr(&items[1])?;
                    let idx = atom_i64(&items[2])? as usize;
                    Ok(Rc::new(EvmExpr::Get(e, idx)))
                }
                "Concat" => {
                    let a = sexp_to_evm_expr(&items[1])?;
                    let b = sexp_to_evm_expr(&items[2])?;
                    Ok(Rc::new(EvmExpr::Concat(a, b)))
                }
                "If" => {
                    let cond = sexp_to_evm_expr(&items[1])?;
                    let inputs = sexp_to_evm_expr(&items[2])?;
                    let t = sexp_to_evm_expr(&items[3])?;
                    let e = sexp_to_evm_expr(&items[4])?;
                    Ok(Rc::new(EvmExpr::If(cond, inputs, t, e)))
                }
                "DoWhile" => {
                    let inputs = sexp_to_evm_expr(&items[1])?;
                    let body = sexp_to_evm_expr(&items[2])?;
                    Ok(Rc::new(EvmExpr::DoWhile(inputs, body)))
                }
                "EnvRead" => {
                    let op = sexp_to_envop(&items[1])?;
                    let st = sexp_to_evm_expr(&items[2])?;
                    Ok(Rc::new(EvmExpr::EnvRead(op, st)))
                }
                "EnvRead1" => {
                    let op = sexp_to_envop(&items[1])?;
                    let arg = sexp_to_evm_expr(&items[2])?;
                    let st = sexp_to_evm_expr(&items[3])?;
                    Ok(Rc::new(EvmExpr::EnvRead1(op, arg, st)))
                }
                "Log" => {
                    let n = atom_i64(&items[1])? as usize;
                    let topics = sexp_to_list(&items[2])?;
                    let data_offset = sexp_to_evm_expr(&items[3])?;
                    let data_size = sexp_to_evm_expr(&items[4])?;
                    let st = sexp_to_evm_expr(&items[5])?;
                    Ok(Rc::new(EvmExpr::Log(n, topics, data_offset, data_size, st)))
                }
                "Revert" => {
                    let off = sexp_to_evm_expr(&items[1])?;
                    let sz = sexp_to_evm_expr(&items[2])?;
                    let st = sexp_to_evm_expr(&items[3])?;
                    Ok(Rc::new(EvmExpr::Revert(off, sz, st)))
                }
                "ReturnOp" => {
                    let off = sexp_to_evm_expr(&items[1])?;
                    let sz = sexp_to_evm_expr(&items[2])?;
                    let st = sexp_to_evm_expr(&items[3])?;
                    Ok(Rc::new(EvmExpr::ReturnOp(off, sz, st)))
                }
                "ExtCall" => {
                    let tgt = sexp_to_evm_expr(&items[1])?;
                    let val = sexp_to_evm_expr(&items[2])?;
                    let ao = sexp_to_evm_expr(&items[3])?;
                    let al = sexp_to_evm_expr(&items[4])?;
                    let ro = sexp_to_evm_expr(&items[5])?;
                    let rl = sexp_to_evm_expr(&items[6])?;
                    let st = sexp_to_evm_expr(&items[7])?;
                    Ok(Rc::new(EvmExpr::ExtCall(tgt, val, ao, al, ro, rl, st)))
                }
                "Call" => {
                    let name = atom_string(&items[1])?;
                    let args = sexp_to_list(&items[2])?;
                    Ok(Rc::new(EvmExpr::Call(name, args)))
                }
                "Selector" => {
                    let sig = atom_string(&items[1])?;
                    Ok(Rc::new(EvmExpr::Selector(sig)))
                }
                "LetBind" => {
                    let name = atom_string(&items[1])?;
                    let value = sexp_to_evm_expr(&items[2])?;
                    let body = sexp_to_evm_expr(&items[3])?;
                    Ok(Rc::new(EvmExpr::LetBind(name, value, body)))
                }
                "Var" => {
                    let name = atom_string(&items[1])?;
                    Ok(Rc::new(EvmExpr::Var(name)))
                }
                "VarStore" => {
                    let name = atom_string(&items[1])?;
                    let value = sexp_to_evm_expr(&items[2])?;
                    Ok(Rc::new(EvmExpr::VarStore(name, value)))
                }
                "Drop" => {
                    let name = atom_string(&items[1])?;
                    Ok(Rc::new(EvmExpr::Drop(name)))
                }
                "Function" => {
                    let name = atom_string(&items[1])?;
                    let in_ty = sexp_to_type(&items[2])?;
                    let out_ty = sexp_to_type(&items[3])?;
                    let body = sexp_to_evm_expr(&items[4])?;
                    Ok(Rc::new(EvmExpr::Function(name, in_ty, out_ty, body)))
                }
                "StorageField" => {
                    let name = atom_string(&items[1])?;
                    let slot = atom_i64(&items[2])? as usize;
                    let ty = sexp_to_type(&items[3])?;
                    Ok(Rc::new(EvmExpr::StorageField(name, slot, ty)))
                }
                "InlineAsm" => {
                    let inputs = sexp_to_list(&items[1])?;
                    let hex = atom_string(&items[2])?;
                    let num_outputs = atom_i64(&items[3])? as i32;
                    Ok(Rc::new(EvmExpr::InlineAsm(inputs, hex, num_outputs)))
                }
                "MemRegion" => {
                    let id = atom_i64(&items[1])?;
                    let size = atom_i64(&items[2])?;
                    Ok(Rc::new(EvmExpr::MemRegion(id, size)))
                }
                "DynAlloc" => {
                    let size = sexp_to_evm_expr(&items[1])?;
                    Ok(Rc::new(EvmExpr::DynAlloc(size)))
                }
                other => Err(IrError::Extraction(format!(
                    "unknown expression constructor: {other}"
                ))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected s-expression list, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_const(sexp: &Sexp) -> Result<EvmConstant, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "SmallInt" => Ok(EvmConstant::SmallInt(atom_i64(&items[1])?)),
                "LargeInt" => Ok(EvmConstant::LargeInt(atom_string(&items[1])?)),
                "ConstBool" => Ok(EvmConstant::Bool(atom_bool(&items[1])?)),
                "ConstAddr" => Ok(EvmConstant::Addr(atom_string(&items[1])?)),
                other => Err(IrError::Extraction(format!("unknown constant: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected constant, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_type(sexp: &Sexp) -> Result<EvmType, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "Base" => Ok(EvmType::Base(sexp_to_basetype(&items[1])?)),
                "TupleT" => {
                    let types = sexp_to_type_list(&items[1])?;
                    Ok(EvmType::TupleT(types))
                }
                "ArrayT" => {
                    let elem = sexp_to_basetype(&items[1])?;
                    let len = atom_str(&items[2])?
                        .parse::<usize>()
                        .map_err(|e| IrError::Extraction(format!("bad array length: {e}")))?;
                    Ok(EvmType::ArrayT(elem, len))
                }
                other => Err(IrError::Extraction(format!("unknown type: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!("expected type, got: {sexp:?}"))),
    }
}

fn sexp_to_basetype(sexp: &Sexp) -> Result<EvmBaseType, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "UIntT" => Ok(EvmBaseType::UIntT(atom_i64(&items[1])? as u16)),
                "IntT" => Ok(EvmBaseType::IntT(atom_i64(&items[1])? as u16)),
                "BytesT" => Ok(EvmBaseType::BytesT(atom_i64(&items[1])? as u8)),
                "AddrT" => Ok(EvmBaseType::AddrT),
                "BoolT" => Ok(EvmBaseType::BoolT),
                "UnitT" => Ok(EvmBaseType::UnitT),
                "StateT" => Ok(EvmBaseType::StateT),
                other => Err(IrError::Extraction(format!("unknown base type: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected base type, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_type_list(sexp: &Sexp) -> Result<Vec<EvmBaseType>, IrError> {
    let mut result = Vec::new();
    let mut current = sexp;
    loop {
        match current {
            Sexp::List(items) if !items.is_empty() => {
                let head = atom_str(&items[0])?;
                match head.as_str() {
                    "TLNil" => break,
                    "TLCons" => {
                        result.push(sexp_to_basetype(&items[1])?);
                        current = &items[2];
                    }
                    other => {
                        return Err(IrError::Extraction(format!(
                            "expected TLCons or TLNil, got: {other}"
                        )))
                    }
                }
            }
            _ => {
                return Err(IrError::Extraction(format!(
                    "expected type list, got: {current:?}"
                )))
            }
        }
    }
    Ok(result)
}

fn sexp_to_ctx(sexp: &Sexp) -> Result<EvmContext, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "InFunction" => Ok(EvmContext::InFunction(atom_string(&items[1])?)),
                "InBranch" => {
                    let b = atom_bool(&items[1])?;
                    let pred = sexp_to_evm_expr(&items[2])?;
                    let input = sexp_to_evm_expr(&items[3])?;
                    Ok(EvmContext::InBranch(b, pred, input))
                }
                "InLoop" => {
                    let input = sexp_to_evm_expr(&items[1])?;
                    let pred = sexp_to_evm_expr(&items[2])?;
                    Ok(EvmContext::InLoop(input, pred))
                }
                other => Err(IrError::Extraction(format!("unknown context: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected context, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_binop(sexp: &Sexp) -> Result<EvmBinaryOp, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "OpAdd" => Ok(EvmBinaryOp::Add),
                "OpSub" => Ok(EvmBinaryOp::Sub),
                "OpMul" => Ok(EvmBinaryOp::Mul),
                "OpDiv" => Ok(EvmBinaryOp::Div),
                "OpSDiv" => Ok(EvmBinaryOp::SDiv),
                "OpMod" => Ok(EvmBinaryOp::Mod),
                "OpSMod" => Ok(EvmBinaryOp::SMod),
                "OpExp" => Ok(EvmBinaryOp::Exp),
                "OpCheckedAdd" => Ok(EvmBinaryOp::CheckedAdd),
                "OpCheckedSub" => Ok(EvmBinaryOp::CheckedSub),
                "OpCheckedMul" => Ok(EvmBinaryOp::CheckedMul),
                "OpLt" => Ok(EvmBinaryOp::Lt),
                "OpGt" => Ok(EvmBinaryOp::Gt),
                "OpSLt" => Ok(EvmBinaryOp::SLt),
                "OpSGt" => Ok(EvmBinaryOp::SGt),
                "OpEq" => Ok(EvmBinaryOp::Eq),
                "OpAnd" => Ok(EvmBinaryOp::And),
                "OpOr" => Ok(EvmBinaryOp::Or),
                "OpXor" => Ok(EvmBinaryOp::Xor),
                "OpShl" => Ok(EvmBinaryOp::Shl),
                "OpShr" => Ok(EvmBinaryOp::Shr),
                "OpSar" => Ok(EvmBinaryOp::Sar),
                "OpByte" => Ok(EvmBinaryOp::Byte),
                "OpLogAnd" => Ok(EvmBinaryOp::LogAnd),
                "OpLogOr" => Ok(EvmBinaryOp::LogOr),
                "OpSLoad" => Ok(EvmBinaryOp::SLoad),
                "OpTLoad" => Ok(EvmBinaryOp::TLoad),
                "OpMLoad" => Ok(EvmBinaryOp::MLoad),
                "OpCalldataLoad" => Ok(EvmBinaryOp::CalldataLoad),
                other => Err(IrError::Extraction(format!("unknown binary op: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected binary op, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_unop(sexp: &Sexp) -> Result<EvmUnaryOp, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "OpIsZero" => Ok(EvmUnaryOp::IsZero),
                "OpNot" => Ok(EvmUnaryOp::Not),
                "OpNeg" => Ok(EvmUnaryOp::Neg),
                "OpSignExtend" => Ok(EvmUnaryOp::SignExtend),
                "OpClz" => Ok(EvmUnaryOp::Clz),
                other => Err(IrError::Extraction(format!("unknown unary op: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected unary op, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_ternop(sexp: &Sexp) -> Result<EvmTernaryOp, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "OpSStore" => Ok(EvmTernaryOp::SStore),
                "OpTStore" => Ok(EvmTernaryOp::TStore),
                "OpMStore" => Ok(EvmTernaryOp::MStore),
                "OpMStore8" => Ok(EvmTernaryOp::MStore8),
                "OpKeccak256" => Ok(EvmTernaryOp::Keccak256),
                "OpSelect" => Ok(EvmTernaryOp::Select),
                "OpCalldataCopy" => Ok(EvmTernaryOp::CalldataCopy),
                "OpMcopy" => Ok(EvmTernaryOp::Mcopy),
                other => Err(IrError::Extraction(format!("unknown ternary op: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected ternary op, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_envop(sexp: &Sexp) -> Result<EvmEnvOp, IrError> {
    match sexp {
        Sexp::List(items) if !items.is_empty() => {
            let head = atom_str(&items[0])?;
            match head.as_str() {
                "EnvCaller" => Ok(EvmEnvOp::Caller),
                "EnvCallValue" => Ok(EvmEnvOp::CallValue),
                "EnvCallDataSize" => Ok(EvmEnvOp::CallDataSize),
                "EnvOrigin" => Ok(EvmEnvOp::Origin),
                "EnvGasPrice" => Ok(EvmEnvOp::GasPrice),
                "EnvBlockHash" => Ok(EvmEnvOp::BlockHash),
                "EnvCoinbase" => Ok(EvmEnvOp::Coinbase),
                "EnvTimestamp" => Ok(EvmEnvOp::Timestamp),
                "EnvNumber" => Ok(EvmEnvOp::Number),
                "EnvGasLimit" => Ok(EvmEnvOp::GasLimit),
                "EnvChainId" => Ok(EvmEnvOp::ChainId),
                "EnvSelfBalance" => Ok(EvmEnvOp::SelfBalance),
                "EnvBaseFee" => Ok(EvmEnvOp::BaseFee),
                "EnvGas" => Ok(EvmEnvOp::Gas),
                "EnvAddress" => Ok(EvmEnvOp::Address),
                "EnvBalance" => Ok(EvmEnvOp::Balance),
                "EnvCodeSize" => Ok(EvmEnvOp::CodeSize),
                "EnvReturnDataSize" => Ok(EvmEnvOp::ReturnDataSize),
                other => Err(IrError::Extraction(format!("unknown env op: {other}"))),
            }
        }
        _ => Err(IrError::Extraction(format!(
            "expected env op, got: {sexp:?}"
        ))),
    }
}

fn sexp_to_list(sexp: &Sexp) -> Result<Vec<RcExpr>, IrError> {
    let mut result = Vec::new();
    let mut current = sexp;
    loop {
        match current {
            Sexp::List(items) if !items.is_empty() => {
                let head = atom_str(&items[0])?;
                match head.as_str() {
                    "Nil" => break,
                    "Cons" => {
                        result.push(sexp_to_evm_expr(&items[1])?);
                        current = &items[2];
                    }
                    other => {
                        return Err(IrError::Extraction(format!(
                            "expected Cons or Nil, got: {other}"
                        )))
                    }
                }
            }
            _ => {
                return Err(IrError::Extraction(format!(
                    "expected list, got: {current:?}"
                )))
            }
        }
    }
    Ok(result)
}

// ---- Atom helpers ----

fn atom_str(sexp: &Sexp) -> Result<String, IrError> {
    match sexp {
        Sexp::Atom(s) => Ok(s.clone()),
        Sexp::List(items) if items.len() == 1 => atom_str(&items[0]),
        _ => Err(IrError::Extraction(format!("expected atom, got: {sexp:?}"))),
    }
}

fn atom_i64(sexp: &Sexp) -> Result<i64, IrError> {
    let s = atom_str(sexp)?;
    s.parse::<i64>()
        .map_err(|e| IrError::Extraction(format!("expected integer: {e}")))
}

fn atom_bool(sexp: &Sexp) -> Result<bool, IrError> {
    let s = atom_str(sexp)?;
    match s.as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(IrError::Extraction(format!("expected bool, got: {s}"))),
    }
}

fn atom_string(sexp: &Sexp) -> Result<String, IrError> {
    let s = atom_str(sexp)?;
    // Strip surrounding quotes if present
    if s.starts_with('"') && s.ends_with('"') {
        Ok(s[1..s.len() - 1].to_owned())
    } else {
        Ok(s)
    }
}

// ============================================================
// Pretty-printing
// ============================================================

/// Pretty-print an `EvmExpr` as an indented s-expression.
pub fn expr_to_pretty(expr: &EvmExpr, indent: usize) -> String {
    let flat = expr_to_sexp(expr);
    pretty_sexp(&flat, indent)
}

/// Pretty-print a flat s-expression string with indentation.
///
/// Leaf forms (constants, types, operators, contexts) stay inline.
/// Compound forms break arguments onto separate lines when the flat
/// representation exceeds ~80 columns from the current indent.
pub fn pretty_sexp(sexp: &str, indent: usize) -> String {
    let tokens = tokenize_sexp(sexp);
    let (tree, _) = parse_sexp_tokens(&tokens, 0);
    format_tree(&tree, indent)
}

/// A simple s-expression tree for pretty-printing.
#[derive(Debug)]
enum STree {
    Atom(String),
    List(Vec<Self>),
}

fn tokenize_sexp(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        match bytes[i] {
            b'(' => {
                tokens.push("(".to_string());
                i += 1;
            }
            b')' => {
                tokens.push(")".to_string());
                i += 1;
            }
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'"' => {
                // Quoted string — consume until closing quote
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1; // skip escaped char
                    }
                    i += 1;
                }
                i += 1; // closing quote
                tokens.push(s[start..i].to_string());
            }
            _ => {
                let start = i;
                while i < bytes.len()
                    && !matches!(bytes[i], b'(' | b')' | b' ' | b'\t' | b'\n' | b'\r')
                {
                    i += 1;
                }
                tokens.push(s[start..i].to_string());
            }
        }
    }
    tokens
}

fn parse_sexp_tokens(tokens: &[String], pos: usize) -> (STree, usize) {
    if pos >= tokens.len() {
        return (STree::Atom(String::new()), pos);
    }
    if tokens[pos] == "(" {
        let mut children = Vec::new();
        let mut i = pos + 1;
        while i < tokens.len() && tokens[i] != ")" {
            let (child, next) = parse_sexp_tokens(tokens, i);
            children.push(child);
            i = next;
        }
        (STree::List(children), i + 1) // skip ")"
    } else {
        (STree::Atom(tokens[pos].clone()), pos + 1)
    }
}

fn flat_len(tree: &STree) -> usize {
    match tree {
        STree::Atom(s) => s.len(),
        STree::List(children) => {
            if children.is_empty() {
                return 2; // "()"
            }
            // "(" + children joined by " " + ")"
            2 + children.iter().map(flat_len).sum::<usize>() + children.len() - 1
        }
    }
}

fn flat_str(tree: &STree) -> String {
    match tree {
        STree::Atom(s) => s.clone(),
        STree::List(children) => {
            let inner: Vec<String> = children.iter().map(flat_str).collect();
            format!("({})", inner.join(" "))
        }
    }
}

/// Returns true if this tree is a "leaf-like" form that should never be broken
/// across lines: operators, types, constants, contexts, selectors, vars.
fn is_leaf_form(tree: &STree) -> bool {
    match tree {
        STree::Atom(_) => true,
        STree::List(children) => {
            if children.is_empty() {
                return true;
            }
            if let STree::Atom(head) = &children[0] {
                matches!(
                    head.as_str(),
                    // Operators
                    "OpAdd" | "OpSub" | "OpMul" | "OpDiv" | "OpSDiv" | "OpMod" | "OpSMod"
                    | "OpExp" | "OpCheckedAdd" | "OpCheckedSub" | "OpCheckedMul"
                    | "OpLt" | "OpGt" | "OpSLt" | "OpSGt" | "OpEq"
                    | "OpAnd" | "OpOr" | "OpXor" | "OpShl" | "OpShr" | "OpSar" | "OpByte"
                    | "OpLogAnd" | "OpLogOr" | "OpSLoad" | "OpTLoad" | "OpMLoad" | "OpCalldataLoad"
                    | "OpIsZero" | "OpNot" | "OpNeg" | "OpSignExtend"
                    | "OpSStore" | "OpTStore" | "OpMStore" | "OpMStore8" | "OpKeccak256" | "OpSelect" | "OpCalldataCopy" | "OpMcopy"
                    // Types
                    | "UIntT" | "IntT" | "BytesT" | "AddrT" | "BoolT" | "UnitT" | "StateT"
                    | "Base" | "TupleT" | "TLCons" | "TLNil"
                    // Constants
                    | "SmallInt" | "LargeInt" | "ConstBool" | "ConstAddr"
                    // Context
                    | "InFunction"
                    // Leaves
                    | "Selector" | "Var" | "VarStore" | "StorageField" | "MemRegion" | "DynAlloc"
                    // Empty/Arg (no sub-expressions)
                    | "Arg" | "Empty" | "Const"
                )
            } else {
                false
            }
        }
    }
}

fn format_tree(tree: &STree, indent: usize) -> String {
    match tree {
        STree::Atom(s) => s.clone(),
        STree::List(children) => {
            if children.is_empty() {
                return "()".to_string();
            }

            // Always inline leaf forms
            if is_leaf_form(tree) {
                return flat_str(tree);
            }

            // If the whole thing fits in ~80 cols, keep it inline
            let total_flat = flat_len(tree);
            if indent + total_flat <= 100 {
                return flat_str(tree);
            }

            // Break: put head on first line, each arg on its own indented line
            let head = flat_str(&children[0]);
            if children.len() == 1 {
                return format!("({head})");
            }

            let child_indent = indent + 2;
            let pad = " ".repeat(child_indent);
            let mut out = format!("({head}");
            for child in &children[1..] {
                let formatted = format_tree(child, child_indent);
                out.push('\n');
                out.push_str(&pad);
                out.push_str(&formatted);
            }
            out.push(')');
            out
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_helpers;

    #[test]
    fn test_const_roundtrip() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let expr = ast_helpers::const_int(42, ctx);
        let sexp = expr_to_sexp(&expr);
        let parsed = sexp_to_expr(&sexp).unwrap();
        assert_eq!(*expr, *parsed);
    }

    #[test]
    fn test_bop_roundtrip() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let a = ast_helpers::const_int(1, ctx.clone());
        let b = ast_helpers::const_int(2, ctx);
        let expr = ast_helpers::add(a, b);
        let sexp = expr_to_sexp(&expr);
        let parsed = sexp_to_expr(&sexp).unwrap();
        assert_eq!(*expr, *parsed);
    }

    #[test]
    fn test_sstore_roundtrip() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let slot = ast_helpers::const_int(0, ctx.clone());
        let val = ast_helpers::const_int(42, ctx.clone());
        let state = Rc::new(EvmExpr::Arg(EvmType::Base(EvmBaseType::StateT), ctx));
        let expr = ast_helpers::sstore(slot, val, state);
        let sexp = expr_to_sexp(&expr);
        let parsed = sexp_to_expr(&sexp).unwrap();
        assert_eq!(*expr, *parsed);
    }

    #[test]
    fn test_if_roundtrip() {
        let ctx = EvmContext::InFunction("test".to_owned());
        let cond = ast_helpers::const_bool(true, ctx.clone());
        let inputs = ast_helpers::empty(EvmType::Base(EvmBaseType::UnitT), ctx.clone());
        let then_e = ast_helpers::const_int(1, ctx.clone());
        let else_e = ast_helpers::const_int(0, ctx);
        let expr = ast_helpers::if_then_else(cond, inputs, then_e, else_e);
        let sexp = expr_to_sexp(&expr);
        let parsed = sexp_to_expr(&sexp).unwrap();
        assert_eq!(*expr, *parsed);
    }

    #[test]
    fn test_selector_roundtrip() {
        let expr = Rc::new(EvmExpr::Selector("transfer(address,uint256)".to_owned()));
        let sexp = expr_to_sexp(&expr);
        let parsed = sexp_to_expr(&sexp).unwrap();
        assert_eq!(*expr, *parsed);
    }
}
