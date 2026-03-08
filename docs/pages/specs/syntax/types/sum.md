---
title: Sum types
---

# Sum types

The sum type is a union of multiple types where the value represents
exactly one of the inner variants.

## Signature

```text
<union_member_signature> ::= <identifier> ["(" <type_signature> ")"] ;
<union_signature> ::= ["|"] <union_member_signature> ("|" <union_member_signature>)* ;
```

Dependencies:

* `<identifier>`
* `<type_signature>`

The `<union_signature>` declares a sum type — a data structure that holds
one of its declared members. Each `<union_member_signature>` is named by an
identifier, optionally followed by exactly one payload type in parentheses.
A leading `|` is permitted for formatting convenience.

Each member maps to `UnionMember { name, inner: Option<TypeSig> }` in the AST.
The overall signature maps to `TypeSig::Union(Vec<UnionMember>)`.

## Instantiation

```text
<union_instantiation> ::= <identifier> "::" <identifier> "(" [<expression> ("," <expression>)* [","]] ")" ;
```

Dependencies:

* `<identifier>`
* `<expression>`

The `<union_instantiation>` creates a union value. It consists of the union
type name, `::`, the variant name, and arguments in parentheses. This produces
`Expr::UnionInstantiation(type_name, variant_name, args, span)`.

:::note
Although each variant carries at most one type in its signature, the
instantiation syntax accepts multiple comma-separated expressions. For
variants with a tuple payload, these expressions correspond to the tuple
elements.
:::

## Union pattern

```text
<union_pattern> ::= <identifier> "::" <identifier> ["(" <identifier> ("," <identifier>)* [","] ")"] ;
```

Dependencies:

* `<identifier>`

The `<union_pattern>` matches a specific variant by type name and member name,
optionally binding payload values to identifiers. It maps to
`UnionPattern { union_name, member_name, bindings }` in the AST.

## Pattern match expression

```text
<pattern_match> ::= <expression> "matches" <union_pattern> ;
```

Dependencies:

* `<expression>`
* `<union_pattern>`

The `matches` keyword produces `Expr::PatternMatch(expr, pattern, span)` and
can be used anywhere an expression is valid.

## Semantics

A union where no member has an internal type is effectively an enumeration
over integers:

```edge
type Mutex = Locked | Unlocked;

// Mutex::Locked == 0
// Mutex::Unlocked == 1
```

Unions where any members have an internal type become proper type unions.
Each variant may carry **at most one** payload type:

```edge
type StackUnion = A(u8) | B(u248);

type MemoryUnion = A(u256) | B | C(u8);
```

:::note
Data-carrying variants are heap-allocated: the discriminant is stored at the
base memory address and the single payload is stored at `base + 32`. The union
value for a data-carrying variant is the base memory pointer, not an inline
integer. Unit variants (no payload) are represented as an inline integer
discriminant.
:::

A union pattern consists of the type name and the member name separated by
`::`. This pattern may be used in both `match` statements and `if` conditions:

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
