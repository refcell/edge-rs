import{u as s,j as e}from"./index-B0DkJJZv.js";const l={title:"Loops",description:"undefined"};function o(n){const i={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...s(),...n.components};return e.jsxs(e.Fragment,{children:[e.jsx(i.header,{children:e.jsxs(i.h1,{id:"loops",children:["Loops",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#loops",children:e.jsx(i.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(i.p,{children:`Loops are blocks of code that may be executed repeatedly
based on some conditions.`}),`
`,e.jsxs(i.h2,{id:"loop-control",children:["Loop Control",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#loop-control",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(i.code,{children:[e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<loop_break> ::= "break" ;'})}),`
`,e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<loop_continue> ::= "continue" ;'})})]})})}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<loop_break>"}),` keyword "breaks" the loop's execution,
jumping to the end of the loop immediately.`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<loop_continue>"}),` keyword "continues" the loop's
execution from the start, short circuiting the remainder
of the loop.`]}),`
`,e.jsxs(i.h2,{id:"loop-block",children:["Loop Block",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#loop-block",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<loop_block> ::= "{" ((<expr> | <stmt> | <loop_break> | <loop_continue>) ";")* "}" ;'})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<expr>"})}),`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<stmt>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<loop_block>"}),` is a block of code to be executed
repeatedly. All other loops are derived from this single
loop block.`]}),`
`,e.jsxs(i.h2,{id:"core-loop",children:["Core Loop",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#core-loop",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<core_loop> ::= "loop" <loop_block> ;'})})})})}),`
`,e.jsx(i.p,{children:`The core loop block is the simplest of blocks, it contains
no code to be injected anywhere else. All other loops are
syntactic sugar over the core loop. The "desugaring" step
for each loop is in the control flow semantic rules.`}),`
`,e.jsxs(i.h2,{id:"for-loop",children:["For Loop",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#for-loop",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<for_loop> ::= "for" "(" [(<stmt> | <expr>)]";" [<expr>] ";" [(<stmt> | <expr>)] ")" <loop_block> ;'})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<expr>"})}),`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<stmt>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<for_loop>"}),` is a loop block prefixed with three
individually optional items. The first may be a statement
or expression, the second may only be an expression, and
the third may be an expression or statement.`]}),`
`,e.jsxs(i.h2,{id:"while-loop",children:["While Loop",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#while-loop",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<while_loop> ::= "while" "(" <expr> ")" <loop_block> ;'})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<while_loop>"}),` is a loop block prefixed with one
required expression.`]}),`
`,e.jsxs(i.h2,{id:"do-while-loop",children:["Do While Loop",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#do-while-loop",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(i.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(i.code,{children:e.jsx(i.span,{className:"line",children:e.jsx(i.span,{children:'<do_while_loop> ::= "do" "while" <loop_block> "(" <expr> ")" ;'})})})})}),`
`,e.jsx(i.p,{children:"Dependencies:"}),`
`,e.jsxs(i.ul,{children:[`
`,e.jsx(i.li,{children:e.jsx(i.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(i.p,{children:["The ",e.jsx(i.code,{children:"<do_while_loop>"}),` is a loop block suffixed with
one required expression.`]}),`
`,e.jsxs(i.h2,{id:"semantics",children:["Semantics",e.jsx(i.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(i.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(i.aside,{"data-callout":"note",children:e.jsx(i.p,{children:"Todo: the loop semantics section is still under construction."})})]})}function r(n={}){const{wrapper:i}={...s(),...n.components};return i?e.jsx(i,{...n,children:e.jsx(o,{...n})}):o(n)}export{r as default,l as frontmatter};
