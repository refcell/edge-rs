---
title: Statements
---

# Statements

```text
<stmt> ::=
    | <variable_declaration>
    | <variable_assignment>
    | <type_declaration>
    | <type_assignment>
    | <trait_declaration>
    | <impl_block>
    | <function_declaration>
    | <function_assignment>
    | <abi_declaration>
    | <contract_declaration>
    | <contract_impl_block>
    | <core_loop>
    | <for_loop>
    | <while_loop>
    | <do_while_loop>
    | <code_block>
    | <if_else_if_branch>
    | <if_match_branch>
    | <match>
    | <constant_assignment>
    | <comptime_branch>
    | <comptime_function>
    | <module_declaration>
    | <module_import> ;
```

## Dependencies:

* `<variable_declaration>`
* `<variable_assignment>`
* `<type_declaration>`
* `<type_assignment>`
* `<trait_declaration>`
* `<impl_block>`
* `<function_declaration>`
* `<function_assignment>`
* `<abi_declaration>`
* `<contract_declaration>`
* `<contract_impl_block>`
* `<core_loop>`
* `<for_loop>`
* `<while_loop>`
* `<do_while_loop>`
* `<code_block>`
* `<if_else_if_branch>`
* `<if_match_branch>`
* `<match>`
* `<constant_assignment>`
* `<comptime_branch>`
* `<comptime_function>`
* `<module_declaration>`
* `<module_import>`

The `<stmt>` is similar to an expression, however the item
does not return<sup>1</sup> a value.

**1**: See Disambiguation: Return vs Return™️
