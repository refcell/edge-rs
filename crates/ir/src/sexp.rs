//! S-expression conversion for the egglog round-trip.
//!
//! Converts between `EvmExpr` and egglog-compatible s-expression strings.
//! Used to insert IR into an egglog EGraph and extract optimized results.

use std::rc::Rc;

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
        EvmExpr::Single(e) => format!("(Single {})", expr_to_sexp(e)),
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
        EvmExpr::Log(n, topics, data, st) => {
            let topics_s = list_to_sexp(topics);
            format!(
                "(Log {} {} {} {})",
                n,
                topics_s,
                expr_to_sexp(data),
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
            format!("(Call \"{}\" {})", name, expr_to_sexp(args))
        }
        EvmExpr::Selector(sig) => format!("(Selector \"{}\")", sig),
        EvmExpr::LetBind(name, value, body) => {
            format!(
                "(LetBind \"{}\" {} {})",
                name,
                expr_to_sexp(value),
                expr_to_sexp(body)
            )
        }
        EvmExpr::Var(name) => format!("(Var \"{}\")", name),
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
    }
}

fn const_sexp(c: &EvmConstant) -> String {
    match c {
        EvmConstant::SmallInt(i) => format!("(SmallInt {})", i),
        EvmConstant::LargeInt(s) => format!("(LargeInt \"{}\")", s),
        EvmConstant::Bool(b) => format!("(ConstBool {})", b),
        EvmConstant::Addr(s) => format!("(ConstAddr \"{}\")", s),
    }
}

fn type_sexp(ty: &EvmType) -> String {
    match ty {
        EvmType::Base(b) => format!("(Base {})", basetype_sexp(b)),
        EvmType::TupleT(types) => {
            let list = types.iter().rev().fold("(TLNil)".to_owned(), |acc, t| {
                format!("(TLCons {} {})", basetype_sexp(t), acc)
            });
            format!("(TupleT {})", list)
        }
    }
}

fn basetype_sexp(bt: &EvmBaseType) -> String {
    match bt {
        EvmBaseType::UIntT(n) => format!("(UIntT {})", n),
        EvmBaseType::IntT(n) => format!("(IntT {})", n),
        EvmBaseType::BytesT(n) => format!("(BytesT {})", n),
        EvmBaseType::AddrT => "(AddrT)".to_owned(),
        EvmBaseType::BoolT => "(BoolT)".to_owned(),
        EvmBaseType::UnitT => "(UnitT)".to_owned(),
        EvmBaseType::StateT => "(StateT)".to_owned(),
    }
}

fn ctx_sexp(ctx: &EvmContext) -> String {
    match ctx {
        EvmContext::InFunction(name) => format!("(InFunction \"{}\")", name),
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

fn binop_sexp(op: &EvmBinaryOp) -> &'static str {
    match op {
        EvmBinaryOp::Add => "(OpAdd)",
        EvmBinaryOp::Sub => "(OpSub)",
        EvmBinaryOp::Mul => "(OpMul)",
        EvmBinaryOp::Div => "(OpDiv)",
        EvmBinaryOp::SDiv => "(OpSDiv)",
        EvmBinaryOp::Mod => "(OpMod)",
        EvmBinaryOp::SMod => "(OpSMod)",
        EvmBinaryOp::Exp => "(OpExp)",
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

fn unop_sexp(op: &EvmUnaryOp) -> &'static str {
    match op {
        EvmUnaryOp::IsZero => "(OpIsZero)",
        EvmUnaryOp::Not => "(OpNot)",
        EvmUnaryOp::Neg => "(OpNeg)",
        EvmUnaryOp::SignExtend => "(OpSignExtend)",
    }
}

fn ternop_sexp(op: &EvmTernaryOp) -> &'static str {
    match op {
        EvmTernaryOp::SStore => "(OpSStore)",
        EvmTernaryOp::TStore => "(OpTStore)",
        EvmTernaryOp::MStore => "(OpMStore)",
        EvmTernaryOp::MStore8 => "(OpMStore8)",
        EvmTernaryOp::Keccak256 => "(OpKeccak256)",
        EvmTernaryOp::Select => "(OpSelect)",
    }
}

fn envop_sexp(op: &EvmEnvOp) -> &'static str {
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
            "trailing tokens after s-expression: {:?}",
            rest
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
                "Single" => {
                    let e = sexp_to_evm_expr(&items[1])?;
                    Ok(Rc::new(EvmExpr::Single(e)))
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
                    let data = sexp_to_evm_expr(&items[3])?;
                    let st = sexp_to_evm_expr(&items[4])?;
                    Ok(Rc::new(EvmExpr::Log(n, topics, data, st)))
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
                    let args = sexp_to_evm_expr(&items[2])?;
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
