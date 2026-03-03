//! EVM bytecode code generator

use crate::opcode::Opcode;
use indexmap::IndexMap;

/// The input to the code generator (mirrors `IrProgram` structure)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenInput {
    /// All contracts to generate code for
    pub contracts: Vec<ContractInput>,
}

/// A single contract's code generation input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractInput {
    /// Contract name (for metadata)
    pub name: String,
    /// All functions
    pub functions: Vec<FunctionInput>,
}

/// A function's code generation input
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionInput {
    /// Function name
    pub name: String,
    /// 4-byte ABI selector
    pub selector: [u8; 4],
    /// Whether publicly callable
    pub is_pub: bool,
    /// IR instructions for the function body
    pub body: Vec<GenInstr>,
}

/// Simplified IR instructions for code generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenInstr {
    /// Push bytes onto stack
    Push(Vec<u8>),
    /// Pop top of stack
    Pop,
    /// Duplicate nth stack item (1-indexed)
    Dup(u8),
    /// Swap top with nth item (1-indexed)
    Swap(u8),
    /// ADD
    Add,
    /// SUB
    Sub,
    /// MUL
    Mul,
    /// DIV
    Div,
    /// MOD
    Mod,
    /// LT
    Lt,
    /// GT
    Gt,
    /// EQ
    Eq,
    /// ISZERO
    IsZero,
    /// AND
    And,
    /// OR
    Or,
    /// XOR
    Xor,
    /// NOT
    Not,
    /// SHL
    Shl,
    /// SHR
    Shr,
    /// SLOAD
    SLoad,
    /// SSTORE
    SStore,
    /// MLOAD
    MLoad,
    /// MSTORE
    MStore,
    /// CALLDATALOAD
    CallDataLoad,
    /// CALLDATASIZE
    CallDataSize,
    /// CALLER
    Caller,
    /// CALLVALUE
    CallValue,
    /// NUMBER
    Number,
    /// TIMESTAMP
    Timestamp,
    /// KECCAK256
    Keccak256,
    /// `LOG(n_topics)`
    Log(u8),
    /// JUMP
    Jump,
    /// JUMPI
    JumpI,
    /// JUMPDEST with label name
    JumpDest(String),
    /// Push label address (resolved in second pass)
    PushLabel(String),
    /// RETURN
    Return,
    /// REVERT
    Revert,
    /// STOP
    Stop,
}

/// Error type for code generation
#[derive(Debug, thiserror::Error)]
pub enum CodeGenError {
    /// A label was referenced but never defined
    #[error("undefined label: {0}")]
    UndefinedLabel(String),
    /// An unsupported instruction was encountered
    #[error("unsupported instruction: {0}")]
    UnsupportedInstruction(String),
    /// No public functions in contract
    #[error("no public functions found")]
    NoPublicFunctions,
}

/// The EVM bytecode code generator
#[derive(Debug, Default)]
pub struct CodeGenerator;

impl CodeGenerator {
    /// Create a new code generator
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Generate EVM runtime bytecode for a contract
    ///
    /// The output is the runtime bytecode (not deploy bytecode).
    /// Layout:
    /// 1. Dispatcher: reads calldata\[0:4\], routes to function by selector
    /// 2. Function bodies (one after another)
    /// 3. Revert fallback at end (for unknown selectors)
    pub fn generate(&self, input: &ContractInput) -> Result<Vec<u8>, CodeGenError> {
        let mut assembler = Assembler::new();

        // Emit dispatcher
        self.emit_dispatcher(&mut assembler, input)?;

        // Emit each public function
        for func in &input.functions {
            if func.is_pub {
                self.emit_function(&mut assembler, func)?;
            }
        }

        // Revert block for unknown selectors
        assembler.emit_jumpdest("__revert__");
        assembler.emit_push(&[0]);
        assembler.emit_push(&[0]);
        assembler.emit_opcode(Opcode::Revert);

        assembler.resolve_labels()
    }

