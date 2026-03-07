import{u as c,j as e}from"./index-B0DkJJZv.js";const d={title:"Comments",description:"undefined"};function i(s){const n={a:"a",blockquote:"blockquote",code:"code",div:"div",h1:"h1",header:"header",p:"p",pre:"pre",span:"span",...c(),...s.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"comments",children:["Comments",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#comments",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<line_comment> ::= "//" (!"\\n" <ascii_char>)* "\\n" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<block_comment> ::= "/*" (!"*/" <ascii_char>)* "*/" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<item_devdoc> ::= "///" (!"\\n" <ascii_char>)* "\\n" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<module_devdoc> ::= "//!" (!"\\n" <ascii_char>)* "\\n" ;'})})]})})}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<line_comment>"})," is a single line comment, ignored by the parser."]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<block_comment>"})," is a multi line comment, ignored by the parser."]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<item_devdoc>"}),` is a developer documentation comment, treated as
documentation for the immediately following item.`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<module_devdoc>"}),` is a developer documentation comment, treated as
documentation for the module in which it is defined.`]}),`
`,e.jsxs(n.blockquote,{children:[`
`,e.jsx(n.p,{children:"Developer documentation comments are treated as Github-flavored markdown."}),`
`]})]})}function t(s={}){const{wrapper:n}={...c(),...s.components};return n?e.jsx(n,{...s,children:e.jsx(i,{...s})}):i(s)}export{t as default,d as frontmatter};
