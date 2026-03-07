import{u as a,j as e}from"./index-B0DkJJZv.js";const r={title:"Product Types",description:"undefined"};function i(n){const s={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",table:"table",tbody:"tbody",td:"td",th:"th",thead:"thead",tr:"tr",ul:"ul",...a(),...n.components};return e.jsxs(e.Fragment,{children:[e.jsx(s.header,{children:e.jsxs(s.h1,{id:"product-types",children:["Product Types",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#product-types",children:e.jsx(s.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(s.p,{children:"The product type is a compound type composed of none or more internal types."}),`
`,e.jsxs(s.h2,{id:"signature",children:["Signature",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#signature",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<struct_field_signature> ::= <ident> ":" <type_signature> ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<struct_signature> ::="})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    ["packed"] "{"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'        [<struct_field_signature> ("," <struct_field_signature>)* [","]]'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "}" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<tuple_signature> ::= ["packed"] "(" <type_signature> ("," <type_signature>)* [","] ")" ;'})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<ident>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<type_parameters>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<type_signature>"})}),`
`]}),`
`,e.jsxs(s.h2,{id:"instantiation",children:["Instantiation",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#instantiation",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<struct_field_instantiation> ::= <ident> ":" <expr> ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<struct_instantiation> ::="})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    [<data_location>] <struct_signature> "{"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'        [<struct_field_instantiation> ("," <struct_field_instantiation>)* [","]]'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "}" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<tuple_instantiation> ::= [<data_location>] <ident> "(" [<expr> ("," <expr>)* [","]] ")" ;'})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<ident>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<expr>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<data_location>"})}),`
`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<struct_instantiation>"}),` is an instantiation, or creation, of a struct. It may
optionally include a data location annotation, however the semantic rules for this
are in the data location semantic rules. It is instantiated by the struct identifier
followed by a comma separated list of field name and value pairs delimited by curly braces.`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<tuple_instantiation>"}),` is an instantiation, or creation, of a tuple. It may
optionally include a data location annotation, however the semantic rules for this
are in the data location semantic rules. It is instantiated by a comma separated list
of expressions delimited by parenthesis.`]}),`
`,e.jsxs(s.h2,{id:"field-access",children:["Field Access",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#field-access",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<struct_field_access> ::= <ident> "." <ident> ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<tuple_field_access> ::= <ident> "." <dec_char> ;'})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<ident>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<dec_char>"})}),`
`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<struct_field_access>"}),` is written as the struct's identifier followed by the
field's identifier separated by a period.`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<tuple_field_access>"}),` is written as the tuple's identifier followed by the
field's index separated by a period.`]}),`
`,e.jsxs(s.h2,{id:"examples",children:["Examples",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#examples",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type PrimitiveStruct = {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    a: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    b: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    c: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"};"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const primitiveStruct: PrimitiveStruct = PrimitiveStruct { a: 1, b: 2, c: 3 };"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const a = primitiveStruct.a;"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type PackedTuple = packed (u8, u8, u8);"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const packedTuple: PackedTuple = (1, 2, 3);"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const one = packedTuple.0;"})})]})})}),`
`,e.jsxs(s.h2,{id:"semantics",children:["Semantics",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(s.p,{children:`The struct field signature maps a type identifier to a type signature. The
field may be accessed by the struct's identifier and field identifier separated
by a dot.`}),`
`,e.jsx(s.p,{children:`Prefixing the signature with the "packed" keyword will pack the fields by their
bitsize, otherwise each field is padded to its own 256 bit word.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Rgb = packed { r: u8, g: u8, b: u8 };"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"let rgb = Rgb { r: 1, g: 2, b: 3 };"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// rgb = 0x010203"})})]})})}),`
`,e.jsx(s.p,{children:`Instantiation depends on the data location. Structs that can fit into a single word,
either a single field struct or a packed struct with a bitsize sum less than or equal
to 256, sit on the stack by default. Instantiating a struct in memory requires the
memory data location annotation. If a struct that does not fit into a single word
does not have a data location annotation, a compiler error is thrown.`}),`
`,e.jsx(s.p,{children:`Stack struct instantiation consists of optionally bitpacking fields and leaving the
struct on the stack. Memory instantiation consists of allocating new memory, optionally
bitpacking fields, storing the struct in memory, and leaving the pointer to it on the
stack.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type MemoryRgb = { r: u8, g: u8, b: u8 };"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"let memoryRgb = MemoryRgb{ r: 1, g: 2, b: 3 };"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// ptr = .."})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// mstore(ptr, 1)"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// mstore(add(32, ptr), 2)"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"// mstore(add(64, ptr), 3)"})})]})})}),`
`,e.jsx(s.p,{children:`Persistent and transient storage structs must be instantiated at the file level. If
anything except zero values are assigned, storage writes will be injected into the initcode
to be run on deployment. A reasonable convention for creating a storage layout without
the contract object abstraction would be to create a Storage type which is a struct,
mapping identifiers to storage slots. Nested structs will also allow granular control
over which variables get packed.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Storage = {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    a: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    b: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    c: packed {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        a: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        b: u8"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsxs(s.span,{className:"line",children:[e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const storage = @default<"}),e.jsx(s.span,{style:{color:"#B31D28",fontStyle:"italic","--shiki-dark":"#FF938A","--shiki-dark-font-style":"italic"},children:"Storage"}),e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:">();"})]}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn main() {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    storage.a = 1;      // sstore(0, 1)"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    storage.b = 2;      // sstore(1, 2)"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    storage.c.a = 3;    // ca = shl(8, 3)"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    storage.c.b = 4;    // sstore(2, or(ca, 4))"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsx(s.p,{children:`Packing rules for buffer locations is to pack everything exactly by its bit length.
Packing rules for map locations is to right-align the first field, for each subsequent
field, if its bitsize fits into the same word as the previous, it is left-shifted to
the first available bits, otherwise, if the bitsize would overflow, it becomes a new
word.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Storage = {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    a: u128,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    b: u8,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    c: addr,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    d: u256"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})}),`
`,e.jsx(s.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"const storage = Storage {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    a: 1,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    b: 2,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    c: 0x3,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    d: 4,"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"};"})})]})})}),`
`,e.jsxs(s.table,{children:[e.jsx(s.thead,{children:e.jsxs(s.tr,{children:[e.jsx(s.th,{children:"Slot"}),e.jsx(s.th,{children:"Value"})]})}),e.jsxs(s.tbody,{children:[e.jsxs(s.tr,{children:[e.jsx(s.td,{children:"0x00"}),e.jsx(s.td,{children:"0x0000000000000000000000000000000200000000000000000000000000000001"})]}),e.jsxs(s.tr,{children:[e.jsx(s.td,{children:"0x01"}),e.jsx(s.td,{children:"0x0000000000000000000000000000000000000000000000000000000000000003"})]}),e.jsxs(s.tr,{children:[e.jsx(s.td,{children:"0x02"}),e.jsx(s.td,{children:"0x0000000000000000000000000000000000000000000000000000000000000004"})]})]})]})]})}function t(n={}){const{wrapper:s}={...a(),...n.components};return s?e.jsx(s,{...n,children:e.jsx(i,{...n})}):i(n)}export{t as default,r as frontmatter};
