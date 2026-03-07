# Expressions

```
<binary_operation> ::= <expr> <binary_operator> <expr> ;
<unary_operation> ::= <unary_operator> <expr> ;

<expr> ::=
    | <array_instantiation>
    | <array_element_access>
    | <struct_instantiation>
    | <tuple_instantiation>
    | <struct_field_access>
    | <tuple_field_access>
    | <union_instantiation>
    | <pattern_match>
    | <arrow_function>
    | <function_call>
    | <binary_operation>
    | <unary_operation>
    | <ternary>
    | <literal>
    | <ident>
    | ("(" <expr> ")");
```

## Dependencies:

* `<binary_operator>`
* `<unary_operator>`
* `<array_instantiation>`
* `<array_element_access>`
* `<struct_instantiation>`
* `<tuple_instantiation>`
* `<struct_field_access>`
* `<tuple_field_access>`
* `<union_instantiation>`
* `<pattern_match>`
* `<arrow_function>`
* `<function_call>`
* `<ternary>`
* `<literal>`
* `<ident>`

The `<expr>` is defined as an item that returns<sup>1</sup> a value.

The `<binary_operation>` is an expression composed of two sub-expressions
with an infixed binary operator. Semantics are beyond the scope of the
syntax specification, see operator precedence semantics for more.

The `<unary_operation>` is an expression composed of a prefixed unary
operator and a sub-expression.

**1**: See Disambiguation: Return vs Return™️
