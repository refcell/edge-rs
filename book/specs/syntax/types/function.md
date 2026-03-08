# Function Types

The function type is a type composed of input and output types.

## Signature

```
<function_signature> ::= <type_signature> "->" <type_signature> ;
```

Dependencies:

* `<type_signature>`

The `<function_signature>` consists of an input type signature
and an output type signature, separated by an arrow.

Note: `<type_signature>` also contains a tuple signature,
therefore a function with multiple inputs and outputs is
implicitly operating on a tuple.

## Declaration

```
<function_param> ::= <ident> [":" <type_signature>] ;

<function_declaration> ::=
    "fn" <ident> "("
        [<function_param> ("," <function_param>)* [","]]
    ")" ["->" "(" <type_signature> ("," <type_signature>)* [","] ")"] ;
```

Dependencies:

* `<ident>`
* `<type_signature>`

A parameter may omit its type annotation, in which case the type
defaults to `Self`. This is intended for use with `self` in trait
and impl methods:

```edge
// These are equivalent:
fn add(self, rhs: Self) -> (Self);
fn add(self: Self, rhs: Self) -> (Self);
```

## Assignment

```
<function_assignment> ::= <function_declaration> <code_block> ;
```

Dependencies:

* `<code_block>`

The `<function_assignment>` is defined as the "fn" keyword followed
by its identifier, followed by optional comma separated pairs of
identifiers and type signatures, delimited by parenthesis, then
optionally followed by an arrow and a list of comma separated return
types signatures delimited by parenthesis, then finally the code
block of the function body.

## Arrow Functions

```
<arrow_function> ::= (<ident> | ("(" <ident> ("," <ident>)* [","] ")")) "=>" <code_block> ;
```

Dependencies:

* `<ident>`
* `<code_block>`

The `<arrow_function>` is defined as either a single identifier
or a comma separated, parenthesis delimited list of identifiers,
followed by the "=>" bigram, followed by a code block.

## Call

```
<function_call> ::= <ident> "(" [<expr> ("," <expr>) [","]] ")" ;
```

Dependencies:

* `<ident>`
* `<expr>`

The `<function_call>` is an identifier followed by a comma
separated list of expressions delimited by parenthesis.

## Semantics

> todo
