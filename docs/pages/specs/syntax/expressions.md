---
title: Expressions
---

# Expressions

```text
<expression> ::=
    | <array_instantiation>
    | <array_index>
    | <struct_instantiation>
    | <tuple_instantiation>
    | <field_access>
    | <tuple_field_access>
    | <union_instantiation>
    | <pattern_match>
    | <arrow_function>
    | <function_call>
    | <binary_operation>
    | <unary_operation>
    | <ternary>
    | <literal>
    | <identifier>
    | <comptime_expression>
    | <path_expression>
    | <at_builtin>
    | <assign_expression>
    | <inline_asm>
    | "(" <expression> ")" ;
```

An `<expression>` is any construct that produces a value.

## Binary operations

```text
<binary_operation> ::= <expression> <binary_operator> <expression> ;
```

Binary operations use an infixed operator between two sub-expressions.
See [operators](./operators) for the full operator table and precedence.

## Unary operations

```text
<unary_operation> ::= <unary_operator> <expression> ;
```

Prefix unary operators: `-` (negation), `~` (bitwise NOT), `!` (logical NOT).

## Ternary

```text
<ternary> ::= <expression> "?" <expression> ":" <expression> ;
```

The ternary operator is right-associative. Both branches are full expressions.

## Literals

The `<literal>` non-terminal is defined in
[Literals](/specs/syntax/compile/literals). In expression context, the following
additional details apply:

- Integer literals support `_` as a visual separator (e.g. `1_000_000`). Type suffixes
  (e.g. `42u8`, `256u16`) are recognized by the lexer but currently silently discarded —
  the type is inferred from context or defaults to `u256`.
- String literals use either double or single quotes. Supported escape sequences:
  `\n`, `\t`, `\r`, `\\`, `\"`, `\'`.
- Hex and binary literals produce byte-array values (`Lit::Hex` and `Lit::Bin`
  respectively).

## Function calls

```text
<function_call> ::= <expression> ["::" "<" <type_signature> ("," <type_signature>)* ">"] "(" [<expression> ("," <expression>)*] ")" ;
```

Functions are called with parenthesized argument lists. Turbofish syntax
(`::<T, U>`) provides explicit type arguments.

## Field and index access

```text
<field_access> ::= <expression> "." <identifier> ;
<tuple_field_access> ::= <expression> "." <dec_digit>+ ;
<array_index> ::= <expression> "[" <expression> [":" <expression>] "]" ;
```

Dot access resolves struct fields by name or tuple fields by numeric index.
Array indexing supports both single-element access (`arr[i]`) and slicing
(`arr[start:end]`).

## Instantiation

```text
<struct_instantiation> ::= [<data_location>] <identifier> "{" <identifier> ":" <expression> ("," <identifier> ":" <expression>)* "}" ;
<tuple_instantiation> ::= [<data_location>] "(" [<expression> ("," <expression>)*] ")" ;
<array_instantiation> ::= [<data_location>] "[" [<expression> ("," <expression>)*] "]" ;
<union_instantiation> ::= <identifier> "::" <identifier> "(" [<expression> ("," <expression>)*] ")" ;
```

Struct, tuple, and array instantiations may be prefixed with a `<data_location>`
annotation. Union variants are instantiated with path syntax (`Type::Variant(args)`).

## Pattern matching expression

```text
<pattern_match> ::= <expression> "matches" <identifier> "::" <identifier> ["(" <identifier> ("," <identifier>)* ")"] ;
```

The `matches` keyword tests whether an expression matches a union variant,
optionally binding the variant's payload to identifiers. Commonly used in
`if` conditions.

## Arrow functions

```text
<arrow_function> ::= (<identifier> | "(" [<identifier> ("," <identifier>)*] ")") "=>" <code_block> ;
```

Arrow functions (closures) take identifier parameters and a brace-delimited body.

## Compile-time expressions

```text
<comptime_expression> ::= "comptime" "(" <expression> ")" ;
```

Wraps an expression for compile-time evaluation.

## Path expressions

```text
<path_expression> ::= <identifier> ("::" <identifier>)+ ;
```

Double-colon-separated identifier paths, used for module paths and union variant access.

## Builtin calls

```text
<at_builtin> ::= "@" <identifier> ["(" [<expression> ("," <expression>)*] ")"] ;
```

The `@` sigil invokes compiler builtins. The parser accepts any identifier
after `@`; validation of builtin names happens in later compiler stages.

## Assignment expression

```text
<assign_expression> ::= <expression> "=" <expression> ;
```

Assignment at the expression level (precedence 0, right-associative). Produces
`Expr::Assign`.

## Inline assembly

```text
<inline_asm> ::= "asm" "(" [<expression> ("," <expression>)*] ")" ["->" "(" [<identifier> ("," <identifier>)*] ")"] "{" <asm_op>* "}" ;

<asm_op> ::= <evm_opcode> | <integer_literal> | <identifier> ;
```

Inline assembly provides direct access to EVM opcodes. Inputs are pushed onto
the stack (leftmost = top of stack). Outputs are optionally bound to identifiers;
use `_` to discard a stack value.