    fn emit_dispatcher(
        &self,
        asm: &mut Assembler,
        input: &ContractInput,
    ) -> Result<(), CodeGenError> {
        let pub_fns: Vec<_> = input.functions.iter().filter(|f| f.is_pub).collect();

        if pub_fns.is_empty() {
            // No public functions, just revert
            asm.emit_push(&[0]);
            asm.emit_push(&[0]);
            asm.emit_opcode(Opcode::Revert);
            return Ok(());
        }

        // Load 4-byte selector from calldata
        asm.emit_push(&[0x00]); // offset 0
        asm.emit_opcode(Opcode::CalldataLoad); // push calldata[0..32]
        asm.emit_push(&[0xe0]); // 224 = 32*7
        asm.emit_opcode(Opcode::Shr); // SHR → top 4 bytes of calldata

        // Route to each function
        for func in &pub_fns {
            let fn_label = format!("fn_{}", func.name);
            asm.emit_opcode(Opcode::Dup1); // dup selector
            asm.emit_push(&func.selector); // push expected selector
            asm.emit_opcode(Opcode::Eq); // compare
            asm.emit_push_label(&fn_label); // push fn label dest
            asm.emit_opcode(Opcode::JumpI); // jump if match
        }

        // No match → revert
        asm.emit_push_label("__revert__");
        asm.emit_opcode(Opcode::Jump);

        Ok(())
    }

    fn emit_function(
        &self,
        asm: &mut Assembler,
        func: &FunctionInput,
    ) -> Result<(), CodeGenError> {
        let fn_label = format!("fn_{}", func.name);
        asm.emit_jumpdest(&fn_label); // function entry point
        asm.emit_opcode(Opcode::Pop); // pop the selector that's still on stack from dispatcher

        // Emit all IR instructions
        for instr in &func.body {
            self.emit_instr(asm, instr)?;
        }

        // If no explicit RETURN/STOP at end, add STOP
        let last_is_terminal = func.body.last().is_some_and(|i| {
            matches!(
                i,
                GenInstr::Return
                    | GenInstr::Revert
                    | GenInstr::Stop
                    | GenInstr::Jump
                    | GenInstr::JumpI
            )
        });
        if !last_is_terminal {
            asm.emit_opcode(Opcode::Stop);
        }

        Ok(())
    }

    fn emit_instr(&self, asm: &mut Assembler, instr: &GenInstr) -> Result<(), CodeGenError> {
        match instr {
            GenInstr::Push(bytes) => asm.emit_push(bytes),
            GenInstr::Pop => asm.emit_opcode(Opcode::Pop),
            GenInstr::Dup(n) => {
                let op = match n {
                    1 => Opcode::Dup1,
                    2 => Opcode::Dup2,
                    3 => Opcode::Dup3,
                    4 => Opcode::Dup4,
                    5 => Opcode::Dup5,
                    6 => Opcode::Dup6,
                    7 => Opcode::Dup7,
                    8 => Opcode::Dup8,
                    9 => Opcode::Dup9,
                    10 => Opcode::Dup10,
                    11 => Opcode::Dup11,
                    12 => Opcode::Dup12,
                    13 => Opcode::Dup13,
                    14 => Opcode::Dup14,
                    15 => Opcode::Dup15,
                    _ => Opcode::Dup16,
                };
                asm.emit_opcode(op);
            }
            GenInstr::Swap(n) => {
                let op = match n {
                    1 => Opcode::Swap1,
                    2 => Opcode::Swap2,
                    3 => Opcode::Swap3,
                    4 => Opcode::Swap4,
                    5 => Opcode::Swap5,
                    6 => Opcode::Swap6,
                    7 => Opcode::Swap7,
                    8 => Opcode::Swap8,
                    9 => Opcode::Swap9,
                    10 => Opcode::Swap10,
                    11 => Opcode::Swap11,
                    12 => Opcode::Swap12,
                    13 => Opcode::Swap13,
                    14 => Opcode::Swap14,
                    15 => Opcode::Swap15,
                    _ => Opcode::Swap16,
                };
                asm.emit_opcode(op);
            }
            GenInstr::Add => asm.emit_opcode(Opcode::Add),
            GenInstr::Sub => asm.emit_opcode(Opcode::Sub),
            GenInstr::Mul => asm.emit_opcode(Opcode::Mul),
            GenInstr::Div => asm.emit_opcode(Opcode::Div),
            GenInstr::Mod => asm.emit_opcode(Opcode::Mod),
            GenInstr::Lt => asm.emit_opcode(Opcode::Lt),
            GenInstr::Gt => asm.emit_opcode(Opcode::Gt),
            GenInstr::Eq => asm.emit_opcode(Opcode::Eq),
            GenInstr::IsZero => asm.emit_opcode(Opcode::IsZero),
            GenInstr::And => asm.emit_opcode(Opcode::And),
            GenInstr::Or => asm.emit_opcode(Opcode::Or),
            GenInstr::Xor => asm.emit_opcode(Opcode::Xor),
            GenInstr::Not => asm.emit_opcode(Opcode::Not),
            GenInstr::Shl => asm.emit_opcode(Opcode::Shl),
            GenInstr::Shr => asm.emit_opcode(Opcode::Shr),
            GenInstr::SLoad => asm.emit_opcode(Opcode::SLoad),
            GenInstr::SStore => asm.emit_opcode(Opcode::SStore),
            GenInstr::MLoad => asm.emit_opcode(Opcode::MLoad),
            GenInstr::MStore => asm.emit_opcode(Opcode::MStore),
            GenInstr::CallDataLoad => asm.emit_opcode(Opcode::CalldataLoad),
            GenInstr::CallDataSize => asm.emit_opcode(Opcode::CalldataSize),
            GenInstr::Caller => asm.emit_opcode(Opcode::Caller),
            GenInstr::CallValue => asm.emit_opcode(Opcode::CallValue),
            GenInstr::Number => asm.emit_opcode(Opcode::Number),
            GenInstr::Timestamp => asm.emit_opcode(Opcode::Timestamp),
            GenInstr::Keccak256 => asm.emit_opcode(Opcode::Keccak256),
            GenInstr::Log(n) => {
                let op = match n {
                    0 => Opcode::Log0,
                    1 => Opcode::Log1,
                    2 => Opcode::Log2,
                    3 => Opcode::Log3,
                    _ => Opcode::Log4,
                };
                asm.emit_opcode(op);
            }
            GenInstr::Jump => asm.emit_opcode(Opcode::Jump),
            GenInstr::JumpI => asm.emit_opcode(Opcode::JumpI),
            GenInstr::JumpDest(label) => asm.emit_jumpdest(label),
            GenInstr::PushLabel(label) => asm.emit_push_label(label),
            GenInstr::Return => asm.emit_opcode(Opcode::Return),
            GenInstr::Revert => asm.emit_opcode(Opcode::Revert),
            GenInstr::Stop => asm.emit_opcode(Opcode::Stop),
        }
        Ok(())
    }
}

