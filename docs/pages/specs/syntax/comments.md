---
title: Comments
---

# Comments

```text
<line_comment> ::= "//" (!"\n" <ascii_char>)* "\n" ;

<block_comment> ::= "/*" (!"*/" <ascii_char> | <block_comment>)* "*/" ;

<item_devdoc> ::= "///" (!"\n" <ascii_char>)* "\n" ;

<module_devdoc> ::= "//!" (!"\n" <ascii_char>)* "\n" ;
```

The `<line_comment>` is a single-line comment, ignored by the parser.

The `<block_comment>` is a multi-line comment, ignored by the parser. Block
comments may be nested; the lexer tracks depth to find the matching close
(`/* /* inner */ outer still open */` is valid).

The `<item_devdoc>` is a developer documentation comment, treated as
documentation for the immediately following item.

The `<module_devdoc>` is a developer documentation comment, treated as
documentation for the module in which it is defined.

Developer documentation comments are treated as GitHub-flavored markdown.

:::note
Unlike regular comments, `DocComment` tokens (`///` and `//!`) are **retained**
by the parser and associated with the item or module they document. Tooling that
consumes the parse tree (e.g. doc generators) will find doc comments there;
plain `//` and `/* */` comments are dropped before the parser ever runs.
:::
