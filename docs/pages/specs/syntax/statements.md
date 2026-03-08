---
title: Statements
---

# Statements

```text
<statement> ::=
    | <variable_declaration>
    | <variable_assignment>
    | <type_assignment>
    | <trait_declaration>
    | <impl_block>
    | <function_assignment>
    | <abi_declaration>
    | <contract_declaration>
    | <contract_impl>
    | <constant_assignment>
    | <module_declaration>
    | <module_import>
    | <core_loop>
    | <for_loop>
    | <while_loop>
    | <do_while_loop>
    | <code_block>
    | <if_else_branch>
    | <if_match_branch>
    | <match>
    | <comptime_branch>
    | <comptime_function>
    | <return>
    | <break>
    | <continue>
    | <event_declaration>
    | <emit_statement>
    | <expression> ";" ;
```

A `<statement>` is a language construct that does not itself produce a value
(unlike an expression). The top-level parse loop collects statements until EOF.

## Control flow statements

```text
<return> ::= "return" [<expression>] ";" ;
<break> ::= "break" ";" ;
<continue> ::= "continue" ";" ;
```

## Code blocks

```text
<code_block> ::= "{" (<statement> | <expression> ";")* [<expression>] "}" ;
```

A code block is a brace-delimited sequence of statements. The final item
may be a bare expression without a trailing semicolon (tail expression),
which becomes the block's value — similar to Rust.

:::note
At the AST level, tail expressions are wrapped as `BlockItem::Stmt(Stmt::Expr(…))`.
There is no distinct AST node for tail expressions; the semantic difference is
inferred from position.
:::

## If / else

```text
<if_else_branch> ::= "if" "(" <expression> ")" <code_block> ("else" "if" "(" <expression> ")" <code_block>)* ["else" <code_block>] ;

<if_match_branch> ::= "if" <expression> "matches" <identifier> "::" <identifier> ["(" <identifier> ("," <identifier>)* ")"] <code_block> ;
```

The standard `if`/`else if`/`else` chain uses parenthesized conditions and
brace-delimited bodies. The `if … matches` form combines a conditional with
union pattern destructuring.

:::note
The `Stmt::IfMatch` variant exists in the AST, but the current parser produces
`Stmt::IfElse` with an `Expr::PatternMatch` as the condition instead. The
dedicated variant is reserved for future use.
:::

## Match

```text
<match> ::= "match" <expression> "{" <match_arm> ("," <match_arm>)* [","] "}" ;

<match_arm> ::= <match_pattern> "=>" (<code_block> | <expression> | "return" [<expression>]) ;

<match_pattern> ::= <union_pattern> | <identifier> | "_" ;

<union_pattern> ::= <identifier> "::" <identifier> ["(" <identifier> ("," <identifier>)* ")"] ;
```

Match arms accept a code block, a bare expression, or a `return` statement as
the body. At the AST level, all arm bodies are normalized to `CodeBlock`.

## Loops

```text
<core_loop> ::= "loop" <loop_block> ;

<for_loop> ::= "for" "(" [<statement> | <expression>] ";" [<expression>] ";" [<statement> | <expression>] ")" <loop_block> ;

<while_loop> ::= "while" "(" <expression> ")" <loop_block> ;

<do_while_loop> ::= "do" <loop_block> "while" "(" <expression> ")" ";" ;

<loop_block> ::= "{" (<statement> | <expression> ";" | "break" ";" | "continue" ";")* "}" ;
```

The `<loop_block>` uses a separate AST type (`LoopBlock` / `LoopItem`) from
regular code blocks. `break` and `continue` have dedicated `LoopItem` variants
in addition to the `Stmt::Break` / `Stmt::Continue` variants used outside loops.

:::warning
`break` and `continue` are parsed but not yet implemented in the compiler
backend. They will silently compile as if the statement were absent.
:::

## Contracts

```text
<contract_declaration> ::= "contract" <identifier> "{" <contract_member>* "}" ;

<contract_member> ::=
    | "let" <identifier> ":" <type_signature> ";"
    | "const" <identifier> [":" <type_signature>] "=" <expression> ";"
    | ["pub"] ["ext"] ["mut"] "fn" <identifier> "(" [<param_list>] ")" ["->" <return_type>] <code_block> ;

<contract_impl> ::= "impl" <identifier> [":" <identifier>] "{" <contract_fn>* "}" ;
```

Contract bodies contain storage field declarations (`let`), constants, and
function definitions. The `impl` block provides the implementation for a
contract, optionally satisfying an ABI interface.

