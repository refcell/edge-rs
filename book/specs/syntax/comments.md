# Comments

```
<line_comment> ::= "//" (!"\n" <ascii_char>)* "\n" ;

<block_comment> ::= "/*" (!"*/" <ascii_char>)* "*/" ;

<item_devdoc> ::= "///" (!"\n" <ascii_char>)* "\n" ;

<module_devdoc> ::= "//!" (!"\n" <ascii_char>)* "\n" ;
```

The `<line_comment>` is a single line comment, ignored by the parser.

The `<block_comment>` is a multi line comment, ignored by the parser.

The `<item_devdoc>` is a developer documentation comment, treated as
documentation for the immediately following item.

The `<module_devdoc>` is a developer documentation comment, treated as
documentation for the module in which it is defined.

> Developer documentation comments are treated as Github-flavored markdown.
