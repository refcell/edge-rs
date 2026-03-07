---
title: Identifiers
---

# Identifiers

```text
<ident> ::= (<alpha_char> | "_") (<alpha_char> | <dec_digit> | "_")* ;
```

Dependencies:

* `<alpha_char>`
* `<dec_char>`

The `<ident>` is a C-style identifier, beginning with an alphabetic character
or underscore, followed by zero or more alphanumeric or underscore characters.
