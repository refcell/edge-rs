---
title: Variables
---

# Variables

## Declaration

```text
<variable_declaration> ::= "let" <ident> [":" <type_signature>] ;
```

Dependencies:

* `<ident>`
* `<type_signature>`

The `<variable_declaration>` marks the declaration of a variable,
it may optionally be assigned at the time of declaration.

## Assignment

```text
<variable_assignment> ::= <ident> "=" <expr> ;
```

Dependencies:

* `<ident>`
* `<expr>`

The `<variable_assignment>` is the assignment of a variable.
Its identifier is assigned the returned value of an expression using
the assignment operator.
