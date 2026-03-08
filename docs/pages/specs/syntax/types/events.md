---
title: Event types
---

# Event types

The event type is a custom type to be logged via EVM log opcodes.

## Inline event signature

```text
<event_field_signature> ::= ["indexed"] <identifier> ":" <type_signature> ;

<event_signature> ::=
    ["anon"] "event" "{" [<event_field_signature> ("," <event_field_signature>)* [","]] "}" ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<event_signature>` is an inline type that maps to
`TypeSig::Event(is_anon, Vec<EventField>)`. Each field has `indexed: bool`
and `ty: TypeSig`. The optional `indexed` keyword precedes the field name.

:::note
Edge uses two representations for events: the **inline event signature**
(a `TypeSig` variant usable in type assignments) and the **standalone event
declaration** (an `EventDecl` item). Both share the same `EventField`
structure.
:::

## Standalone event declaration

```text
<event_declaration> ::=
    "event" <identifier> "(" [<event_field_signature> ("," <event_field_signature>)* [","]] ")" ";" ;
```

Dependencies:

* `<identifier>`
* `<event_field_signature>`

The standalone form produces `Stmt::EventDecl(EventDecl)`. The `EventDecl`
struct has:

- `name: Ident`
- `is_anon: bool`
- `fields: Vec<EventField>`

:::note
The parser always sets `is_anon: false` for standalone event declarations.
Anonymous events may be supported in a future revision.
:::

## Emit

```text
<emit_statement> ::= "emit" <identifier> "(" [<expression> ("," <expression>)* [","]] ")" ";" ;
```

Dependencies:

* `<identifier>`
* `<expression>`

The `emit` statement produces `Stmt::Emit(name, args, span)`. Arguments
correspond to the event's fields in declaration order.

## Semantics

The EVM allows up to four topics per log entry. If `anon` is used, the event
may contain four `indexed` values. Otherwise, the first topic is reserved for
the event selector — the keccak256 hash of the event name followed by a
parenthesized, comma-separated list of the field type names (matching
Solidity's ABI specification). In that case, at most three `indexed` fields
are allowed.
