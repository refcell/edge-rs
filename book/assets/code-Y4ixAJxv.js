import{u as d,j as e}from"./index-B0DkJJZv.js";const c={title:"Code Block",description:"undefined"};function s(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...d(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"code-block",children:["Code Block",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#code-block",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:`A code block is a sequence of items with its own scope.
It may be used independently or in tandem with conditional
statements.`}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<code_block> ::= "{" ((<stmt> | <expr>) ";")* "}" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<stmt>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<code_block>"}),` is a semi-colon separated list of
expressions or statements delimited by curly braces.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Code blocks may be contained in loops, branching
statements, or standalone statements.`}),`
`,e.jsx(n.p,{children:`Code blocks represent a distinct scope. Identifiers
declared in a code block are dropped once the code
block ends.`})]})}function a(i={}){const{wrapper:n}={...d(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(s,{...i})}):s(i)}export{a as default,c as frontmatter};
