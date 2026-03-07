//! Custom egglog U256 sort for 256-bit unsigned integer arithmetic.
//!
//! This sort enables egglog rules to perform full 256-bit constant folding,
//! power-of-2 detection, and range analysis on EVM values that don't fit in i64.
//!
//! Values are interned in a global `IndexSet<W>` behind a `Mutex`,
//! following the same pattern as egglog's built-in `BigIntSort`.

use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use egglog::{
    add_primitives,
    ast::Literal,
    extract::Extractor,
    sort::{FromSort, IntoSort, Sort},
    EGraph, Term, TermDag, TypeInfo, Value,
};
use ruint::aliases::U256;

/// Newtype wrapper around `U256` to satisfy orphan rules for `FromSort`/`IntoSort`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct W(pub U256);

lazy_static::lazy_static! {
    static ref U256_SORT_NAME: egglog::ast::Symbol = "U256".into();
    static ref U256S: Mutex<indexmap::IndexSet<W>> = Mutex::new(indexmap::IndexSet::new());
}

/// Custom egglog sort for 256-bit unsigned integers.
#[derive(Debug)]
pub struct U256Sort;

impl Sort for U256Sort {
    fn name(&self) -> egglog::ast::Symbol {
        *U256_SORT_NAME
    }

    fn as_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync + 'static> {
        self
    }

    #[rustfmt::skip]
    #[allow(clippy::wildcard_imports)]
    fn register_primitives(self: Arc<Self>, eg: &mut TypeInfo) {
        type Opt<T = ()> = Option<T>;

        // ---- Conversion ----
        add_primitives!(eg, "u256-from-i64" = |a: i64| -> W {
            W(if a >= 0 { U256::from(a as u64) } else { U256::MAX - U256::from((-a - 1) as u64) })
        });

        add_primitives!(eg, "u256-to-i64" = |a: W| -> Opt<i64> {
            if a.0 <= U256::from(i64::MAX as u64) { Some(a.0.as_limbs()[0] as i64) } else { None }
        });

        add_primitives!(eg, "u256-from-hex" = |a: egglog::ast::Symbol| -> Opt<W> {
            U256::from_str_radix(a.as_str().strip_prefix("0x").unwrap_or(a.as_str()), 16).ok().map(W)
        });

        add_primitives!(eg, "u256-to-hex" = |a: W| -> egglog::ast::Symbol {
            format!("{:x}", a.0).into()
        });

        // ---- Arithmetic (wrapping) ----
        add_primitives!(eg, "u256-add" = |a: W, b: W| -> W { W(a.0.wrapping_add(b.0)) });
        add_primitives!(eg, "u256-sub" = |a: W, b: W| -> W { W(a.0.wrapping_sub(b.0)) });
        add_primitives!(eg, "u256-mul" = |a: W, b: W| -> W { W(a.0.wrapping_mul(b.0)) });
        add_primitives!(eg, "u256-div" = |a: W, b: W| -> Opt<W> {
            if b.0.is_zero() { None } else { Some(W(a.0 / b.0)) }
        });
        add_primitives!(eg, "u256-mod" = |a: W, b: W| -> Opt<W> {
            if b.0.is_zero() { None } else { Some(W(a.0 % b.0)) }
        });
        add_primitives!(eg, "u256-exp" = |a: W, b: W| -> W { W(a.0.wrapping_pow(b.0)) });

        // ---- Bitwise ----
        add_primitives!(eg, "u256-and" = |a: W, b: W| -> W { W(a.0 & b.0) });
        add_primitives!(eg, "u256-or"  = |a: W, b: W| -> W { W(a.0 | b.0) });
        add_primitives!(eg, "u256-xor" = |a: W, b: W| -> W { W(a.0 ^ b.0) });
        add_primitives!(eg, "u256-not" = |a: W| -> W { W(!a.0) });
        add_primitives!(eg, "u256-shl" = |a: W, b: W| -> W {
            W(if b.0 >= U256::from(256u64) { U256::ZERO } else { a.0 << b.0.as_limbs()[0] as usize })
        });
        add_primitives!(eg, "u256-shr" = |a: W, b: W| -> W {
            W(if b.0 >= U256::from(256u64) { U256::ZERO } else { a.0 >> b.0.as_limbs()[0] as usize })
        });

        // ---- Comparison guards (return Option<()> for rule guards) ----
        add_primitives!(eg, "u256-lt"      = |a: W, b: W| -> Opt { (a.0 < b.0).then_some(()) });
        add_primitives!(eg, "u256-gt"      = |a: W, b: W| -> Opt { (a.0 > b.0).then_some(()) });
        add_primitives!(eg, "u256-eq"      = |a: W, b: W| -> Opt { (a.0 == b.0).then_some(()) });
        add_primitives!(eg, "u256-ne"      = |a: W, b: W| -> Opt { (a.0 != b.0).then_some(()) });
        add_primitives!(eg, "u256-is-zero" = |a: W| -> Opt { a.0.is_zero().then_some(()) });
        add_primitives!(eg, "u256-nonzero" = |a: W| -> Opt { (!a.0.is_zero()).then_some(()) });

        // ---- Optimization helpers ----
        add_primitives!(eg, "u256-is-power-of-2" = |a: W| -> Opt {
            a.0.is_power_of_two().then_some(())
        });
        add_primitives!(eg, "u256-log2" = |a: W| -> Opt<W> {
            if a.0.is_zero() { None } else { Some(W(U256::from(a.0.bit_len() - 1))) }
        });
        add_primitives!(eg, "u256-bits" = |a: W| -> W { W(U256::from(a.0.bit_len())) });
        add_primitives!(eg, "u256-clz" = |a: W| -> W { W(U256::from(256 - a.0.bit_len())) });

        // ---- Overflow checks (return Option<()> for rule guards) ----
        add_primitives!(eg, "u256-add-no-overflow" = |a: W, b: W| -> Opt {
            a.0.checked_add(b.0).is_some().then_some(())
        });
        add_primitives!(eg, "u256-sub-no-underflow" = |a: W, b: W| -> Opt {
            (a.0 >= b.0).then_some(())
        });
        add_primitives!(eg, "u256-mul-no-overflow" = |a: W, b: W| -> Opt {
            a.0.checked_mul(b.0).is_some().then_some(())
        });

        // ---- Lattice ----
        add_primitives!(eg, "u256-min" = |a: W, b: W| -> W { W(a.0.min(b.0)) });
        add_primitives!(eg, "u256-max" = |a: W, b: W| -> W { W(a.0.max(b.0)) });
    }

    fn extract_term(
        &self,
        _egraph: &EGraph,
        value: Value,
        _extractor: &Extractor<'_>,
        termdag: &mut TermDag,
    ) -> Option<(egglog::extract::Cost, Term)> {
        #[cfg(debug_assertions)]
        debug_assert_eq!(value.tag, self.name());

        let val = W::load(self, &value);
        let hex = format!("{:x}", val.0);
        let as_string = termdag.lit(Literal::String(hex.into()));
        Some((1, termdag.app("u256-from-hex".into(), vec![as_string])))
    }
}

