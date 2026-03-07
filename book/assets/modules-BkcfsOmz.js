import{u as s,j as e}from"./index-B0DkJJZv.js";const t={title:"Modules",description:"undefined"};function d(i){const n={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"modules",children:["Modules",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#modules",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(n.h2,{id:"declaration",children:["Declaration",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#declaration",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<module_declaration> ::= ["pub"] "mod" <ident> "{" [<module_devdoc>] (<stmt>)* "}" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<module_devdoc>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<stmt>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<module_declaration>"}),` is composed of an optional "pub" prefix,
the "mod" keyword followed by an identifier then the body of the module
containing an optional devdoc, followed by a list of declarations and
module items, delimited by curly braces.`]}),`
`,e.jsxs(n.h2,{id:"import",children:["Import",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#import",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<module_import_item> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    <ident> ("})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'        "::" ('})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'          | ("{" <module_import_item> ("," <module_import_item>)* [","] "}")'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"          | <module_import_item>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"        )"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    )* ;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<module_import> ::= ["pub"] "use" <ident> ["::" module_import_item] ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<module_import_item>"}),` is a recursive token, containing either
another module import item or a comma separated list of module
import items delimited by curly braces.`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<module_import>"}),` is an optional "pub" annotation followed
by "use", the module name, then module import items.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Namespace semantics in modules are defined in the namespace document."}),`
`,e.jsx(n.p,{children:"Visibility semantics in modules are defined in the visibility document."}),`
`,e.jsx(n.p,{children:`Modules can contain developer documentation, declarations, and assignments.
If the module contains developer documentation, it must be the first item
in the module. This is for readability.`}),`
`,e.jsx(n.p,{children:"Files are implicitly modules with a name equivalent to the file name."}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:`Todo: decide whether module filenames should be sanitized or whether filenames
must already contain only valid identifier characters.`})}),`
`,e.jsx(n.p,{children:`Type, function, abi, and contract declarations must be assigned in the same
module. However, traits are declared without assignment and submodules may
be declared without a block only if there is a file with a matching name.`}),`
`,e.jsx(n.p,{children:`The super identifier represents the direct parent module of the module
in which it's invoked.`})]})}function l(i={}){const{wrapper:n}={...s(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(d,{...i})}):d(i)}export{l as default,t as frontmatter};
