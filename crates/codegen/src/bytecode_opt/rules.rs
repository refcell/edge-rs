//! Rewrite rules for bytecode-level peephole optimization.

/// Duplicate push deduplication rules.
pub(crate) const DUP_DEDUP_RULES: &str = r#"
;; PUSH0 PUSH0 → PUSH0 DUP1
(rewrite (ICons (IPush0) (ICons (IPush0) ?rest))
         (ICons (IPush0) (ICons (IDup 1) ?rest))
  :ruleset bytecode-peepholes)

;; PUSH(x) PUSH(x) → PUSH(x) DUP1 (PushSmall)
(rewrite (IPushCons (PushSmall ?x) (IPushCons (PushSmall ?x) ?rest))
         (IPushCons (PushSmall ?x) (ICons (IDup 1) ?rest))
  :ruleset bytecode-peepholes)

;; PUSH(x) PUSH(x) → PUSH(x) DUP1 (PushHex)
(rewrite (IPushCons (PushHex ?x) (IPushCons (PushHex ?x) ?rest))
         (IPushCons (PushHex ?x) (ICons (IDup 1) ?rest))
  :ruleset bytecode-peepholes)
"#;

/// Cancellation rules: adjacent inverse operations.
pub(crate) const CANCELLATION_RULES: &str = r#"
;; SWAPn SWAPn → ε (self-inverse)
(rewrite (ICons (ISwap ?n) (ICons (ISwap ?n) ?rest))
         ?rest
  :ruleset bytecode-peepholes)

;; NOT NOT → ε
(rewrite (ICons (INot) (ICons (INot) ?rest))
         ?rest
  :ruleset bytecode-peepholes)

;; ISZERO ISZERO → ε (double boolean negation)
(rewrite (ICons (IIsZero) (ICons (IIsZero) ?rest))
         ?rest
  :ruleset bytecode-peepholes)

;; DUP<n> POP → ε (any dup immediately popped is a no-op)
(rewrite (ICons (IDup ?n) (ICons (IPop) ?rest))
         ?rest
  :ruleset bytecode-peepholes)

;; DUP1 SWAP1 → DUP1 (swap of two identical top elements is no-op)
;; After DUP1: stack = [..., a, a]. SWAP1 swaps top two: still [..., a, a].
(rewrite (ICons (IDup 1) (ICons (ISwap 1) ?rest))
         (ICons (IDup 1) ?rest)
  :ruleset bytecode-peepholes)

;; SWAP1 before commutative binary op → drop SWAP1
;; For commutative ops: a OP b = b OP a, so swapping operands is a no-op.
(rewrite (ICons (ISwap 1) (ICons (IAdd) ?rest))
         (ICons (IAdd) ?rest)
  :ruleset bytecode-peepholes)
(rewrite (ICons (ISwap 1) (ICons (IMul) ?rest))
         (ICons (IMul) ?rest)
  :ruleset bytecode-peepholes)
(rewrite (ICons (ISwap 1) (ICons (IAnd) ?rest))
         (ICons (IAnd) ?rest)
  :ruleset bytecode-peepholes)
(rewrite (ICons (ISwap 1) (ICons (IOr) ?rest))
         (ICons (IOr) ?rest)
  :ruleset bytecode-peepholes)
(rewrite (ICons (ISwap 1) (ICons (IXor) ?rest))
         (ICons (IXor) ?rest)
  :ruleset bytecode-peepholes)
(rewrite (ICons (ISwap 1) (ICons (IEq) ?rest))
         (ICons (IEq) ?rest)
  :ruleset bytecode-peepholes)
"#;

