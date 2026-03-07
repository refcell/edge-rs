import{u as d,j as e}from"./index-B0DkJJZv.js";const r={title:"Codesize",description:"undefined"};function t(i){const n={a:"a",aside:"aside",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",p:"p",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",...d(),...i.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"codesize",children:["Codesize",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#codesize",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:`This document details the different options for codesize optimization. Generally,
codesize and runtime efficiency are inversely correlated. Developers will have
granular control both in the compiler's configuration and in the language's syntax.`}),`
`,e.jsxs(n.h2,{id:"inlining-heuristics",children:["Inlining Heuristics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#inlining-heuristics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Function inlining is a direct tradeoff of codesize and runtime efficiency.
Codesize optimization may be used for reducing deployment cost or for keeping
the codesize below the EVM's codesize limit.`}),`
`,e.jsxs(n.h2,{id:"scoring",children:["Scoring",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#scoring",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Functions are assigned a score based on a combination of its projected bytecode
size, projected number of calls, and an optional manually entered score.`}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"Name"}),e.jsx(n.th,{children:"Score"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Bytecode Size"}),e.jsx(n.td,{children:e.jsx(n.code,{children:"fn.bytecode.len()"})})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Call Count"}),e.jsx(n.td,{children:e.jsx(n.code,{children:"fn.calls()"})})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Manual Score"}),e.jsx(n.td,{children:e.jsx(n.code,{children:"u8"})})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"Total"}),e.jsx(n.td,{children:e.jsx(n.code,{children:"(fn.bytecode.len() + 5 * fn.calls()) * man"})})]})]})]}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:"Todo: rewrite this scoring model based on gas estimations for each call."})}),`
`,e.jsx(n.p,{children:"A compiler configuration can be specified for the threshold for function inlining."}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:"Todo: decide on the compiler configuration threshold for function inlining."})}),`
`,e.jsxs(n.h2,{id:"analysis",children:["Analysis",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#analysis",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The analysis for function inline scoring requires the traversal of a directed
graph containing each function and other functions called within it. Traversal
is depth first, as function inline scores are dependent on their bytecode size
which is dependent on the inline scores of functions called within its body.
Once a terminal function, a function with no internal function dependencies,
is found, its inline score will be compared against the configuration threshold.
If the score is greater than the threshold, it is to be inlined and a flag will
be stored in the graph for future references.`}),`
`,e.jsx(n.p,{children:`Cycle detection will both prevent infinite loops in the compiler as well as
detect recursion and corecursion. Recursive and corecursive functions will
never be inlined for simplicity.`}),`
`,e.jsxs(n.h2,{id:"dead-code-elimination",children:["Dead Code Elimination",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#dead-code-elimination",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Eliminating dead code will cut codesize and improve the function inlining score,
as number of calls and projected codesize of each function are both factors in
the function inline score.`}),`
`,e.jsxs(n.h2,{id:"syntax-modifications",children:["Syntax Modifications",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#syntax-modifications",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.aside,{"data-callout":"note",children:e.jsx(n.p,{children:"Todo: syntax-level controls for codesize tuning are still being drafted."})})]})}function s(i={}){const{wrapper:n}={...d(),...i.components};return n?e.jsx(n,{...i,children:e.jsx(t,{...i})}):t(i)}export{s as default,r as frontmatter};
