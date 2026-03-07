import{u as r,j as e}from"./index-B0DkJJZv.js";const a={title:"Specifications",description:"undefined"};function t(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",h3:"h3",h4:"h4",header:"header",li:"li",p:"p",ul:"ul",...r(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"specifications",children:["Specifications",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#specifications",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(n.h2,{id:"all-edge-no-drag",children:["All Edge, No Drag.",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#all-edge-no-drag",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"This document defines Edge, a domain specific language for the Ethereum Virtual Machine (EVM)."}),`
`,e.jsx(n.p,{children:"Edge is a high level, strongly statically typed, multi-paradigm language. It provides:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:"A thin layer of abstraction over the EVM's instruction set architecture (ISA)."}),`
`,e.jsx(n.li,{children:"An extensible polymorphic type system with subtyping."}),`
`,e.jsx(n.li,{children:"First class support for modules and code reuse."}),`
`,e.jsx(n.li,{children:"Compile time code execution to fine-tune the compiler's input."}),`
`]}),`
`,e.jsx(n.p,{children:`Edge's syntax is similar to Rust and Zig where intuitive, however, the language is not designed
to be a general purpose language with EVM features as an afterthought. Rather, it is designed
to extend the EVM instruction set with a reasonable type system and syntax sugar over universally
understood programming constructs.`}),`
`,e.jsxs(n.h3,{id:"notation",children:["Notation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#notation",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"This specification uses a grammar similar to Extended Backus-Naur Form (EBNF) with the following rules."}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:["Non-terminal tokens are wrapped in angle brackets ",e.jsx(n.code,{children:"<ident>"}),"."]}),`
`,e.jsxs(n.li,{children:["Terminal tokens are wrapped in double quotes ",e.jsx(n.code,{children:'"const"'}),"."]}),`
`,e.jsxs(n.li,{children:["Optional items are wrapped in brackets ",e.jsx(n.code,{children:'["mut"]'}),"."]}),`
`,e.jsxs(n.li,{children:["Sequences of zero or more items are wrapped in parenthesis and suffixed with a star ",e.jsx(n.code,{children:'("," <ident>)*'}),"."]}),`
`,e.jsxs(n.li,{children:["Sequences of one or more items are wrapped in parenthesis and suffixed with a plus ",e.jsx(n.code,{children:"(<ident>)+"}),"."]}),`
`]}),`
`,e.jsxs(n.p,{children:[`In contrast to EBNF, we define a rule that all items are non-atomic, that is to say arbitrary whitespace
characters `,e.jsx(n.code,{children:"\\n"}),", ",e.jsx(n.code,{children:"\\t"}),", and ",e.jsx(n.code,{children:"\\r"}),` may surround all tokens unless wrapped with curly braces
`,e.jsx(n.code,{children:'{ "0x" (<hex_digit>)* }'}),"."]}),`
`,e.jsx(n.p,{children:`Generally, we use long-formed names for clarity of each token, however, common tokens are abbreviated
and defined as follows:`}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:'"ident": "identifier"'}),`
`,e.jsx(n.li,{children:'"expr": "expression"'}),`
`,e.jsx(n.li,{children:'"stmt": "statement"'}),`
`]}),`
`,e.jsxs(n.h3,{id:"disambiguation",children:["Disambiguation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#disambiguation",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"This section contains context that may be required throughout the specification."}),`
`,e.jsxs(n.h4,{id:"return-vs-return",children:["Return vs Return",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#return-vs-return",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The word "return" refers to two different behaviors, returned values from expressions and
the halting return opcode.`}),`
`,e.jsx(n.p,{children:`When "return" is used, this refers to the values returned from expressions, that is to say
the values left on the stack, if any.`}),`
`,e.jsxs(n.p,{children:['When "halting return" is used, this refers to the EVM opcode ',e.jsx(n.code,{children:"return"}),` that halts execution and
returns a value from a slice of memory to the caller of the current execution context.`]})]})}function d(i={}){const{wrapper:n}={...r(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(t,{...i})}):t(i)}export{d as default,a as frontmatter};
