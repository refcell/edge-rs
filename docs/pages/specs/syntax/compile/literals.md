---
title: Literals
---

# Literals

## Characters

```text
<bin_digit> ::= "0" | "1" ;
<dec_digit> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
<hex_digit> ::=
    | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "a"
    | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F" ;
<alpha_char> ::=
    | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p"
    | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" ;
<alphanumeric_char> ::= <alpha_char> | <dec_digit> ;

<unicode_char> ::= ? any valid Unicode scalar value ? ;
```

## Numeric literals

```text
<dec_literal> ::= { (<dec_digit> | "_")+ } ;
<hex_literal> ::= { "0x" (<hex_digit> | "_")+ } ;

<numeric_literal> ::= <dec_literal> | <hex_literal> ;
```

Numeric literals are composed of decimal or hexadecimal digits. Each literal
may contain underscores for readability. Hexadecimal literals are prefixed
with `0x`.

The parser stores integer literals as `Lit::Int(u64, Option<PrimitiveType>, Span)`.

:::warning[Known limitations]
- **Integer range cap (`u64`):** The parser stores integer literal values as
  a Rust `u64`. Any literal value larger than 2⁶⁴ − 1 cannot be represented
  as a compile-time constant — even though the language defaults to `u256`.
  Large constants must currently be expressed through arithmetic or runtime
  construction.

- **Type suffixes silently discarded:** Integer type suffixes such as `1u8`
  or `0xffu128` are recognized by the lexer but the suffix annotation is
  silently dropped. The AST always stores the literal as
  `Lit::Int(value, None, span)`. Type is inferred from context or defaults
  to `u256`.
:::

## Binary literals

```text
<bin_literal> ::= { "0b" (<bin_digit> | "_")+ } ;
```

Binary literals are prefixed with `0b` and produce `Lit::Bin(Vec<u8>, Span)`.

## String literals

```text
<string_literal> ::= ('"' (!'"' <unicode_char>)* '"') | ("'" (!"'" <unicode_char>)* "'") ;
```

String literals may use either double or single quotes. Both forms support
escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`.

String literals produce `Lit::Str(String, Span)`.

## Boolean literals

```text
<boolean_literal> ::= "true" | "false" ;
```

:::note
`Lit::Bool(bool, Span)` exists in the AST, but the lexer never constructs
it. The `true` and `false` keywords are currently parsed as keyword
identifiers and resolve to integer constants (1 and 0 respectively) during
compilation.
:::

## Literal

```text
<literal> ::= <numeric_literal> | <bin_literal> | <hex_literal> | <string_literal> | <boolean_literal> ;
```

The `<literal>` maps to `Lit` variants in the AST:

| Syntax | AST variant |
|---|---|
| `42`, `0xFF` | `Lit::Int(u64, Option<PrimitiveType>, Span)` |
| `"hello"` | `Lit::Str(String, Span)` |
| `true`, `false` | (see note above) |
| `0xDEADBEEF` (bytes) | `Lit::Hex(Vec<u8>, Span)` |
| `0b10101010` | `Lit::Bin(Vec<u8>, Span)` |

## Semantics

Numeric literals may contain arbitrary underscores. The type of a numeric
literal is inferred from context; if no type can be inferred, it defaults
to `u256`.

Both numeric and boolean literals are roughly translated to pushing the
value onto the EVM stack.

String literals represent string instantiation, which behaves as a packed
`u8` array instantiation.

```edge
const A = 1;
const B = 0xffFFff;
const C = true;
const D = "asdf";
const E = "💩";
```
