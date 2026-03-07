import{u as a,j as n}from"./index-B0DkJJZv.js";const l={title:"Constants",description:"undefined"};function i(s){const e={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...s.components};return n.jsxs(n.Fragment,{children:[n.jsx(e.header,{children:n.jsxs(e.h1,{id:"constants",children:["Constants",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#constants",children:n.jsx(e.div,{"data-autolink-icon":!0})})]})}),`
`,n.jsxs(e.h2,{id:"declaration",children:["Declaration",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsx(e.code,{children:n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'<constant_declaration> ::= "const" <ident> [<type_signature>] ;'})})})})}),`
`,n.jsx(e.p,{children:"Dependencies:"}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ident>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<type_signature>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<constant_declaration>"}),` is a "const" followed by an
identifier and optional type signature.`]}),`
`,n.jsxs(e.h2,{id:"assignment",children:["Assignment",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#assignment",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsxs(e.code,{children:[n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<constant_assignment> ::="})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    <constant_declaration> "=" <expr> ;'})})]})})}),`
`,n.jsx(e.p,{children:"Dependencies:"}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<expr>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<constant_assignment>"}),` is a constant declaration followed
by an assignment operator and either an expression or a comma
separated list of identifiers delimited by parentheses followed
by a code block.`]}),`
`,n.jsx(e.aside,{"data-callout":"note",children:n.jsx(e.p,{children:"Note: The expression must be a comptime expression, but the grammar should not constrain this."})}),`
`,n.jsxs(e.h2,{id:"semantics",children:["Semantics",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsx(e.p,{children:`Constants must be resolvable at compile time either by assigning
it a literal, another constant, or an expression that can be
resolved at compile time.`}),`
`,n.jsx(e.p,{children:`The type of a constant will only be inferred if its assignment
is a literal with a type annotation, another constant with a
resolved type, or an expression with a resolved type such as
a function call.`}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsxs(e.code,{children:[n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const A: u8 = 1;"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const B = 1u8;"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const C = B;"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const D = a();"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const E: u8 = b();"})}),`
`,n.jsx(e.span,{className:"line","data-empty-line":!0,children:" "}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"comptime fn a() -> u8 {"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    1"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})}),`
`,n.jsx(e.span,{className:"line","data-empty-line":!0,children:" "}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn b() -> T {"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    2"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})})]})}function r(s={}){const{wrapper:e}={...a(),...s.components};return e?n.jsx(e,{...s,children:n.jsx(i,{...s})}):i(s)}export{r as default,l as frontmatter};
