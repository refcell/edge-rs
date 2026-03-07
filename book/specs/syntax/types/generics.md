# Generics

Generics are polymorphic types enabling function and type reuse across different types.

## Type Parameters

```
<type_parameter_single> ::= <ident> <trait_constraints> ;

<type_parameters> ::= "<" <type_parameter_single> ("," <type_parameter_single>)* [","] ">" ;
```

Dependencies:

* `<ident>`
* `<trait_constraints>`

The `<type_parameter_single>` is an individual type parameter for parametric
polymorphic types and functions. We define this as a type name optionally
followed by a trait constraint.

The `<type_parameters>` is a comma separated list of individual type
parameters delimited by angle brackets.

## Semantics

Generics are resolved at compile time through monomorphization.
Generic functions and data types are monomorphized into distinct
unique functions and data types. Function duplication can become
problematic due to the EVM bytecode size limit, so a series of
steps will be taken to allow for granular control over bytecode
size. Those semantics are defined in the Codesize document.
