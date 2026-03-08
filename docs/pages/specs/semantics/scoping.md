---
title: Scoping
---

# Scoping

Items are brought into scope by import or declaration.

## Module

The module scope contains items explicitly imported from another
scope or explicitly declared in the current module scope. Items may
be accessed directly by their identifier with no other annotations.

Files are implicitly modules.

:::warning
The examples below use `mod`, `pub use`, and path syntax (`super::moduleA::TypeA`)
to illustrate scoping concepts. These features are **planned but not yet
implemented** in the parser — see [Modules](/specs/syntax/modules) for details.
:::

```edge
mod moduleA {
    // `TypeA` declared.
    type TypeA = u8;
    // `TypeA` may be accessed as follows:
    const CONST_A: TypeA = 0u8;
}

mod moduleB {
    // import `TypeA` into the local module scope
    use super::moduleA::TypeA;
    // `TypeA` may now be accessed as follows:
    const CONST_A: TypeA = 0u8;
}

mod moduleC {
    // publicly import `TypeA` into the local module scope. "pub" enables exporting.
    pub use super::moduleA::TypeA;
}

mod moduleD {
    // publicly import `moduleA` into the local module scope. "pub" enables exporting.
    pub use super::moduleA;
}

mod moduleF {
    // `TypeA` may be accessed in one of the following ways.
    const CONST_A: super::moduleA::TypeA = 0u8;
    const CONST_B: super::moduleC::TypeA = 0u8;
    const CONST_C: super::moduleD::moduleA::TypeA = 0u8;
}
```

## Implementation

The implementation block scope contains items explicitly imported from
another scope or explicitly declared in the current implementation block
scope. Items may be accessed either directly or under the Self namespace.

```edge
type MyStruct<T> = { inner: T };
type MyError = Overflow | Underflow;

trait TryPlusOne: Add {
    type Error;

    fn tryPlusOne(self: Self) -> Result<Self, Self::Error>;
}

impl MyStruct<T> {
    fn new(inner: T) -> Self {
        return Self { inner: T };
    }
}

impl MyStruct<T>: Add {
    fn add(lhs: Self, rhs: Self) -> Self {
        return Self { inner: lhs.inner + rhs.inner };
    }
}

impl MyStruct<T: Add>: TryPlusOne {
    type Error = MyError;

    fn tryPlusOne(self: Self) -> Result<Self, Self::Error> {
        if self.inner > max<T>() - 1 {
            return Result::Err(Error::Overflow);
        }
        return Add::add(self, Self { inner: 1 });
    }
}
```

## Function

The function scope implicitly imports items from parent scopes up to
the parent module. Items may be explicitly declared or imported from
external modules.

```edge
mod moduleA {
    const CONST_A = 0u8;
    const CONST_B = 1u8;
    const CONST_C = 2u8;
}

use moduleA::CONST_A;

const CONST_D = 3u8;

fn func() -> u8 {
    use moduleA::CONST_B;

    fn innerFunc() -> u8 {
        return CONST_A + CONST_B + moduleA::CONST_C + CONST_D;
    }

    return innerFunc();
}
```

## Blocks

Code blocks, branch blocks, loop blocks, and match blocks
implicitly import items from the parent scopes up until the
parent module. Items may be imported from external modules
explicitly and items may be defined in each.

## IR lowering and name uniqueness

When source code is lowered to the Edge IR, all scoped names are resolved to
unique flat strings. The IR uses plain string identifiers for all `LetBind`
variables, `Function` definitions, and `Call` targets — there is no nested
scope structure in the IR.

The frontend lowering pass is responsible for:

- Resolving module paths (`super::moduleA::TypeA`) to their canonical names.
- Ensuring local variables declared in different scopes (or after function
  inlining) have unique names by appending suffixes as needed.
- Lowering inner functions (e.g., `fn innerFunc()` declared inside `fn func()`)
  to top-level named `Function` nodes in the IR.

At optimization level O1+, the inliner renames all local variables in inlined
function bodies (appending a unique suffix like `_s0`) to prevent collisions
with variables at the call site.

See [Visibility](/specs/semantics/visibility) for how `pub`, `pub ext`, and
`pub mut` interact with scope boundaries and EVM dispatch.