impl FromSort for W {
    type Sort = U256Sort;
    fn load(_sort: &Self::Sort, value: &Value) -> Self {
        let i = value.bits as usize;
        U256S.lock().unwrap().get_index(i).unwrap().clone()
    }
}

impl IntoSort for W {
    type Sort = U256Sort;
    fn store(self, _sort: &Self::Sort) -> Option<Value> {
        let (i, _) = U256S.lock().unwrap().insert_full(self);
        Some(Value {
            #[cfg(debug_assertions)]
            tag: U256Sort.name(),
            bits: i as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_roundtrip() {
        let val = W(U256::from(42u64));
        let stored = val.store(&U256Sort).unwrap();
        let loaded = W::load(&U256Sort, &stored);
        assert_eq!(W(U256::from(42u64)), loaded);
    }

    #[test]
    fn test_u256_large_value_roundtrip() {
        let val = W(U256::MAX);
        let stored = val.clone().store(&U256Sort).unwrap();
        let loaded = W::load(&U256Sort, &stored);
        assert_eq!(val, loaded);
    }

    #[test]
    fn test_u256_dedup() {
        let val = W(U256::from(999u64));
        let s1 = val.clone().store(&U256Sort).unwrap();
        let s2 = val.store(&U256Sort).unwrap();
        assert_eq!(s1.bits, s2.bits);
    }

    #[test]
    fn test_u256_wrapping_arithmetic() {
        assert_eq!(U256::MAX.wrapping_add(U256::from(1u64)), U256::ZERO);
        assert_eq!(U256::ZERO.wrapping_sub(U256::from(1u64)), U256::MAX);
    }

    #[test]
    fn test_u256_power_of_2() {
        assert!(U256::from(1u64).is_power_of_two());
        assert!(U256::from(256u64).is_power_of_two());
        let two_160: U256 = U256::from(1u64) << 160;
        assert!(two_160.is_power_of_two());
        assert!(!(two_160 - U256::from(1u64)).is_power_of_two());
    }
}