## Functions

```text
<function_assignment> ::= ["pub"] ["ext"] ["mut"] "fn" <identifier> ["<" <type_param> ("," <type_param>)* ">"] "(" [<param_list>] ")" ["->" <return_type>] <code_block> ;

<param_list> ::= <identifier> ":" <type_signature> ("," <identifier> ":" <type_signature>)* ;

<return_type> ::= <type_signature> | "(" <type_signature> ("," <type_signature>)* ")" ;

<type_param> ::= <identifier> [":" <identifier> ("&" <identifier>)*] ;
```

Functions support generic type parameters with trait bounds (`<T: Trait & Other>`).
The `self` keyword may appear as the first parameter without a type annotation
(implicit `Self` type). Return types can be a single type or a tuple.

Visibility and modifier flags:
- `pub` — public visibility
- `ext` — external ABI entry point
- `mut` — may mutate contract state

## Type aliases

```text
<type_assignment> ::= "type" <identifier> ["<" <type_param> ("," <type_param>)* ">"] "=" <type_signature_or_union> ";" ;

<type_signature_or_union> ::= <type_signature> | <union_type> ;

<union_type> ::= ["|"] <union_member> ("|" <union_member>)+ ;

<union_member> ::= <identifier> ["(" <type_signature> ")"] ;
```

Type aliases bind a name to a type signature or a union type. Union types
define sum types with named variants that optionally carry a payload.

## Traits and implementations

```text
<trait_declaration> ::= "trait" <identifier> ["<" <type_param> ("," <type_param>)* ">"] [":" <identifier> ("+" <identifier>)*] "{" <trait_item>* "}" ;

<trait_item> ::=
    | "fn" <identifier> "(" [<param_list>] ")" ["->" <return_type>] (";" | <code_block>)
    | "const" <identifier> ":" <type_signature> ["=" <expression>] ";"
    | "type" <identifier> ["=" <type_signature>] ";" ;

<impl_block> ::= "impl" <identifier> ["<" <type_param> ("," <type_param>)* ">"] [":" <identifier> ["<" <type_param> ("," <type_param>)* ">"]] "{" <impl_item>* "}" ;

<impl_item> ::=
    | ["pub"] "fn" <identifier> "(" [<param_list>] ")" ["->" <return_type>] <code_block>
    | ["pub"] "const" <identifier> ":" <type_signature> "=" <expression> ";"
    | ["pub"] "type" <identifier> "=" <type_signature> ";" ;
```

Traits declare abstract interfaces with optional default implementations.
Supertraits use `+` syntax: `trait Ordered: Comparable + Displayable { … }`.
Implementation blocks provide concrete implementations for types, optionally
satisfying a trait: `impl Type : Trait { … }`.

## ABI declarations

```text
<abi_declaration> ::= "abi" <identifier> [":" <identifier> ("+" <identifier>)*] "{" <abi_fn>* "}" ;

<abi_fn> ::= ["mut"] "fn" <identifier> "(" [<param_list>] ")" ["->" <return_type>] ";" ;
```

ABI declarations define external interfaces. They are similar to traits but
specific to the EVM calling convention. Superabis are supported with the
same `+` syntax as supertraits.

## Events and emit

```text
<event_declaration> ::= ["anon"] "event" <identifier> "(" [<event_field> ("," <event_field>)*] ")" ";" ;

<event_field> ::= ["indexed"] <identifier> ":" <type_signature> ;

<emit_statement> ::= "emit" <identifier> "(" [<expression> ("," <expression>)*] ")" ";" ;
```

Events declare log schemas. Fields may be marked `indexed` for topic-based
filtering. The `anon` modifier creates an anonymous event (no topic0 selector).
The `emit` statement fires an event with the given arguments.

:::warning
Anonymous events (`anon event`) are parsed but the `is_anon` flag is always
set to `false` by the current parser. This feature is reserved for future use.
:::

## Compile-time constructs

```text
<comptime_branch> ::= "comptime" <statement> ;
<comptime_function> ::= "comptime" "fn" <identifier> "(" [<param_list>] ")" ["->" <return_type>] <code_block> ;
```

`comptime` can prefix a statement for compile-time conditional compilation,
or prefix a function declaration to define a compile-time function.

:::warning
Compile-time constructs are parsed but have limited backend support. Only
constant expression evaluation (integer arithmetic and bitwise operations)
is currently implemented.
:::
