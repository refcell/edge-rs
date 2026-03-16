# Operators

Operators are syntax sugar over built-in functions.

## Binary

```
<arithmetic_binary_operator> ::=
    | "+" | "+="
    | "-" | "-="
    | "*" | "*="
    | "/" | "/="
    | "%" | "%="
    | "**" | "**=" ;
<bitwise_binary_operator> ::=
    | "|" | "|="
    | ">>" | ">>="
    | "<<" | "<<="
    | "&" | "&="
    | "^" | "^=" ;

<logical_binary_operator> ::=
    | "=="
    | "!="
    | "&&"
    | "||"
    | ">" | ">="
    | "<" | "<=" ;

<binary_operator> ::=
    | <arithmetic_binary_operator>
    | <bitwise_binary_operator>
    | <logical_binary_operator> ;
```

## Unary

```
<arithmetic_unary_operator> ::= "-" ;

<bitwise_unary_operator> ::= "~" ;

<logical_unary_operator> ::= "!" ;

<unary_operator> ::=
    | <arithmetic_unary_operator>
    | <bitwise_unary_operator>
    | <logical_unary_operator> ;
```

## Index

```
<index_operator> ::= "[" <expr> "]" ;
```

The index operator `[]` dispatches to the `Index` trait. Any type implementing
`Index<Idx, Output>` can be indexed with `value[key]`.

## Semantics

| operator | types    | behavior                     | panic case     |
| ---------|----------|------------------------------|----------------|
| +        | integers | checked addition             | overflow       |
| -        | integers | checked subtraction (binary) | underflow      |
| -        | integers | checked negation (unary)     | overflow       |
| *        | integers | checked multiplication       | overflow       |
| /        | integers | checked division             | divide by zero |
| %        | integers | checked modulus              | divide by zero |
| **       | integers | exponentiation               | -              |
| &        | integers | bitwise AND                  | -              |
| \|       | integers | bitwise OR                   | -              |
| ~        | integers | bitwise NOT                  | -              |
| ^        | integers | bitwise XOR                  | -              |
| >>       | integers | bitwise shift right          | -              |
| <<       | integers | bitwise shift left           | -              |
| ==       | any      | equality                     | -              |
| !=       | any      | inequality                   | -              |
| &&       | booleans | logical AND                  | -              |
| \|\|     | booleans | logical OR                   | -              |
| !        | booleans | logical NOT                  | -              |
| >        | integers | greater than                 | -              |
| >=       | integers | greater than or equal to     | -              |
| <        | integers | less than                    | -              |
| <=       | integers | less than or equal to        | -              |
| []       | any      | index                        | -              |

### Checked Arithmetic

The `+`, `-`, and `*` operators are **checked** by default: they revert on
overflow or underflow. The compiler's range analysis pass can **elide** these
checks when it can statically prove the operation is safe (e.g., adding two
values whose combined upper bound fits in the type). This happens
automatically at optimization level O1 and above.

For performance-critical code where overflow is known to be impossible, the
standard library provides unchecked variants via the `UnsafeAdd`, `UnsafeSub`,
and `UnsafeMul` traits.

### Operator Overloading

Direct operator overloading is disallowed. However, the following operators
can be customized for user-defined types by implementing the corresponding
standard library trait from `std::ops`:

| operator      | trait                      | method(s)            |
|---------------|----------------------------|----------------------|
| `+`           | `Add`                      | `add(self, rhs)`     |
| `-`           | `Sub`                      | `sub(self, rhs)`     |
| `*`           | `Mul`                      | `mul(self, rhs)`     |
| `/`           | `Div`                      | `div(self, rhs)`     |
| `%`           | `Mod`                      | `mod_(self, rhs)`    |
| `==`          | `Eq`                       | `eq(self, rhs)`      |
| `<` `<=` `>` `>=` | `Ord`                 | `lt` `le` `gt` `ge` |
| `[]`          | `Index<Idx, Output>`       | `index(self, idx)`   |

Bitwise and logical operators (`&`, `|`, `^`, `~`, `<<`, `>>`, `&&`, `||`)
are currently primitives-only. Exponentiation (`**`) is also primitives-only.
Trait-based overloading for these operators may be added in the future.

Example:

```edge
use std::ops::Add;
use std::ops::Eq;

type Wrapper = { value: u256 };

impl Wrapper: Add {
    fn add(self, rhs: Self) -> (Self) {
        Wrapper { value: self.value + rhs.value }
    }
}

impl Wrapper: Eq {
    fn eq(self, rhs: Self) -> (bool) {
        self.value == rhs.value
    }
}

// Now `Wrapper { value: 1 } + Wrapper { value: 2 }` works.
```
