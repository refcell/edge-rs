import{u as a,j as n}from"./index-B0DkJJZv.js";const r={title:"Trait Constraints",description:"undefined"};function s(i){const e={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...i.components};return n.jsxs(n.Fragment,{children:[n.jsx(e.header,{children:n.jsxs(e.h1,{id:"trait-constraints",children:["Trait Constraints",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#trait-constraints",children:n.jsx(e.div,{"data-autolink-icon":!0})})]})}),`
`,n.jsx(e.p,{children:`Traits are interface-like declarations that constrain generic types to
implement specific methods or contain specific properties.`}),`
`,n.jsxs(e.h2,{id:"declaration",children:["Declaration",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsxs(e.code,{children:[n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<trait_declaration> ::="})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    ["pub"] "trait" <ident> [<type_parameters>] [<trait_constraints>] "{"'})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    ("})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <type_declaration>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <type_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <constant_declaration>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <constant_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <function_declaration>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        | <function_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    )*"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    "}" ;'})})]})})}),`
`,n.jsx(e.p,{children:"Dependencies:"}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ident>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_parameters>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_declaration>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_assignment>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<constant_declaration>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<constant_assignment>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<function_declaration>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<function_assignment>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<trait_declaration>"}),` is a declaration of a set of associated
types, constants, and functions that may itself take type parameters
and may be constrained to a super type. Semantics of the declaration
are listed under trait solving rules.`]}),`
`,n.jsxs(e.h2,{id:"constraints",children:["Constraints",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#constraints",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsx(e.code,{children:n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'<trait_constraints> ::= ":" <ident> ("&" <ident>)* ;'})})})})}),`
`,n.jsx(e.p,{children:"Dependencies:"}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ident>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<trait_constraints>"}),` contains a colon followed by an
ampersand separated list of identifiers of implemented traits.
The ampersand is meant to indicate that all of the trait
identifiers are implemented for the type.`]}),`
`,n.jsxs(e.h2,{id:"semantics",children:["Semantics",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(e.p,{children:`Traits can be defined with associated types, constants,
and functions. The trait declaration itself allows for
optional assignment for each item as a default. Any
declarations in the trait that are not assigned in the
trait declaration must be assigned in the implementation
of the trait for the data type. Additionally, any assignments
in the trait declaration can be overridden in the trait
implementation.`}),`
`,n.jsx(e.p,{children:`While types can depend on trait constraints, traits can
also depend on other trait constraints. These assert that
types that implement a given trait also implement its
"super traits".`}),`
`,n.jsxs(e.h2,{id:"solving",children:["Solving",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#solving",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(e.aside,{"data-callout":"note",children:n.jsx(e.p,{children:"Todo: trait-solving semantics are still being drafted."})})]})}function d(i={}){const{wrapper:e}={...a(),...i.components};return e?n.jsx(e,{...i,children:n.jsx(s,{...i})}):s(i)}export{d as default,r as frontmatter};
