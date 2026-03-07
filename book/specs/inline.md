# Inline Assembly

## Opcodes

```
<opcode> ::=
    | "stop" | "add" | "mul" | "sub" | "div" | "sdiv" | "mod" | "smod" | "addmod" | "mulmod" | "exp"
    | "signextend" | "lt" | "gt" | "slt" | "sgt" | "eq" | "iszero" | "and" | "or" | "xor" | "not"
    | "byte" | "shl" | "shr" | "sar" | "sha3" | "address" | "balance" | "origin" | "caller"
    | "callvalue" | "calldataload" | "calldatasize" | "calldatacopy" | "codesize" | "codecopy"
    | "gasprice" | "extcodesize" | "extcodecopy" | "returndatasize" | "returndatacopy"
    | "extcodehash" | "blockhash" | "coinbase" | "timestamp" | "number" | "prevrandao" | "gaslimit"
    | "chainid" | "selfbalance" | "basefee" | "pop" | "mload" | "mstore" | "mstore8" | "sload"
    | "sstore" | "jump" | "jumpi" | "pc" | "msize" | "gas" | "jumpdest" | "push0" | "dup1" | "dup2"
    | "dup3" | "dup4" | "dup5" | "dup6" | "dup7" | "dup8" | "dup9" | "dup10" | "dup11" | "dup12"
    | "dup13" | "dup14" | "dup15" | "dup16" | "swap1" | "swap2" | "swap3" | "swap4" | "swap5"
    | "swap6" | "swap7" | "swap8" | "swap9" | "swap10" | "swap11" | "swap12" | "swap13" | "swap14"
    | "swap15" | "swap16" | "log0" | "log1" | "log2" | "log3" | "log4" | "create" | "call"
    | "callcode" | "return" | "delegatecall" | "create2" | "staticcall" | "revert" | "invalid"
    | "selfdestruct" | <numeric_literal> | <ident> ;
```

Dependencies:

* `<numeric_literal>`
* `<ident>`

The `<opcode>` is one of the mnemonic EVM instructions,
or a numeric literal, or an identifier.

## Inline Assembly Block

```
<assembly_output> ::= <ident> | "_" ;

<inline_assembly> ::=
    "asm"
    "(" [<expr> ("," <expr>)* [","]] ")"
    "->" "(" [<assembly_output> ("," <assembly_output>)* [","]] ")"
    "{" (<opcode>)* "}"
```

Dependencies:

* `<expr>`

The `<inline_assembly>` consists of the "asm" keyword,
followed by an optional comma separated, parenthesis delimited
list of argument expressions, then an arrow, an optional comma
separated, parenthesis delimited list of return identifiers,
and finally a code block containing only the `<opcodes>`.

## Semantics

Arguments are ordered such that the state of the stack at the
start of the block, top to bottom, is the list of arguments,
left to right. Identifiers in the output list are ordered such
that the state of the stack at the end of the assembly block,
top to bottom, is the list of outputs, left to right.

Note that if the input arguments contain local variables, the
stack scheduling required to construct the pre-assembly stack
state may be unprofitable in cases with small assembly code
blocks.

```
asm (1, 2, 3) -> (a) {
    // state:   // [1, 2, 3]
    add         // [3, 3]
    mul         // [9]
}
```

Inside the assembly block, numeric literals are implicitly
converted into pushN instructions. All literals are put into
the smallest N for pushN by bits, however, this is also
accounting for leading zeros. For example, 0x0000 would
become push2 0000 to allow for bytecode padding. Identifiers
may be variables, constants, or ad hoc opcodes. When identifiers
are variables, they are scheduled in the stack. When identifiers
are constants, they are replaced with their push instructions
just as numeric literals are. When identifiers are ad hoc opcodes,
they are replaced with their respective byte(s).
