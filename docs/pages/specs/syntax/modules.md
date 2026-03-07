---
title: Modules
---

# Modules

## Declaration

```text
<module_declaration> ::= ["pub"] "mod" <ident> "{" [<module_devdoc>] (<stmt>)* "}" ;
```

Dependencies:

* `<ident>`
* `<module_devdoc>`
* `<stmt>`

The `<module_declaration>` is composed of an optional "pub" prefix,
the "mod" keyword followed by an identifier then the body of the module
containing an optional devdoc, followed by a list of declarations and
module items, delimited by curly braces.

## Import

```text
<module_import_item> ::=
    <ident> (
        "::" (
          | ("{" <module_import_item> ("," <module_import_item>)* [","] "}")
          | <module_import_item>
        )
    )* ;

<module_import> ::= ["pub"] "use" <ident> ["::" module_import_item] ;
```

Dependencies:

* `<ident>`

The `<module_import_item>` is a recursive token, containing either
another module import item or a comma separated list of module
import items delimited by curly braces.

The `<module_import>` is an optional "pub" annotation followed
by "use", the module name, then module import items.

## Semantics

Namespace semantics in modules are defined in the namespace document.

Visibility semantics in modules are defined in the visibility document.

Modules can contain developer documentation, declarations, and assignments.
If the module contains developer documentation, it must be the first item
in the module. This is for readability.

Files are implicitly modules with a name equivalent to the file name.

:::note
Todo: decide whether module filenames should be sanitized or whether filenames
must already contain only valid identifier characters.
:::

Type, function, abi, and contract declarations must be assigned in the same
module. However, traits are declared without assignment and submodules may
be declared without a block only if there is a file with a matching name.

The super identifier represents the direct parent module of the module
in which it's invoked.
