import{u as i,j as e}from"./index-B0DkJJZv.js";const d={title:"Inline Assembly",description:"undefined"};function a(n){const s={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...i(),...n.components};return e.jsxs(e.Fragment,{children:[e.jsx(s.header,{children:e.jsxs(s.h1,{id:"inline-assembly",children:["Inline Assembly",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#inline-assembly",children:e.jsx(s.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsxs(s.h2,{id:"opcodes",children:["Opcodes",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#opcodes",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<opcode> ::="})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "stop" | "add" | "mul" | "sub" | "div" | "sdiv" | "mod" | "smod" | "addmod" | "mulmod" | "exp"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "signextend" | "lt" | "gt" | "slt" | "sgt" | "eq" | "iszero" | "and" | "or" | "xor" | "not"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "byte" | "shl" | "shr" | "sar" | "sha3" | "address" | "balance" | "origin" | "caller"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "callvalue" | "calldataload" | "calldatasize" | "calldatacopy" | "codesize" | "codecopy"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "gasprice" | "extcodesize" | "extcodecopy" | "returndatasize" | "returndatacopy"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "extcodehash" | "blockhash" | "coinbase" | "timestamp" | "number" | "prevrandao" | "gaslimit"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "chainid" | "selfbalance" | "basefee" | "pop" | "mload" | "mstore" | "mstore8" | "sload"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "sstore" | "jump" | "jumpi" | "pc" | "msize" | "gas" | "jumpdest" | "push0" | "dup1" | "dup2"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "dup3" | "dup4" | "dup5" | "dup6" | "dup7" | "dup8" | "dup9" | "dup10" | "dup11" | "dup12"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "dup13" | "dup14" | "dup15" | "dup16" | "swap1" | "swap2" | "swap3" | "swap4" | "swap5"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "swap6" | "swap7" | "swap8" | "swap9" | "swap10" | "swap11" | "swap12" | "swap13" | "swap14"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "swap15" | "swap16" | "log0" | "log1" | "log2" | "log3" | "log4" | "create" | "call"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "callcode" | "return" | "delegatecall" | "create2" | "staticcall" | "revert" | "invalid"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    | "selfdestruct" | <numeric_literal> | <ident> ;'})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<numeric_literal>"})}),`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<ident>"})}),`
`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<opcode>"}),` is one of the mnemonic EVM instructions,
or a numeric literal, or an identifier.`]}),`
`,e.jsxs(s.h2,{id:"inline-assembly-block",children:["Inline Assembly Block",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#inline-assembly-block",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'<assembly_output> ::= <ident> | "_" ;'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:"<inline_assembly> ::="})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "asm"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "(" [<expr> ("," <expr>)* [","]] ")"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "->" "(" [<assembly_output> ("," <assembly_output>)* [","]] ")"'})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{children:'    "{" (<opcode>)* "}"'})})]})})}),`
`,e.jsx(s.p,{children:"Dependencies:"}),`
`,e.jsxs(s.ul,{children:[`
`,e.jsx(s.li,{children:e.jsx(s.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(s.p,{children:["The ",e.jsx(s.code,{children:"<inline_assembly>"}),` consists of the "asm" keyword,
followed by an optional comma separated, parenthesis delimited
list of argument expressions, then an arrow, an optional comma
separated, parenthesis delimited list of return identifiers,
and finally a code block containing only the `,e.jsx(s.code,{children:"<opcodes>"}),"."]}),`
`,e.jsxs(s.h2,{id:"semantics",children:["Semantics",e.jsx(s.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(s.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(s.p,{children:`Arguments are ordered such that the state of the stack at the
start of the block, top to bottom, is the list of arguments,
left to right. Identifiers in the output list are ordered such
that the state of the stack at the end of the assembly block,
top to bottom, is the list of outputs, left to right.`}),`
`,e.jsx(s.p,{children:`Note that if the input arguments contain local variables, the
stack scheduling required to construct the pre-assembly stack
state may be unprofitable in cases with small assembly code
blocks.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(s.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(s.code,{children:[e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"asm (1, 2, 3) -> (a) {"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    // state:   // [1, 2, 3]"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    add         // [3, 3]"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    mul         // [9]"})}),`
`,e.jsx(s.span,{className:"line",children:e.jsx(s.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsx(s.p,{children:`Inside the assembly block, numeric literals are implicitly
converted into pushN instructions. All literals are put into
the smallest N for pushN by bits, however, this is also
accounting for leading zeros. For example, 0x0000 would
become push2 0000 to allow for bytecode padding. Identifiers
may be variables, constants, or ad hoc opcodes. When identifiers
are variables, they are scheduled in the stack. When identifiers
are constants, they are replaced with their push instructions
just as numeric literals are. When identifiers are ad hoc opcodes,
they are replaced with their respective byte(s).`})]})}function r(n={}){const{wrapper:s}={...i(),...n.components};return s?e.jsx(s,{...n,children:e.jsx(a,{...n})}):a(n)}export{r as default,d as frontmatter};
