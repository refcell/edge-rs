import{u as s,j as e}from"./index-B0DkJJZv.js";const t={title:"Array Types",description:"undefined"};function a(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",h3:"h3",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"array-types",children:["Array Types",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#array-types",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"The array type is a list of elements of a single type."}),`
`,e.jsxs(n.h2,{id:"signature",children:["Signature",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<array_signature> ::= ["packed"] "[" <type_signature> ";" <expr> "]" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<array_signature>"}),` consists of an optional "packed" keyword prefix
to a type signature and expression separated by a colon, delimited by brackets.`]}),`
`,e.jsxs(n.h2,{id:"instantiation",children:["Instantiation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#instantiation",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<array_instantiation> ::= [<data_location>] "[" <expr> ("," <expr>)* [","] "]" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<data_location>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<array_instantiation>"}),` is an optional data location annotation followed
by a comma separated list of expressions delimited by brackets.`]}),`
`,e.jsxs(n.h2,{id:"element-access",children:["Element Access",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#element-access",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<array_element_access> ::= <ident> "[" <expr> [":" <expr>] "]" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<array_element_access>"}),` is the array's identifier followed a
bracket-delimited expression and optionally a second expression, colon
separated.`]}),`
`,e.jsxs(n.h2,{id:"examples",children:["Examples",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#examples",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type TwoElementIntegerArray = [u8; 2];"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type TwoElementPackedIntegerArray = packed [u8; 2];"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const arr: TwoElementIntegerArray = [1, 2];"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const elem: u8 = arr[0];"})})]})})}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.h3,{id:"instantiation-1",children:["Instantiation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#instantiation-1",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Instantiation of a fixed-length array stores one element per 32 byte word in
either data location. The only difference between data locations in terms of
instantiation behavior is if all elements of the array are populated with
constant values and the array belongs in memory, a performance optimization
may include code-copying an instance of the constant array from the bytecode
into memory.`}),`
`,e.jsxs(n.h3,{id:"access",children:["Access",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#access",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Array element access depends on whether the second expression is included.
If a single expression is inside the access brackets, the single element is
returned from the array. If a second expression follows the first with a colon
in between, a pointer of the same data location is returned. The type of the
new array pointer is the same type but the size is now the size of the second
expression's value minus the first expression's value. If the index values are
known at compile time and are greater than or equal to the array's length, a
compiler error is thrown, else a bounds check against the array's length is
added into the runtime bytecode.`})]})}function d(i={}){const{wrapper:n}={...s(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(a,{...i})}):a(i)}export{d as default,t as frontmatter};
