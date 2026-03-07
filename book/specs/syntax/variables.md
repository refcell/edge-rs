# Variables

## Declaration

```
<variable_declaration> ::= "let" <ident> [":" <type_signature>] ["=" <expr>] ";"
```

Dependencies:

* `<ident>`
* `<type_signature>`
* `<expr>`

The `<variable_declaration>` marks the declaration of a variable.
It may optionally include a type annotation and an initializer expression.
When an initializer is provided, the variable is assigned the value of
the expression at the point of declaration.

```
let x: u256;           // declaration only
let y: u256 = 42;      // declaration with initialization
let z = x + y;         // declaration with type inference (future)
```

## Assignment

```
<variable_assignment> ::= <ident> "=" <expr> ;
```

Dependencies:

* `<ident>`
* `<expr>`

The `<variable_assignment>` is the assignment of a variable.
Its identifier is assigned the returned value of an expression using
the assignment operator.
