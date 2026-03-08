---
title: External Calls
---

# External Calls

External calls enable cross-contract interaction through ABI-typed
addresses. The `Impl` type wraps an address with compile-time ABI
information, enabling type-safe method dispatch that compiles to
EVM CALL, STATICCALL, or DELEGATECALL instructions.

## Syntax Options

Two syntax options are under consideration. Both share identical
semantics; the difference is purely syntactic.

### Option A: `Impl<ABI>` (keyword or builtin parameterized type)

```text
<impl_type> ::= "Impl" "<" <ident> ("+" <ident>)* ">" ;
```

`Impl` takes one or more plus-separated interface identifiers as
type parameters. Whether `Impl` is a new keyword or a compiler-
recognized builtin type name (like `map` or `Result`) is an open
question. As a keyword, it cannot be shadowed or redefined by
user code; as a builtin type, it is lighter-weight but could
theoretically be shadowed.

```edge
let token: &s Impl<IERC20>;
let multi: Impl<IERC20 + IERC20Metadata>;
fn withdraw(token: Impl<IERC20>, amount: u256) { }
```

### Option B: `impl ABI` (keyword in type position)

```text
<impl_type> ::= "impl" <ident> ("+" <ident>)* ;
```

Reuses the existing `impl` keyword in type position, following
Rust's `impl Trait` syntax exactly. The parser disambiguates
impl blocks (`impl Foo {`) from impl types (`impl Foo` in type
position) by context.

```edge
let token: &s impl IERC20;
let multi: impl IERC20 + IERC20Metadata;
fn withdraw(token: impl IERC20, amount: u256) { }
```

The remainder of this specification uses Option A syntax. All
examples apply equally to Option B with the obvious substitution.

## Impl Type

When the inner identifier resolves to an `abi` declaration, the
`Impl` type represents an address that conforms to the given
external interface. At the EVM level, values of this type are
20-byte addresses. The `Impl` wrapper exists purely at compile
time to enable typed method dispatch.

When the inner identifier resolves to a `trait` declaration, the
`Impl` type is syntactic sugar for a generic type parameter with
a trait bound. `Impl<T>` in argument position desugars to an
anonymous generic `<__T: T>`.

Both ABI and trait forms use the same syntax, and the compiler
disambiguates based on whether the inner identifier was declared
with `abi` or `trait`. The two have different semantics, return
types, and positional validity rules (see below).

Mixing ABI and trait identifiers in a single `Impl` type is a
compile error because they represent fundamentally different
dispatch mechanisms. ABI methods compile to cross-contract EVM
call instructions with ABI encoding, gas forwarding, and fallible
results. Trait methods compile to monomorphized inline code with
direct returns. A single `Impl<A + B>` expression must resolve
to one dispatch mechanism or the other — there is no meaningful
way to combine them.

Multiple ABI identifiers or multiple trait identifiers may be
composed with `+`.

## Casting

Since `Impl<ABI>` is an address at runtime, it supports free
casting between `Impl` types and `addr` via `as`:

```edge
let token: Impl<IERC20> = 0xdead as Impl<IERC20>;

// Extract raw address
let raw: addr = token as addr;

// Reinterpret as a different ABI
let as_metadata: Impl<IERC20Metadata> = token as Impl<IERC20Metadata>;

// Widen to a composed type
let full: Impl<IERC20 + IERC20Metadata> = token as Impl<IERC20 + IERC20Metadata>;
```

All casts are compile-time only with no runtime cost. The
underlying value is always a 20-byte address.

## Construction

An `Impl` value is constructed via `as` casting (see Casting
above). The `as` keyword is a new addition to the language's
keyword set.

## Method Calls

```text
<external_method_call> ::=
    <expr> [<call_modifier>]* "." <ident> "(" [<expr> ("," <expr>)* [","]] ")" ;

<call_modifier> ::=
    | "." "value" "(" <expr> ")"
    | "." "gas" "(" <expr> ")"
    | "." "delegate" "(" ")" ;
```

Dependencies:

* `<expr>`
* `<ident>`

Method calls on `Impl<ABI>` values dispatch to external contracts.
The compiler generates the 4-byte function selector from the ABI
function signature, ABI-encodes the arguments into memory, and
emits the appropriate EVM call instruction.

The call modifiers `value`, `gas`, and `delegate` are not keywords.
They are recognized as method names on the internal call builder.

