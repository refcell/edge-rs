---
title: Literals
---

# Literals

## Characters

```text
<bin_char> ::= "0" | "1" ;
<dec_char> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;
<hex_char> ::=
    | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "a"
    | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F";
<alpha_char> ::=
    | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p"
    | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" ;
<alphanumeric_char> ::= <alpha_char> | <dec_char> ;

<unicode_char> ::= ? "i ain't writing all that. happy for you tho. or sorry that happened" ? ;
```

## Numeric

```text
<bin_literal> ::= { "0b" (<bin_char> | "_")+ [<numeric_type>]} ;
<dec_literal> ::= { (<dec_char> | "_")+ [<numeric_type>]} ;
<hex_literal> ::= { "0x" (<hex_char> | "_")+ [<numeric_type>]} ;

<numeric_literal> ::= <bin_literal> | <dec_literal> | <hex_literal> ;
```

Numeric literals are composed of binary, decimal, and hexadecimal digits.
Each digit may contain an arbitrary number of underscore characters in
them and may be suffixed with a numeric type.

Binary literals are prefixed with 0b and hexadecimal literals are prefixed
with `0x`.

## String

```text
<string_literal> ::= { '"' (!'"' <unicode_char>)* '"' } | { "'" (!"'" <unicode_char>)* "'" };
```

String literals contain alphanumeric characters delimited by double or
single quotes.

## Boolean

```text
<boolean_literal> ::= "true" | "false" ;
```

Boolean literals may be either "true" or "false".

## Literal

```text
<literal> ::= <numeric_literal> | <string_literal> | <boolean_literal> ;
```

## Semantics

Numeric literals may contain arbitrary underscores in the same literal.
Numeric literals may also be suffixed with the numeric type to constrain
its type. If there is no type suffix, the type is inferred by the context.
If a type cannot be inferred, it will default to a u256.

Both numeric and boolean literals are roughly translated to pushing the
value onto the stack.

String literals represent string instantiation. String instantiation
behaves as a packed u8 array instantiation.

```edge
const A = 1;
const B = 1u8;
const C = 0b11001100;
const D = 0xffFFff;
const E = true;
const F = "asdf";
const G = "💩";
```
