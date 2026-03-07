import{u as t,j as e}from"./index-B0DkJJZv.js";const r={title:"Generics",description:"undefined"};function s(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...t(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"generics",children:["Generics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#generics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"Generics are polymorphic types enabling function and type reuse across different types."}),`
`,e.jsxs(n.h2,{id:"type-parameters",children:["Type Parameters",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#type-parameters",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<type_parameter_single> ::= <ident> <trait_constraints> ;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<type_parameters> ::= "<" <type_parameter_single> ("," <type_parameter_single>)* [","] ">" ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<trait_constraints>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<type_parameter_single>"}),` is an individual type parameter for parametric
polymorphic types and functions. We define this as a type name optionally
followed by a trait constraint.`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<type_parameters>"}),` is a comma separated list of individual type
parameters delimited by angle brackets.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Generics are resolved at compile time through monomorphization.
Generic functions and data types are monomorphized into distinct
unique functions and data types. Function duplication can become
problematic due to the EVM bytecode size limit, so a series of
steps will be taken to allow for granular control over bytecode
size. Those semantics are defined in the Codesize document.`})]})}function d(i={}){const{wrapper:n}={...t(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(s,{...i})}):s(i)}export{d as default,r as frontmatter};
