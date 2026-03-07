---
title: Sum Types
---

# Sum Types

The sum type is a union of multiple types where the data type represents
one of the inner types.

## Signature

```text
<union_member_signature> ::= <ident> ["(" <type_signature> ")"] ;
<union_signature> ::= ["|"] <union_member_signature> ("|" <union_member_signature>)* ;
```

Dependencies:

* `<ident>`
* `<type_signature>`

The `<union_declaration>` is a declaration of a sum type, or data structure
that contains one of its internally declared members. Each `<union_member>`
is named by an identifier, optionally followed by a number of comma separated
types delimited by parenthesis.

## Instantiation

```text
<union_instantiation> ::= <ident> "::" <ident> "(" [<expr> ("," <expr>)* [","]] ")" ;
```

Dependencies:

* `<ident>`
* `<expr>`

The `<union_instantiation>` instantiates, or creates, the sum type. This
consists of the union's identifier, followed by the member's identifier,
followed by an optional comma separated list of expressions.

Behavior of instantiation is defined in the data location rule.

## Union Pattern

```text
<union_pattern> ::= <ident> "::" <ident> ["(" <ident> ("," <ident>)* [","] ")"];
```

Dependencies:

* `<ident>`

The `<union_pattern>` is a pattern consisting of the union's name and
a member's name separated by a double colon.

## Pattern Match

```text
<pattern_match> ::= <ident> "matches" <union_pattern> ;
```

Dependencies:

* `<ident>`

## Semantics

All unions have a Unions where no member has its own internal type is
effectively an enumeration over integers.

```edge
type Mutex = Locked | Unlocked;

// Mutex::Locked == 0
// Mutex::Unlocked == 1
```

Unions where any members have an internal type become proper type unions.
The only case in which a union can exist on the stack rather than another
data location is if the largest of the internal types has a bitsize of 248
or less. If any member's internal type is greater than 248, a data location
must be specified.

```edge
type StackUnion = A(u8) | B(u248);

type MemoryUnion = A(u256) | B | C(u8);
```

A union pattern consists of its identifier and the member identifier
separated by colons. This pattern may be used both in match statements
and if statements.

```edge
type Option<T> = None | Some(T);

impl Option<T> {
    fn unwrap(self) -> T {
        match self {
            Option::Some(inner) => return inner,
            Option::None => revert(),
        };
    }

    fn unwrapOr(self, default: T) -> T {
        let mut value = default;
        if self matches Option::Some(inner) {
            value = inner;
        }
        return value;
    }
}
```
