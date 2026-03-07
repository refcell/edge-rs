import{u as s,j as e}from"./index-B0DkJJZv.js";const l={title:"Literals",description:"undefined"};function a(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",p:"p",pre:"pre",span:"span",...s(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"literals",children:["Literals",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#literals",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(n.h2,{id:"characters",children:["Characters",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#characters",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<bin_char> ::= "0" | "1" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<dec_char> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<hex_char> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "a"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    | "b" | "c" | "d" | "e" | "f" | "A" | "B" | "C" | "D" | "E" | "F";'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<alpha_char> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<alphanumeric_char> ::= <alpha_char> | <dec_char> ;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:`<unicode_char> ::= ? "i ain't writing all that. happy for you tho. or sorry that happened" ? ;`})})]})})}),`
`,e.jsxs(n.h2,{id:"numeric",children:["Numeric",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#numeric",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<bin_literal> ::= { "0b" (<bin_char> | "_")+ [<numeric_type>]} ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<dec_literal> ::= { (<dec_char> | "_")+ [<numeric_type>]} ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<hex_literal> ::= { "0x" (<hex_char> | "_")+ [<numeric_type>]} ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<numeric_literal> ::= <bin_literal> | <dec_literal> | <hex_literal> ;"})})]})})}),`
`,e.jsx(n.p,{children:`Numeric literals are composed of binary, decimal, and hexadecimal digits.
Each digit may contain an arbitrary number of underscore characters in
them and may be suffixed with a numeric type.`}),`
`,e.jsxs(n.p,{children:[`Binary literals are prefixed with 0b and hexadecimal literals are prefixed
with `,e.jsx(n.code,{children:"0x"}),"."]}),`
`,e.jsxs(n.h2,{id:"string",children:["String",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#string",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:`<string_literal> ::= { '"' (!'"' <unicode_char>)* '"' } | { "'" (!"'" <unicode_char>)* "'" };`})})})})}),`
`,e.jsx(n.p,{children:`String literals contain alphanumeric characters delimited by double or
single quotes.`}),`
`,e.jsxs(n.h2,{id:"boolean",children:["Boolean",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#boolean",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<boolean_literal> ::= "true" | "false" ;'})})})})}),`
`,e.jsx(n.p,{children:'Boolean literals may be either "true" or "false".'}),`
`,e.jsxs(n.h2,{id:"literal",children:["Literal",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#literal",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<literal> ::= <numeric_literal> | <string_literal> | <boolean_literal> ;"})})})})}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Numeric literals may contain arbitrary underscores in the same literal.
Numeric literals may also be suffixed with the numeric type to constrain
its type. If there is no type suffix, the type is inferred by the context.
If a type cannot be inferred, it will default to a u256.`}),`
`,e.jsx(n.p,{children:`Both numeric and boolean literals are roughly translated to pushing the
value onto the stack.`}),`
`,e.jsx(n.p,{children:`String literals represent string instantiation. String instantiation
behaves as a packed u8 array instantiation.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const A = 1;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const B = 1u8;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const C = 0b11001100;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const D = 0xffFFff;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const E = true;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'const F = "asdf";'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'const G = "💩";'})})]})})})]})}function c(i={}){const{wrapper:n}={...s(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(a,{...i})}):a(i)}export{c as default,l as frontmatter};