/// Constant folding rules (PushSmall i64 arithmetic only).
///
/// EVM operand order: stack `[..., a, b]` where `b` is TOS.
/// For `PUSH i PUSH j OP`: i is pushed first (deeper), j second (TOS).
/// So EVM sees stack top = j, second = i.
///
/// In our cons-list, `IPushCons i (IPushCons j (ICons OP rest))` means
/// i is emitted first (pushed first, deeper on stack), j emitted second (TOS).
pub(crate) const CONST_FOLD_RULES: &str = r#"
;; PUSH i, PUSH j, ADD → PUSH (i+j)
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IAdd) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (+ ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, SUB → PUSH (j-i)  [EVM: TOS=j, second=i, SUB = j - i]
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (ISub) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (- ?j ?i)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, MUL → PUSH (i*j)
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IMul) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (* ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, AND → PUSH (i&j)
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IAnd) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (& ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, OR → PUSH (i|j)
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IOr) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (| ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, XOR → PUSH (i^j)
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IXor) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (^ ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, SHL → PUSH (j << i)  [EVM: shift=TOS=j... wait]
;; EVM SHL: pops (shift, value), pushes value << shift.
;; Stack [i, j] → SHL = j << i (i is deeper = value, j is TOS = shift).
;; Wait no: i is pushed first (deeper), j pushed second (TOS).
;; SHL pops shift (TOS=j) then value (i), result = i << j.
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IShl) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (<< ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH j, SHR → PUSH (i >> j)  [EVM: SHR pops shift(TOS=j), value(i)]
(rule ((= ?seq (IPushCons (PushSmall ?i) (IPushCons (PushSmall ?j) (ICons (IShr) ?rest)))))
      ((union ?seq (IPushCons (PushSmall (>> ?i ?j)) ?rest)))
  :ruleset bytecode-const-fold)

;; PUSH i, PUSH i, EQ → PUSH 1 (equal constants always equal)
(rewrite (IPushCons (PushSmall ?x) (IPushCons (PushSmall ?x) (ICons (IEq) ?rest)))
         (IPushCons (PushSmall 1) ?rest)
  :ruleset bytecode-const-fold)

;; PUSH(hex) x, PUSH(hex) x, EQ → PUSH 1
(rewrite (IPushCons (PushHex ?x) (IPushCons (PushHex ?x) (ICons (IEq) ?rest)))
         (IPushCons (PushSmall 1) ?rest)
  :ruleset bytecode-const-fold)
"#;

/// Strength reduction rules.
pub(crate) const STRENGTH_REDUCTION_RULES: &str = r#"
;; === PushSmall 0 identity rules ===

;; PUSH 0 ADD → ε (additive identity)
(rewrite (IPushCons (PushSmall 0) (ICons (IAdd) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 1 MUL → ε (multiplicative identity)
(rewrite (IPushCons (PushSmall 1) (ICons (IMul) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 1 DIV → ε (division identity)
(rewrite (IPushCons (PushSmall 1) (ICons (IDiv) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 1 SDIV → ε (signed division identity)
(rewrite (IPushCons (PushSmall 1) (ICons (ISDiv) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 0 OR → ε (bitwise OR identity)
(rewrite (IPushCons (PushSmall 0) (ICons (IOr) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 0 XOR → ε (bitwise XOR identity)
(rewrite (IPushCons (PushSmall 0) (ICons (IXor) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 0 EQ → ISZERO (saves push bytes)
(rewrite (IPushCons (PushSmall 0) (ICons (IEq) ?rest))
         (ICons (IIsZero) ?rest)
  :ruleset bytecode-strength-red)

;; PUSH 0 SHL → ε (shift by 0 = identity)
(rewrite (IPushCons (PushSmall 0) (ICons (IShl) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 0 SHR → ε (shift by 0 = identity)
(rewrite (IPushCons (PushSmall 0) (ICons (IShr) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH 0 SAR → ε (shift by 0 = identity)
(rewrite (IPushCons (PushSmall 0) (ICons (ISar) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; === PushSmall 0 annihilator rules ===
;; PUSH 0 MUL: stack [x, 0] → MUL → [0]. Replace with POP + PUSH0.
(rewrite (IPushCons (PushSmall 0) (ICons (IMul) ?rest))
         (ICons (IPop) (ICons (IPush0) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 0 AND: stack [x, 0] → AND → [0]. Replace with POP + PUSH0.
(rewrite (IPushCons (PushSmall 0) (ICons (IAnd) ?rest))
         (ICons (IPop) (ICons (IPush0) ?rest))
  :ruleset bytecode-strength-red)

;; === PUSH0 (opcode) identity rules ===
;; These fire when the compiler emits Op(Push0) rather than Push(vec![0]).

;; PUSH0 ADD → ε (additive identity)
(rewrite (ICons (IPush0) (ICons (IAdd) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 OR → ε (bitwise OR identity)
(rewrite (ICons (IPush0) (ICons (IOr) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 XOR → ε (bitwise XOR identity)
(rewrite (ICons (IPush0) (ICons (IXor) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 SHL → ε (shift by 0 = identity)
(rewrite (ICons (IPush0) (ICons (IShl) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 SHR → ε (shift by 0 = identity)
(rewrite (ICons (IPush0) (ICons (IShr) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 SAR → ε (shift by 0 = identity)
(rewrite (ICons (IPush0) (ICons (ISar) ?rest))
         ?rest
  :ruleset bytecode-strength-red)

;; PUSH0 EQ → ISZERO (saves 1 byte)
(rewrite (ICons (IPush0) (ICons (IEq) ?rest))
         (ICons (IIsZero) ?rest)
  :ruleset bytecode-strength-red)

;; === PUSH0 (opcode) annihilator rules ===

;; PUSH0 MUL: stack [x, 0] → [0]. Equivalent to POP + PUSH0.
;; Gas savings: MUL=5 vs POP(2)+PUSH0(3)=5. Size neutral. But eliminates a MUL.
(rewrite (ICons (IPush0) (ICons (IMul) ?rest))
         (ICons (IPop) (ICons (IPush0) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH0 AND: stack [x, 0] → [0]. Equivalent to POP + PUSH0.
(rewrite (ICons (IPush0) (ICons (IAnd) ?rest))
         (ICons (IPop) (ICons (IPush0) ?rest))
  :ruleset bytecode-strength-red)

;; === Multiplication by power of 2 → shift ===
;; PUSH 2 MUL → PUSH 1 SHL (saves gas: MUL=5, SHL=3)
(rewrite (IPushCons (PushSmall 2) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 1) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 4 MUL → PUSH 2 SHL
(rewrite (IPushCons (PushSmall 4) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 2) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 8 MUL → PUSH 3 SHL
(rewrite (IPushCons (PushSmall 8) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 3) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 16 MUL → PUSH 4 SHL
(rewrite (IPushCons (PushSmall 16) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 4) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 32 MUL → PUSH 5 SHL
(rewrite (IPushCons (PushSmall 32) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 5) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 64 MUL → PUSH 6 SHL
(rewrite (IPushCons (PushSmall 64) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 6) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 128 MUL → PUSH 7 SHL
(rewrite (IPushCons (PushSmall 128) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 7) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 256 MUL → PUSH 8 SHL
(rewrite (IPushCons (PushSmall 256) (ICons (IMul) ?rest))
         (IPushCons (PushSmall 8) (ICons (IShl) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 2 DIV → PUSH 1 SHR (saves gas: DIV=5, SHR=3; unsigned only)
(rewrite (IPushCons (PushSmall 2) (ICons (IDiv) ?rest))
         (IPushCons (PushSmall 1) (ICons (IShr) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 4 DIV → PUSH 2 SHR
(rewrite (IPushCons (PushSmall 4) (ICons (IDiv) ?rest))
         (IPushCons (PushSmall 2) (ICons (IShr) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 8 DIV → PUSH 3 SHR
(rewrite (IPushCons (PushSmall 8) (ICons (IDiv) ?rest))
         (IPushCons (PushSmall 3) (ICons (IShr) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 256 DIV → PUSH 8 SHR
(rewrite (IPushCons (PushSmall 256) (ICons (IDiv) ?rest))
         (IPushCons (PushSmall 8) (ICons (IShr) ?rest))
  :ruleset bytecode-strength-red)
"#;

/// MOD by power-of-2 → AND with (2^N - 1).
/// x MOD 2^N == x AND (2^N - 1). AND costs 3 gas, MOD costs 5 gas.
pub(crate) const MOD_TO_AND_RULES: &str = r#"
;; PUSH 2 MOD → PUSH 1 AND (x % 2 == x & 1)
(rewrite (IPushCons (PushSmall 2) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 1) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 4 MOD → PUSH 3 AND
(rewrite (IPushCons (PushSmall 4) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 3) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 8 MOD → PUSH 7 AND
(rewrite (IPushCons (PushSmall 8) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 7) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 16 MOD → PUSH 15 AND
(rewrite (IPushCons (PushSmall 16) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 15) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 32 MOD → PUSH 31 AND
(rewrite (IPushCons (PushSmall 32) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 31) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 64 MOD → PUSH 63 AND
(rewrite (IPushCons (PushSmall 64) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 63) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 128 MOD → PUSH 127 AND
(rewrite (IPushCons (PushSmall 128) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 127) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)

;; PUSH 256 MOD → PUSH 255 AND
(rewrite (IPushCons (PushSmall 256) (ICons (IMod) ?rest))
         (IPushCons (PushSmall 255) (ICons (IAnd) ?rest))
  :ruleset bytecode-strength-red)
"#;

/// Dead push removal rules.
pub(crate) const DEAD_PUSH_RULES: &str = r#"
;; PUSH(small) POP → ε
(rewrite (IPushCons (PushSmall ?x) (ICons (IPop) ?rest))
         ?rest
  :ruleset bytecode-dead-push)

;; PUSH(hex) POP → ε
(rewrite (IPushCons (PushHex ?x) (ICons (IPop) ?rest))
         ?rest
  :ruleset bytecode-dead-push)

;; PUSH0 POP → ε
(rewrite (ICons (IPush0) (ICons (IPop) ?rest))
         ?rest
  :ruleset bytecode-dead-push)
"#;
