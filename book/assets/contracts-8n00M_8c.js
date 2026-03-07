import{u as a,j as e}from"./index-B0DkJJZv.js";const s={title:"Contract Objects",description:"undefined"};function t(i){const n={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"contract-objects",children:["Contract Objects",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#contract-objects",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"Contract objects serve as an object-like interface to contract constructs."}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<contract_field_declaration> ::= <ident> ":" <type_signature> ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<contract_declaration> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "contract" <ident> "{"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'        [<contract_field_declaration> ("," <contract_field_declaration>)* [","]]'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "}" ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<contract_field_declaration>"}),` is an identifier and type signature,
separated by a colon.`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<contract_declaration>"}),` is the contract keyword, followed by its
identifier, followed by a curly brace delimited, comma separated list
of field declarations.`]}),`
`,e.jsxs(n.h2,{id:"implementation",children:["Implementation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#implementation",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<contract_impl_block> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "impl" <ident> [":" <ident>] "{"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'        (["ext"] ["mut"] <function_declaration>)*'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "}"'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<function_declaration>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<contract_impl_block>"}),` is composed of the "impl" keyword, followed
by its identifier, optionally followed by a colon and abi identifier,
followed by list of function declarations, optionally "ext" and/or "mut",
delimited by curly braces.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The contract object desugars to a single main function and storage
layout with a dispatcher.`}),`
`,e.jsx(n.p,{children:`Contract field declarations create the storage layout which start at
zero and increment by one for each field. Fields are never packed,
however, storage packing may be achieved by declaring contract fields
as packed structs or tuples.`}),`
`,e.jsx(n.p,{children:`Contract implementation blocks contain definitions of external
functions in the contract object. If the impl block contains a colon
and identifier, this indicates the impl block is satisfying an abi's
constrained functions. The "ext" keyword indicates the function is
publicly exposed via the contract's dispatcher. The "mut" keyword
indicates the function may mutate the global state in the EVM-sense;
that is to say "mut" functions require a "call" instruction while
those without may use "call" or "staticcall" to interface with the
contract.`}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:`Todo: revisit this section and decide whether plain types cover the same use
case as contract objects.`})})]})}function d(i={}){const{wrapper:n}={...a(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(t,{...i})}):t(i)}export{d as default,s as frontmatter};
