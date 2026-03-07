import{u as a,j as n}from"./index-B0DkJJZv.js";const c={title:"Expressions",description:"undefined"};function i(s){const e={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",strong:"strong",ul:"ul",...a(),...s.components};return n.jsxs(n.Fragment,{children:[n.jsx(e.header,{children:n.jsxs(e.h1,{id:"expressions",children:["Expressions",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#expressions",children:n.jsx(e.div,{"data-autolink-icon":!0})})]})}),`
`,n.jsx(n.Fragment,{children:n.jsx(e.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:n.jsxs(e.code,{children:[n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<binary_operation> ::= <expr> <binary_operator> <expr> ;"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<unary_operation> ::= <unary_operator> <expr> ;"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"<expr> ::="})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <array_instantiation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <array_element_access>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <struct_instantiation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <tuple_instantiation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <struct_field_access>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <tuple_field_access>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <union_instantiation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <pattern_match>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <arrow_function>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <function_call>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <binary_operation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <unary_operation>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <ternary>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <literal>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:"    | <ident>"})}),`
`,n.jsx(e.span,{className:"line",children:n.jsx(e.span,{children:'    | ("(" <expr> ")");'})})]})})}),`
`,n.jsxs(e.h2,{id:"dependencies",children:["Dependencies:",n.jsx(e.a,{"aria-hidden":"true",tabIndex:"-1",href:"#dependencies",children:n.jsx(e.div,{"data-autolink-icon":!0})})]}),`
`,n.jsxs(e.ul,{children:[`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<binary_operator>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<unary_operator>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<array_instantiation>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<array_element_access>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<struct_instantiation>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<tuple_instantiation>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<struct_field_access>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<tuple_field_access>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<union_instantiation>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<pattern_match>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<arrow_function>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<function_call>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ternary>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<literal>"})}),`
`,n.jsx(e.li,{children:n.jsx(e.code,{children:"<ident>"})}),`
`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<expr>"})," is defined as an item that returns","1"," a value."]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<binary_operation>"}),` is an expression composed of two sub-expressions
with an infixed binary operator. Semantics are beyond the scope of the
syntax specification, see operator precedence semantics for more.`]}),`
`,n.jsxs(e.p,{children:["The ",n.jsx(e.code,{children:"<unary_operation>"}),` is an expression composed of a prefixed unary
operator and a sub-expression.`]}),`
`,n.jsxs(e.p,{children:[n.jsx(e.strong,{children:"1"}),": See Disambiguation: Return vs Return™️"]})]})}function l(s={}){const{wrapper:e}={...a(),...s.components};return e?n.jsx(e,{...s,children:n.jsx(i,{...s})}):i(s)}export{l as default,c as frontmatter};
