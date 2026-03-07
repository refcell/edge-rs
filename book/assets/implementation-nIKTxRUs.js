import{u as s,j as n}from"./index-B0DkJJZv.js";const l={title:"Implementation",description:"undefined"};function t(i){const e={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...i.components};return n.jsxs(n.Fragment,{children:[n.jsx(e.header,{children:n.jsxs(e.h1,{id:"implementation",children:["Implementation",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#implementation",children:n.jsx(e.div,{"data-autolink-icon":!0})})]})}),`
`,n.jsx(e.p,{children:"Implementation blocks enable method-call syntax."}),`
`,n.jsxs(e.h2,{id:"implementation-block",children:["Implementation Block",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#implementation-block",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsxs(e.code,{children:[n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<impl_block> ::="})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    "impl" <ident> [<type_parameters>] [":" <ident> [<type_parameters>]] "{"'})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        ("})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"            | <function_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"            | <constant_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"            | <type_assignment>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"        )*"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    "}"'})})]})})}),`
`,n.jsx(e.p,{children:"Dependencies:"}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ident>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_parameters>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<function_assignment>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<constant_assignment>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_assignment>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<impl_block>"}),` is the implementation block for a give
type. The type identifier is optionally followed by type
parameters then optionally followed by a "for" clause.
The "for" clause contains trait identifiers and optional
type parameters for the traits. Followed by this is a list
of function, constant, and type assignments delimited by
curly braces.`]}),`
`,n.jsxs(e.h2,{id:"semantics",children:["Semantics",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(e.p,{children:`Associated functions, constants, and types are defined for a
given type. If the type contains any generics in any of its
internal assignments, the type parameters must be brought
into scope by annotating them directly following the type's
identifier.`}),`
`,n.jsx(e.p,{children:`If the impl block is to satisfy a trait's interface, the
type's identifier and optional type parameters are followed
by the trait's identifier and optional type parameters. In
this case, only associated functions, constants, and types
that are declared in the trait's declaration may be defined
in the impl block. Additionally, all declarations in a
trait's declaration that are not assigned in the trait's
declaration must be assigned in the impl block for the
given data type.`})]})}function r(i={}){const{wrapper:e}={...s(),...i.components};return e?n.jsx(e,{...i,children:n.jsx(t,{...i})}):t(i)}export{r as default,l as frontmatter};
