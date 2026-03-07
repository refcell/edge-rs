import{u as i,j as e}from"./index-B0DkJJZv.js";const a={title:"Data Locations",description:"undefined"};function t(s){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",h3:"h3",header:"header",li:"li",p:"p",pre:"pre",span:"span",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...i(),...s.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"data-locations",children:["Data Locations",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#data-locations",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<storage_pointer> ::= "&s" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<transient_storage_pointer> ::= "&t" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<memory_pointer> ::= "&m" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<calldata_pointer> ::= "&cd" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<returndata_pointer> ::= "&rd" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<internal_code_pointer> ::= "&ic" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<external_code_pointer> ::= "&ec" ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<data_location> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <storage_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <transient_storage_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <memory_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <calldata_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <returndata_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <internal_code_pointer>"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"    | <external_code_pointer> ;"})})]})})}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<location>"}),` is a data location annotation indicating to which data
location a pointer's data exists. We define seven distinct annotations
for data location pointers. This is a divergence from general purpose
programming languages to more accurately represent the EVM execution
environment.`]}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&s"})," persistent storage"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&t"})," transient storage"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&m"})," memory"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&cd"})," calldata"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&rd"})," returndata"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&ic"})," internal (local) code"]}),`
`,e.jsxs(n.li,{children:[e.jsx(n.code,{children:"&ec"})," external code"]}),`
`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:"Data locations can be grouped into two broad categories, buffers and maps."}),`
`,e.jsxs(n.h3,{id:"maps",children:["Maps",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#maps",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Persistent and transient storage are part of the map category,
256 bit keys map to 256 bit values. Both may be written or read one
word at a time.`}),`
`,e.jsxs(n.h3,{id:"buffers",children:["Buffers",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#buffers",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Memory, calldata, returndata, internal code, and external code are
all linear data buffers. All can be either read to the stack or copied
into memory, but only memory can be written or copied to.`}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"Name"}),e.jsx(n.th,{children:"Read to Stack"}),e.jsx(n.th,{children:"Copy to Memory"}),e.jsx(n.th,{children:"Write"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"memory"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"true"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"calldata"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"false"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"returndata"}),e.jsx(n.td,{children:"false"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"false"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"internal code"}),e.jsx(n.td,{children:"false"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"false"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"external code"}),e.jsx(n.td,{children:"false"}),e.jsx(n.td,{children:"true"}),e.jsx(n.td,{children:"false"})]})]})]}),`
`,e.jsxs(n.h3,{id:"transitions",children:["Transitions",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#transitions",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Transitioning from map to memory buffer is performed by loading each
element from the map to the stack and storing each stack item in memory
O(N).`}),`
`,e.jsx(n.p,{children:`Transitioning from memory buffer to a map is performed by loading each
element from memory to the stack and storing each stack item in the map
O(N).`}),`
`,e.jsx(n.p,{children:`Transitioning from any other buffer to a map is performed by copying the
buffer's data into memory then transitioning the data from memory into the
map O(N+1).`}),`
`,e.jsxs(n.h3,{id:"pointer-bit-sizes",children:["Pointer Bit Sizes",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#pointer-bit-sizes",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Pointers to different data locations consist of different sizes based on
the properties of that data location. In depth semantics of each data
location are specified in the type system documents.`}),`
`,e.jsxs(n.table,{children:[e.jsx(n.thead,{children:e.jsxs(n.tr,{children:[e.jsx(n.th,{children:"Location"}),e.jsx(n.th,{children:"Bit Size"}),e.jsx(n.th,{children:"Reason"})]})}),e.jsxs(n.tbody,{children:[e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"persistent storage"}),e.jsx(n.td,{children:"256"}),e.jsx(n.td,{children:"Storage is 256 bit key value hashmap"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"transient storage"}),e.jsx(n.td,{children:"256"}),e.jsx(n.td,{children:"Transient storage is 256 bit key value hashmap"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"memory"}),e.jsx(n.td,{children:"32"}),e.jsx(n.td,{children:"Theoretical maximum memory size does not grow to 0xffffffff"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"calldata"}),e.jsx(n.td,{children:"32"}),e.jsx(n.td,{children:"Theoretical maximum calldata size does not grow to 0xffffffff"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"returndata"}),e.jsx(n.td,{children:"32"}),e.jsx(n.td,{children:"Maximum returndata size is equal to maximum memory size"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"internal code"}),e.jsx(n.td,{children:"16"}),e.jsx(n.td,{children:"Code size is less than 0xffff"})]}),e.jsxs(n.tr,{children:[e.jsx(n.td,{children:"external code"}),e.jsx(n.td,{children:"176"}),e.jsx(n.td,{children:"Contains 160 bit address and 16 bit code pointer"})]})]})]})]})}function d(s={}){const{wrapper:n}={...i(),...s.components};return n?e.jsx(n,{...s,children:e.jsx(t,{...s})}):t(s)}export{d as default,a as frontmatter};
