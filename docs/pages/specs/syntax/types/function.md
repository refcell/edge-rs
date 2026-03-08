---
title: Function types
---

# Function types

The function type is a type composed of input and output types.

## Signature

```text
<function_signature> ::= <type_signature> "->" <type_signature> ;
```

Dependencies:

* `<type_signature>`

The `<function_signature>` maps to `TypeSig::Function(input, output)`.
Since `<type_signature>` includes tuple signatures, a function with
multiple inputs or outputs implicitly operates on a tuple.

## Declaration

```text
<function_declaration> ::=
    ["pub"] ["ext"] ["mut"]
    "fn" <identifier>
    ["<" <type_param> ("," <type_param>)* ">"]
    "("
        [(<identifier> ":" <type_signature>) ("," <identifier> ":" <type_signature>)* [","]]
    ")"
    ["->" "(" <type_signature> ("," <type_signature>)* [","] ")"] ;

<type_param> ::= <identifier> [":" <identifier> ("+" <identifier>)*] ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<function_declaration>` maps to `FnDecl` in the AST with these fields:

- `name: Ident`
- `type_params: Vec<TypeParam>`
- `params: Vec<(Ident, TypeSig)>`
- `returns: Vec<TypeSig>`
- `is_pub: bool`, `is_ext: bool`, `is_mut: bool`

The `pub`, `ext`, and `mut` keywords are **independent** — each sets a
separate boolean flag. They may appear in any combination:

```edge
pub fn read() -> u256 { ... }
pub ext fn deposit() { ... }
pub mut fn transfer() { ... }
pub ext mut fn swap() { ... }
```

The `self` keyword may appear as a parameter name, in which case the
`: Type` annotation is optional (implicit `Self` type).

## Assignment

```text
<function_assignment> ::= <function_declaration> <code_block> ;
```

Dependencies:

* `<code_block>`

The `<function_assignment>` is a function declaration followed by a code
block body. It produces `Stmt::FnAssign(FnDecl, CodeBlock)`.

## Arrow functions

```text
<arrow_function> ::= (<identifier> | "(" [<identifier> ("," <identifier>)* [","]] ")") "=>" <code_block> ;
```

Dependencies:

* `<identifier>`
* `<code_block>`

Arrow functions produce `Expr::ArrowFunction(params, body, span)`. The body
must be a brace-delimited code block. Supported forms:

```edge
x => { x + 1 }
(x, y) => { x + y }
() => { 42 }
```

## Call

```text
<function_call> ::=
    <expression>
    ["::" "<" <type_signature> ("," <type_signature>)* ">"]
    "(" [<expression> ("," <expression>)* [","]] ")" ;
```

Dependencies:

* `<expression>`
* `<type_signature>`

The `<function_call>` produces `Expr::FunctionCall(callee, args, type_args, span)`.
The callee is any expression — supporting method calls (`obj.method()`),
higher-order calls (`get_fn()()`), and turbofish instantiations
(`foo::<u32>(...)`).

## Compile-time functions

```text
<comptime_function_assignment> ::= "comptime" <function_assignment> ;
```

The `comptime` keyword before a function assignment declares a compile-time
function. This produces `Stmt::ComptimeFn(FnDecl, CodeBlock)`, which is
distinct from `Stmt::FnAssign`. See [compile-time functions](/specs/syntax/compile/functions).

## Semantics

:::warning
The function-type semantics section is still under construction. Runtime
calling conventions, ABI encoding, and stack-frame layout are not yet
documented.
:::
