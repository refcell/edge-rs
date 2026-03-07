import{u as t,j as e}from"./index-B0DkJJZv.js";const r={title:"ABI",description:"undefined"};function a(i){const n={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...t(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"abi",children:["ABI",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#abi",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:`The application binary interface is both a construct to
generate a JSON ABI by the compiler as well as a subtyping
construct for contract objects.`}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<abi_declaration> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "abi" <ident> [":" <ident> ("&" <ident>)*] "{"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"        ("})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'            ["mut"] <function_declaration> ";"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"        )*"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "}" ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<function_declaration>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<abi_declaration>"}),` is prefixed with "abi",
followed by its identifier, then an optional
colon and list of ampersand separated identifiers,
and finally a series of zero or more function
declarations optionally prefixed by "mut" and
delimited by curly braces.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The optional "mut" keyword indicates whether the
function will mutate the state of the smart contract
or the EVM. This allows contracts to determine
whether to use the call or staticcall instruction
to interface with a target conforming to the given
ABI.`}),`
`,e.jsx(n.p,{children:`The optional ampersand separated list of identifiers
represents other ABI identifiers to enable ABI
subtyping.`}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:`Todo: revisit ABI subtyping here and decide whether traits cover the same use
case.`})})]})}function d(i={}){const{wrapper:n}={...t(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(a,{...i})}):a(i)}export{d as default,r as frontmatter};
