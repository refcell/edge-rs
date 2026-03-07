import{u as s,j as e}from"./index-B0DkJJZv.js";const d={title:"Event Types",description:"undefined"};function t(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"event-types",children:["Event Types",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#event-types",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"The event type is a custom type to be logged."}),`
`,e.jsxs(n.h2,{id:"signature",children:["Signature",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<event_field_signature> ::= <ident> ":" ( "indexed" "<" <type_signature> ">" | <type_signature> ) ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<event_signature> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    ["anon"] "event" "{" [<event_field_signature> ("," <event_field_signature>)* [","]] "}" ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<event_field_signature>"}),` is an optional "anon" word,
followed by "event", followed by either a type signature
or a type signature delimited by angle brackets and prefixed
with "indexed".`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The event type is assigned an identifier the same way other
types are assigned an identifier. The EVM allows up to four
topics, therefore if "anon" is used, the event may contain
four "indexed" values, else the event may contain three. If
the event is not anonymous, the first topic follows Solidity's
ABI specification. That is to say the first topic is the
keccak256 hash digest of the event identifier, followed by
a comma separated list of the event type names with no
whitespace, delimited by parenthesis.`})]})}function r(i={}){const{wrapper:n}={...s(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(t,{...i})}):t(i)}export{r as default,d as frontmatter};