/// Internal assembler that handles label resolution
struct Assembler {
    /// Raw bytes being built
    bytes: Vec<u8>,
    /// Label name → byte offset
    label_offsets: IndexMap<String, u32>,
    /// (`byte_offset`, `label_name`) — places where label refs need patching
    label_refs: Vec<(usize, String)>,
}

impl Assembler {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            label_offsets: IndexMap::new(),
            label_refs: Vec::new(),
        }
    }

    /// Emit a single opcode byte
    fn emit_opcode(&mut self, op: Opcode) {
        self.bytes.push(op as u8);
    }

    /// Emit PUSH<n> + bytes
    fn emit_push(&mut self, data: &[u8]) {
        if data.is_empty() {
            self.bytes.push(Opcode::Push0 as u8);
            return;
        }
        let n = data.len().min(32);
        self.bytes.push(Opcode::push_for_size(n) as u8);
        self.bytes.extend_from_slice(&data[..n]);
    }

    /// Emit a PUSH2 for a label reference (2-byte destination placeholder)
    /// Records the patch location.
    fn emit_push_label(&mut self, label: &str) {
        self.bytes.push(Opcode::Push2 as u8);
        let ref_offset = self.bytes.len();
        self.bytes.push(0x00); // placeholder high byte
        self.bytes.push(0x00); // placeholder low byte
        self.label_refs.push((ref_offset, label.to_string()));
    }

    /// Define a label at the current position
    fn define_label(&mut self, label: &str) {
        let offset = self.bytes.len() as u32;
        self.label_offsets.insert(label.to_string(), offset);
    }

    /// Emit JUMPDEST and define the label
    fn emit_jumpdest(&mut self, label: &str) {
        self.emit_opcode(Opcode::JumpDest);
        self.define_label(label);
    }

    /// Finalize: patch all label references and return bytes
    fn resolve_labels(mut self) -> Result<Vec<u8>, CodeGenError> {
        for (ref_offset, label) in &self.label_refs {
            let target = self.label_offsets.get(label.as_str())
                .copied()
                .ok_or_else(|| CodeGenError::UndefinedLabel(label.clone()))?;
            // Write 2-byte big-endian target
            self.bytes[*ref_offset] = (target >> 8) as u8;
            self.bytes[*ref_offset + 1] = (target & 0xff) as u8;
        }
        Ok(self.bytes)
    }
}
