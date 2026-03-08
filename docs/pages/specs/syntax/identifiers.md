---
title: Identifiers
---

# Identifiers

```text
<identifier> ::= (<alpha_char> | "_") (<alpha_char> | <dec_digit> | "_")* ;
```

Dependencies:

* `<alpha_char>`
* `<dec_digit>`

The `<identifier>` is a C-style identifier, beginning with an alphabetic character
or underscore, followed by zero or more alphanumeric or underscore characters.

## Reserved names

Identifiers share their lexical space with keywords, primitive type names, and
boolean literals. The lexer resolves ambiguity in the following priority order:

1. **EVM primitive type** — `u8`–`u256`, `i8`–`i256`, `b1`–`b32`, `addr`, `bool`, `bit`
2. **Keyword** — e.g. `let`, `fn`, `contract`, `mod`, `use`, `mut`, `pub`, `Self`, …
3. **Boolean literal** — `true`, `false`
4. **Identifier** — everything else

Any string that matches a higher-priority rule will **never** produce an
`Ident` token. In particular, `Self` (capital S) is a reserved keyword and
cannot be used as a plain identifier.

## Special identifiers

The parser accepts `self` and `super` as identifiers in certain contexts
(e.g. module paths, method receivers). These are keywords but are returned
as identifier nodes with the names `"self"` and `"super"` respectively.