```edge
abi IERC20 {
    fn balanceOf(account: addr) -> (u256);
    mut fn transfer(to: addr, amount: u256) -> (bool);
}

contract Vault {
    let token: &s Impl<IERC20>;

    pub fn withdraw(to: addr, amount: u256) {
        match token.transfer(to, amount) {
            Result::Ok(success) => { }
            Result::Err(err) => { revert; }
        }
    }

    pub fn checkBalance() -> (u256) {
        match token.balanceOf(@address()) {
            Result::Ok(bal) => { return bal; }
            Result::Err(err) => { revert; }
        }
    }
}
```

## Call Modifiers

Call modifiers configure the EVM call instruction parameters.
They are chained between the expression and the terminal method
call. Modifiers are order-independent and are evaluated at
compile time.

### value

```edge
token.value(1000).transfer(to, amount)
```

Sets the `msg.value` for the call. Only valid with CALL
instruction (i.e., `mut` functions). Using `value` with
`delegate` is a compile error, as DELEGATECALL does not
accept a value parameter.

### gas

```edge
token.gas(50000).transfer(to, amount)
```

Sets the gas limit forwarded to the call. If omitted, all
available gas is forwarded via the GAS opcode.

### delegate

```edge
token.delegate().transfer(to, amount)
```

Switches the call instruction from CALL to DELEGATECALL.
The target contract's code executes in the caller's storage
context. Using `delegate` with `value` is a compile error.

### Combined

```edge
token.value(1000).gas(100000).transfer(to, amount)
```

Multiple modifiers may be chained in any order before the
terminal method call.

## Return Type

All external method calls return `Result<T, CallErr>` where `T`
is the return type declared in the ABI function signature. This
is mandatory; external calls can fail due to reverts, out-of-gas,
or invalid target code, and these failures must be handled
explicitly.

```edge
// Must handle the result
match token.transfer(to, amount) {
    Result::Ok(success) => { return success; }
    Result::Err(err) => { revert; }
}
```

`CallErr` contains the failure information from the call. The
success flag from the EVM CALL instruction serves as the
discriminant for the `Result` union.

## Instruction Selection

The EVM call instruction is selected based on the ABI function
declaration and call modifiers:

* Functions declared without `mut` use STATICCALL by default.
* Functions declared with `mut` use CALL by default.
* The `delegate` modifier overrides either to DELEGATECALL.

## Positional Validity

The `Impl` type is valid in different positions depending on
whether the inner identifier is an ABI or trait declaration.

When wrapping an ABI declaration, `Impl` is valid in all type
positions: function arguments, return types, local variables,
storage fields, and struct fields. At the EVM level, the value
is always an address.

```edge
contract Vault {
    let token: &s Impl<IERC20>;          // storage field

    pub fn getToken() -> (Impl<IERC20>) { // return type
        return token;
    }
}
```

When wrapping a trait declaration, `Impl` is only valid in
function argument position, where it desugars to a generic
type parameter.

```edge
// These two declarations are equivalent:
fn process(x: Impl<Serializable>) { }
fn process<T: Serializable>(x: T) { }

// Compile error: cannot store erased trait type
let x: &s Impl<Serializable>;
```

## Composition

Multiple interfaces may be composed using `+` to indicate
that a value conforms to all listed interfaces.

```edge
abi IERC20 {
    fn balanceOf(account: addr) -> (u256);
    mut fn transfer(to: addr, amount: u256) -> (bool);
}

abi IERC20Metadata {
    fn name() -> (b32);
    fn symbol() -> (b32);
}

fn inspect(token: Impl<IERC20 + IERC20Metadata>) {
    // Can call methods from both ABIs
    match token.balanceOf(@address()) {
        Result::Ok(bal) => { }
        Result::Err(err) => { revert; }
    }
    match token.name() {
        Result::Ok(n) => { }
        Result::Err(err) => { revert; }
    }
}
```

Composing ABI identifiers with trait identifiers is a compile
error. The `+` operator may only combine identifiers of the
same kind.

## Semantics

The `Impl` type bridges the gap between ABI declarations and
external contract interaction. An ABI declaration defines an
interface; `Impl<ABI>` creates a typed handle to an address
that is asserted to conform to that interface.

The type carries no runtime overhead. ABI encoding, selector
computation, and instruction selection are performed entirely
at compile time. The only runtime operations are memory writes
for argument encoding, the call instruction itself, and return
data decoding.

## New Keywords

This feature requires one new keyword:

* `as` — used for type casting in `<expr> "as" <type>`

For Option A, whether `Impl` is a new keyword or a builtin
type name is an open question (see Syntax Options above).
For Option B, `impl` is already a keyword. The call modifier
names (`value`, `gas`, `delegate`) are not keywords; they are
recognized contextually as method names on the call builder.
