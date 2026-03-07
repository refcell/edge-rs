import{u as a,j as e}from"./index-B0DkJJZv.js";const r={title:"Primitive Types",description:"undefined"};function i(n){const s={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...n.components};return e.jsxs(e.Fragment,{children:[e.jsx(s.header,{children:e.jsxs(s.h1,{id:"primitive-types",children:["Primitive Types",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#primitive-types",children:e.jsx(s.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<integer_size> ::= "8" | "16" | "24" | "32" | "40" | "48" | "56" | "64" | "72" | "80" | "88" | "96"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "104" | "112" | "120" | "128" | "136" | "144" | "152" | "160" | "168" | "176" | "184" | "192"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "200" | "208" | "216" | "224" | "232" | "240" | "248" | "256" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<fixed_bytes_size> ::= "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "10" | "11" | "12"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "13" | "14" | "15" | "16" | "17" | "18" | "19" | "20" | "21" | "22" | "23" | "24" | "25"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "26" | "27" | "28" | "29" | "30" | "31" | "32" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<signed_integer> ::= {"i" <integer_size>} ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<unsigned_integer> ::= {"u" <integer_size>} ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<fixed_bytes> ::= {"b" <fixed_bytes_size>} ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<address> ::= "addr" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<boolean> ::= "bool" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<bit> ::= "bit" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<pointer> ::= <data_location> "ptr" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<numeric_type> ::= <signed_integer> | <unsigned_integer> | <fixed_bytes> | <address> ;"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<primitive_data_type> ::="})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"    | <numeric_type>"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"    | <boolean>"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"    | <pointer> ;"})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<data_location>"})}),`
`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<primitive_data_type>"}),` contains signed and unsigned integers, boolean,
address, and fixed bytes types. Additionally, we introduce a pointer type
that must be prefixed with a data location annotation.`]}),`
`,e.jsxs(s.h2,{id:"examples",children:["Examples",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#examples",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"u8"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"u256"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"i8"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"i256"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"b4"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"b32"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"addr"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"bool"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"bit"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"&s ptr"})})]})})}),`
`,e.jsxs(s.h2,{id:"semantics",children:["Semantics",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(s.p,{children:[`Integers occupy the number of bits indicated by their size.
Fixed bytes types occupy the number of bytes indicated by their size,
or `,e.jsx(s.code,{children:"size * 8"}),` bits. Address occupies 160 bits. Booleans occupy eight bits.
Bit occupies a single bit. Pointers occupy a number of bits equal to their
data location annotation.`]}),`
`,e.jsx(s.p,{children:"Pointers can point to both primitive and complex data types."})]})}function d(n={}){const{wrapper:s}={...a(),...n.components};return s?e.jsx(s,{...n,children:e.jsx(i,{...n})}):i(n)}export{d as default,r as frontmatter};
