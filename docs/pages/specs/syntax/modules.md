---
title: Modules
---

# Modules

## Declaration

```text
<module_declaration> ::= ["pub"] "mod" <identifier> (";" | "{" [<module_devdoc>] <statement>* "}") ;
```

Dependencies:

* `<identifier>`
* `<module_devdoc>`
* `<statement>`

The `<module_declaration>` is composed of an optional `pub` prefix,
the `mod` keyword followed by an identifier, then either a semicolon
(external/bodyless form) or a body delimited by curly braces. The
bodyless form (`mod name;`) declares an external module whose content
lives in a file with a matching name.

## Import

```text
<module_import_item> ::=
    "*"
  | <identifier> (
        "::" (
          | "{" <module_import_item> ("," <module_import_item>)* [","] "}"
          | <module_import_item>
        )
    )* ;

<module_import> ::= ["pub"] "use" <identifier> ["::" <module_import_item>] ";" ;
```

Dependencies:

* `<identifier>`

The `<module_import_item>` is a recursive production, containing either a
wildcard (`*`), another module import item, or a comma-separated list of
module import items delimited by curly braces.

The `<module_import>` is an optional `pub` annotation followed
by `use`, the root module name, then optional path segments.

:::warning
Neither `pub mod` nor `pub use` is currently implemented. The parser's
`parse_pub()` function only dispatches to `fn` and `contract` declarations,
so the `pub` modifier before `mod` or `use` is silently ignored. Use plain
`mod` and `use` for all module declarations and imports.
:::

## Semantics

Namespace semantics in modules are defined in the namespace document.

Visibility semantics in modules are defined in the visibility document.

Modules can contain developer documentation, declarations, and assignments.
If the module contains developer documentation, it must be the first item
in the module. This is for readability.

Files are implicitly modules with a name equivalent to the file name.

Type, function, ABI, and contract declarations must be assigned in the same
module. However, traits are declared without assignment and submodules may
be declared without a block only if there is a file with a matching name.

The `super` identifier represents the direct parent module of the module
in which it is invoked.
