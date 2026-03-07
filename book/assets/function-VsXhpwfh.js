import{u as t,j as e}from"./index-B0DkJJZv.js";const d={title:"Function Types",description:"undefined"};function s(n){const i={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...t(),...n.components};return e.jsxs(e.Fragment,{children:[e.jsx(i.header,{children:e.jsxs(i.h1,{id:"function-types",children:["Function Types",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#function-types",children:e.jsx(i.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(i.p,{children:"The function type is a type composed of input and output types."}),`
`,e.jsxs(i.h2,{id:"signature",children:["Signature",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<function_signature> ::= <type_signature> "->" <type_signature> ;'})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<function_signature>"}),` consists of an input type signature
and an output type signature, separated by an arrow.`]}),`
`,e.jsxs(i.p,{children:["Note: ",e.jsx(i.code,{children:"<type_signature>"}),` also contains a tuple signature,
therefore a function with multiple inputs and outputs is
implicitly operating on a tuple.`]}),`
`,e.jsxs(i.h2,{id:"declaration",children:["Declaration",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(i.code,{children:[e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:"<function_declaration> ::="})}),`
`,e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'    "fn" <ident> "("'})}),`
`,e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'        [(<ident> ":" <type_signature>) ("," <ident> ":" <type_signature>)* [","]]'})}),`
`,e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'    ")" ["->" "(" <type_signature> ("," <type_signature>)* [","] ")"] ;'})})]})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<ident>"})}),`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(i.h2,{id:"assignment",children:["Assignment",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#assignment",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:"<function_assignment> ::= <function_declaration> <code_block> ;"})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<code_block>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<function_assignment>"}),` is defined as the "fn" keyword followed
by its identifier, followed by optional comma separated pairs of
identifiers and type signatures, delimited by parenthesis, then
optionally followed by an arrow and a list of comma separated return
types signatures delimited by parenthesis, then finally the code
block of the function body.`]}),`
`,e.jsxs(i.h2,{id:"arrow-functions",children:["Arrow Functions",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#arrow-functions",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsxs(i.span,{className:"line",children:[e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"<"}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"arrow_function"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"> ::= (<"}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"ident"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'> | ("(" <'}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"ident"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'> ("," <'}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"ident"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'>)* [","] ")")) "=>" <'}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"code_block"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"> ;"})]})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<ident>"})}),`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<code_block>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<arrow_function>"}),` is defined as either a single identifier
or a comma separated, parenthesis delimited list of identifiers,
followed by the "=>" bigram, followed by a code block.`]}),`
`,e.jsxs(i.h2,{id:"call",children:["Call",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#call",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsxs(i.span,{className:"line",children:[e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"<"}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"function_call"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"> ::= <"}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"ident"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'> "(" [<'}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"expr"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'> ("," <'}),e.jsx(i.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"expr"}),e.jsx(i.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:'>) [","]] ")" ;'})]})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<ident>"})}),`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<function_call>"}),` is an identifier followed by a comma
separated list of expressions delimited by parenthesis.`]}),`
`,e.jsxs(i.h2,{id:"semantics",children:["Semantics",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(i.aside,{"data-callout":"note",children:e.jsx(i.p,{children:"Todo: the function-type semantics section is still under construction."})})]})}function r(n={}){const{wrapper:i}={...t(),...n.components};return i?e.jsx(i,{...n,children:e.jsx(s,{...n})}):s(n)}export{r as default,d as frontmatter};
