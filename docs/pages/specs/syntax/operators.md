---
title: Operators
---

# Operators

Operators are syntax sugar over built-in functions.

## Binary

```text
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

```text
<arithmetic_unary_operator> ::= "-" ;

<bitwise_unary_operator> ::= "~" ;

<logical_unary_operator> ::= "!" ;

<unary_operator> ::=
    | <arithmetic_unary_operator>
    | <bitwise_unary_operator>
    | <logical_unary_operator> ;
```

## Semantics

Operator overloading is disallowed.

| operator | types    | behavior                     | panic case     |
| ---------|----------|------------------------------|----------------|
| `+`      | integers | checked addition             | overflow       |
| `-`      | integers | checked subtraction (binary) | underflow      |
| `-`      | integers | checked negation (unary)     | overflow       |
| `*`      | integers | checked multiplication       | overflow       |
| `/`      | integers | checked division             | divide by zero |
| `%`      | integers | checked modulus              | divide by zero |
| `**`     | integers | exponentiation               | -              |
| `&`      | integers | bitwise AND                  | -              |
| `\|`     | integers | bitwise OR                   | -              |
| `~`      | integers | bitwise NOT                  | -              |
| `^`      | integers | bitwise XOR                  | -              |
| `>>`     | integers | bitwise shift right          | -              |
| `<<`     | integers | bitwise shift left           | -              |
| `==`     | any      | equality                     | -              |
| `!=`     | any      | inequality                   | -              |
| `&&`     | booleans | logical AND                  | -              |
| `\|\|`   | booleans | logical OR                   | -              |
| `!`      | booleans | logical NOT                  | -              |
| `>`      | integers | greater than                 | -              |
| `>=`     | integers | greater than or equal to     | -              |
| `<`      | integers | less than                    | -              |
| `<=`     | integers | less than or equal to        | -              |
