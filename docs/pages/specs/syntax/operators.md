---
title: Operators
---

# Operators

Operators are syntax sugar over built-in functions. Operator overloading is disallowed.

## Binary operators

```text
<arithmetic_binary_operator> ::=
    | "+" | "-" | "*" | "/" | "%" | "**" ;

<bitwise_binary_operator> ::=
    | "&" | "|" | "^" | "<<" | ">>" ;

<comparison_binary_operator> ::=
    | "==" | "!=" | "<" | "<=" | ">" | ">=" ;

<logical_binary_operator> ::=
    | "&&" | "||" ;

<compound_assignment_operator> ::=
    | "+=" | "-=" | "*=" | "/=" | "%=" | "**="
    | "&=" | "|=" | "^=" | "<<=" | ">>=" ;

<binary_operator> ::=
    | <arithmetic_binary_operator>
    | <bitwise_binary_operator>
    | <comparison_binary_operator>
    | <logical_binary_operator>
    | <compound_assignment_operator> ;
```

## Unary operators

```text
<arithmetic_unary_operator> ::= "-" ;

<bitwise_unary_operator> ::= "~" ;

<logical_unary_operator> ::= "!" ;

<unary_operator> ::=
    | <arithmetic_unary_operator>
    | <bitwise_unary_operator>
    | <logical_unary_operator> ;
```

## Precedence

The expression parser uses precedence climbing (Pratt parsing). Lower numbers
bind less tightly:

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 0 | `=` | Right |
| 1 | `\|\|` | Left |
| 2 | `&&` | Left |
| 3 | `==` `!=` | Left |
| 4 | `<` `>` `<=` `>=` | Left |
| 5 | `\|` (bitwise OR) | Left |
| 6 | `^` (bitwise XOR) | Left |
| 7 | `&` (bitwise AND) | Left |
| 8 | `<<` `>>` | Left |
| 9 | `+` `-` | Left |
| 10 | `*` `/` `%` | Left |
| 11 | `**` | Right |

The ternary operator (`? :`) is parsed after the Pratt binary expression,
with right-to-left associativity.

Compound assignment operators (`+=`, `-=`, etc.) are parsed as binary
operations and produce `Expr::Binary` nodes with the corresponding
`BinOp` variant.

## Semantics

| Operator | Types | Behavior | Panic case |
|----------|-------|----------|------------|
| `+` | integers | checked addition | overflow |
| `-` (binary) | integers | checked subtraction | underflow |
| `-` (unary) | integers | checked negation | overflow |
| `*` | integers | checked multiplication | overflow |
| `/` | integers | checked division | divide by zero |
| `%` | integers | checked modulus | divide by zero |
| `**` | integers | exponentiation | — |
| `&` | integers | bitwise AND | — |
| `\|` | integers | bitwise OR | — |
| `~` | integers | bitwise NOT | — |
| `^` | integers | bitwise XOR | — |
| `>>` | integers | bitwise shift right | — |
| `<<` | integers | bitwise shift left | — |
| `==` | any | equality | — |
| `!=` | any | inequality | — |
| `&&` | booleans | logical AND | — |
| `\|\|` | booleans | logical OR | — |
| `!` | booleans | logical NOT | — |
| `>` | integers | greater than | — |
| `>=` | integers | greater than or equal | — |
| `<` | integers | less than | — |
| `<=` | integers | less than or equal | — |
