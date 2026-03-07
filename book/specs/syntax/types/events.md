# Event Types

The event type is a custom type to be logged.

## Signature

```
<event_field_signature> ::= <ident> ":" ( "indexed" "<" <type_signature> ">" | <type_signature> ) ;

<event_signature> ::=
    ["anon"] "event" "{" [<event_field_signature> ("," <event_field_signature>)* [","]] "}" ;
```

Dependencies:

* `<ident>`
* `<type_signature>`

The `<event_field_signature>` is an optional "anon" word,
followed by "event", followed by either a type signature
or a type signature delimited by angle brackets and prefixed
with "indexed".

## Semantics

The event type is assigned an identifier the same way other
types are assigned an identifier. The EVM allows up to four
topics, therefore if "anon" is used, the event may contain
four "indexed" values, else the event may contain three. If
the event is not anonymous, the first topic follows Solidity's
ABI specification. That is to say the first topic is the
keccak256 hash digest of the event identifier, followed by
a comma separated list of the event type names with no
whitespace, delimited by parenthesis.
