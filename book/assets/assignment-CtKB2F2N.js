import{u as a,j as e}from"./index-B0DkJJZv.js";const t={title:"Type Assignment",description:"undefined"};function i(s){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...s.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"type-assignment",children:["Type Assignment",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#type-assignment",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(n.h2,{id:"signature",children:["Signature",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<type_signature> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <array_signature>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <struct_signature>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <tuple_signature>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <union_signature>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <function_signature>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <ident>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | (<ident> [<type_parameters>]) ;"})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<array_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<struct_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<tuple_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<union_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<function_signature>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_parameters>"})}),`
`]}),`
`,e.jsx(n.p,{children:`Type assignments assign identifiers to type signatures. It may have a struct, tuple, union,
or function signature as well as an identifier followed by optional type parameters.`}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<type_declaration> ::= ["pub"] "type" <ident> [<type_parameters>]'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_parameters>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<type_declaration>"}),' is prefixed with "type" and contains an identifier with optional type parameters.']}),`
`,e.jsxs(n.h2,{id:"assignment",children:["Assignment",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#assignment",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<type_assignment> ::= <type_declaration> "=" <type_signature> ;'})})})})}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<type_assignment>"})," is a type declaration followed by a type signature separated by an assignment operator."]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Type assignment entails creating an identifier associated with a certain data structure or existing type.
If the assignment is to an existing data type, it contains the same fields or members, if any, and exposes the
same associated items, if any.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MyCustomType = packed (u8, u8, u8);"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MyCustomAlias = MyCustomType;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn increment(rgb: MyCustomType) -> MyCustomType {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    return (rgb.0 + 1, rgb.1 + 1, rgb.2 + 1);"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"increment(MyCustomType(1, 2, 3));"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"increment(MyCustomAlias(1, 2, 3));"})})]})})}),`
`,e.jsx(n.p,{children:`A way to create a wrapper around an existing type without exposing the existing type's external interface,
the type may be wrapped in parenthesis, creating a "tuple" of one element, which comes without overhead.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MyCustomType = packed (u8, u8, u8);"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MyNewCustomType = (MyCustomType);"})})]})})})]})}function d(s={}){const{wrapper:n}={...a(),...s.components};return n?e.jsx(n,{...s,children:e.jsx(i,{...s})}):i(s)}export{d as default,t as frontmatter};
