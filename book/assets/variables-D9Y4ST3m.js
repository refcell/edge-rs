import{u as s,j as e}from"./index-B0DkJJZv.js";const d={title:"Variables",description:"undefined"};function a(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"variables",children:["Variables",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#variables",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<variable_declaration> ::= "let" <ident> [":" <type_signature>] ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<variable_declaration>"}),` marks the declaration of a variable,
it may optionally be assigned at the time of declaration.`]}),`
`,e.jsxs(n.h2,{id:"assignment",children:["Assignment",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#assignment",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<variable_assignment> ::= <ident> "=" <expr> ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<variable_assignment>"}),` is the assignment of a variable.
Its identifier is assigned the returned value of an expression using
the assignment operator.`]})]})}function t(i={}){const{wrapper:n}={...s(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(a,{...i})}):a(i)}export{t as default,d as frontmatter};
