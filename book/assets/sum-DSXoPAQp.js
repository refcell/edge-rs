import{u as a,j as e}from"./index-B0DkJJZv.js";const l={title:"Sum Types",description:"undefined"};function s(i){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"sum-types",children:["Sum Types",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#sum-types",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:`The sum type is a union of multiple types where the data type represents
one of the inner types.`}),`
`,e.jsxs(n.h2,{id:"signature",children:["Signature",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<union_member_signature> ::= <ident> ["(" <type_signature> ")"] ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<union_signature> ::= ["|"] <union_member_signature> ("|" <union_member_signature>)* ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<union_declaration>"}),` is a declaration of a sum type, or data structure
that contains one of its internally declared members. Each `,e.jsx(n.code,{children:"<union_member>"}),`
is named by an identifier, optionally followed by a number of comma separated
types delimited by parenthesis.`]}),`
`,e.jsxs(n.h2,{id:"instantiation",children:["Instantiation",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#instantiation",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<union_instantiation> ::= <ident> "::" <ident> "(" [<expr> ("," <expr>)* [","]] ")" ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<union_instantiation>"}),` instantiates, or creates, the sum type. This
consists of the union's identifier, followed by the member's identifier,
followed by an optional comma separated list of expressions.`]}),`
`,e.jsx(n.p,{children:"Behavior of instantiation is defined in the data location rule."}),`
`,e.jsxs(n.h2,{id:"union-pattern",children:["Union Pattern",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#union-pattern",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<union_pattern> ::= <ident> "::" <ident> ["(" <ident> ("," <ident>)* [","] ")"];'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<union_pattern>"}),` is a pattern consisting of the union's name and
a member's name separated by a double colon.`]}),`
`,e.jsxs(n.h2,{id:"pattern-match",children:["Pattern Match",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#pattern-match",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<pattern_match> ::= <ident> "matches" <union_pattern> ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<ident>"})}),`
`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`All unions have a Unions where no member has its own internal type is
effectively an enumeration over integers.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Mutex = Locked | Unlocked;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// Mutex::Locked == 0"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// Mutex::Unlocked == 1"})})]})})}),`
`,e.jsx(n.p,{children:`Unions where any members have an internal type become proper type unions.
The only case in which a union can exist on the stack rather than another
data location is if the largest of the internal types has a bitsize of 248
or less. If any member's internal type is greater than 248, a data location
must be specified.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type StackUnion = A(u8) | B(u248);"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MemoryUnion = A(u256) | B | C(u8);"})})]})})}),`
`,e.jsx(n.p,{children:`A union pattern consists of its identifier and the member identifier
separated by colons. This pattern may be used both in match statements
and if statements.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsxs(n.span,{className:"line",children:[e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Option<"}),e.jsx(n.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"T"}),e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"> = None | Some(T);"})]}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsxs(n.span,{className:"line",children:[e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"impl Option<"}),e.jsx(n.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"T"}),e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"> {"})]}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    fn unwrap(self) -> T {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        match self {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"            Option::Some(inner) => return inner,"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"            Option::None => revert(),"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        };"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    fn unwrapOr(self, default: T) -> T {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        let mut value = default;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        if self matches Option::Some(inner) {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"            value = inner;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        return value;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})})]})}function t(i={}){const{wrapper:n}={...a(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(s,{...i})}):s(i)}export{t as default,l as frontmatter};
