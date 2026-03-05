//! The main compiler struct that orchestrates the compilation pipeline

use std::fs;

use edge_ast::{stmt::Stmt, ty::EventField, Program};
use edge_diagnostics::Diagnostic;
use edge_ir::EventMeta;
use edge_lexer::lexer::Lexer;
use edge_parser::Parser;
use edge_types::tokens::Token;
use indexmap::IndexMap;
use tiny_keccak::{Hasher, Keccak};

use crate::{
    config::{CompilerConfig, EmitKind},
    session::Session,
};

/// Output from a compilation
#[derive(Debug)]
pub struct CompileOutput {
    /// Emitted tokens (if emit=tokens)
    pub tokens: Option<Vec<Token>>,
    /// Emitted AST (if emit=ast)
    pub ast: Option<Program>,
    /// Emitted runtime bytecode for the last contract (backward compat)
    pub bytecode: Option<Vec<u8>>,
    /// Emitted runtime bytecodes for all contracts, keyed by contract name
    pub bytecodes: Option<IndexMap<String, Vec<u8>>>,
    /// Deploy (initcode) bytecodes for all contracts
    pub deploy_bytecodes: Option<IndexMap<String, Vec<u8>>>,
}

/// Compiler errors
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    /// I/O error reading source
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Lexer errors were encountered
    #[error("lex errors encountered")]
    LexErrors,
    /// Parse errors were encountered
    #[error("parse errors encountered")]
    ParseErrors,
    /// Type check errors were encountered
    #[error("type check errors encountered")]
    TypeCheckErrors,
    /// IR lowering errors were encountered
    #[error("IR lowering errors encountered")]
    LowerErrors,
    /// Code generation errors were encountered
    #[error("code generation errors encountered")]
    CodeGenErrors,
    /// Compilation aborted due to errors
    #[error("compilation aborted due to errors")]
    Aborted,
}

/// The Edge compiler
#[derive(Debug)]
pub struct Compiler {
    session: Session,
}

impl Compiler {
    /// Create a new compiler with the given config
    pub fn new(config: CompilerConfig) -> Result<Self, CompileError> {
        let source = fs::read_to_string(&config.input_file)?;
        Ok(Self {
            session: Session::new(config, source),
        })
    }

    /// Run the compilation pipeline
    pub fn compile(&mut self) -> Result<CompileOutput, CompileError> {
        tracing::info!("Compiling {:?}", self.session.config.input_file);

        let emit = self.session.config.emit;

        // Lex phase
        let tokens = self.lex()?;

        if emit == EmitKind::Tokens {
            return Ok(CompileOutput {
                tokens: Some(tokens),
                ast: None,
                bytecode: None,
                bytecodes: None,
                deploy_bytecodes: None,
            });
        }

        // Parse phase
        let ast = self.parse()?;

        if emit == EmitKind::Ast {
            return Ok(CompileOutput {
                tokens: None,
                ast: Some(ast),
                bytecode: None,
                bytecodes: None,
                deploy_bytecodes: None,
            });
        }

        // Type check pass
        let checked = edge_typeck::TypeChecker::new().check(&ast).map_err(|e| {
            self.session
                .emit_error(Diagnostic::error(format!("type error: {e}")));
            self.session.diagnostics.report_all(&self.session.source);
            CompileError::TypeCheckErrors
        })?;

        // Extract event declarations from the AST (used for emit→LOG lowering)
        let event_metas = Self::collect_event_metas(&ast);

        // Lower to IR and generate bytecode for each contract
        let mut all_bytecodes: IndexMap<String, Vec<u8>> = IndexMap::new();
        let mut all_deploy_bytecodes: IndexMap<String, Vec<u8>> = IndexMap::new();

        for contract_info in &checked.contracts {
            // Build storage slots for lowerer
            let storage_slots = contract_info.storage.slots.clone();

            // Build function metadata for lowerer
            let fn_metas: Vec<edge_ir::FnMeta> = contract_info
                .functions
                .iter()
                .map(|f| edge_ir::FnMeta {
                    name: f.name.clone(),
                    selector: f.selector,
                    is_pub: f.is_pub,
                    params: f.params.iter().map(|(name, _type)| name.clone()).collect(),
                })
                .collect();

            // Lower AST to IR — only lower the target contract so each contract
            // gets its own storage slots rather than sharing one set across the
            // entire program (avoids cross-contamination in multi-contract files).
            let lowerer =
                edge_ir::Lowerer::new(storage_slots, fn_metas, event_metas.clone());
            let ir_program = lowerer.lower_one(&ast, &contract_info.name).map_err(|e| {
                self.session
                    .emit_error(Diagnostic::error(format!("IR lowering error: {e}")));
                self.session.diagnostics.report_all(&self.session.source);
                CompileError::LowerErrors
            })?;

            // Find the matching IR contract
            let ir_contract = ir_program
                .contracts
                .iter()
                .find(|c| c.name == contract_info.name)
                .ok_or_else(|| {
                    self.session.emit_error(Diagnostic::error(format!(
                        "contract {} not found in IR program",
                        contract_info.name
                    )));
                    CompileError::Aborted
                })?;

            // Convert IR to codegen input
            let contract_input = Self::ir_to_codegen(ir_contract);
            let codegen = edge_codegen::CodeGenerator::new();

            // Generate runtime bytecode
            let bytecode = codegen.generate(&contract_input).map_err(|e| {
                self.session
                    .emit_error(Diagnostic::error(format!("codegen error: {e}")));
                CompileError::CodeGenErrors
            })?;

            // Generate deploy (initcode) bytecode
            let deploy_bytecode = codegen.generate_deploy(&contract_input).map_err(|e| {
                self.session
                    .emit_error(Diagnostic::error(format!("deploy codegen error: {e}")));
                CompileError::CodeGenErrors
            })?;

            all_bytecodes.insert(contract_info.name.clone(), bytecode);
            all_deploy_bytecodes.insert(contract_info.name.clone(), deploy_bytecode);
        }

        // Last contract's bytecode for backward compatibility
        let last_bytecode = all_bytecodes.values().last().cloned().unwrap_or_default();

        Ok(CompileOutput {
            tokens: None,
            ast: Some(ast),
            bytecode: Some(last_bytecode),
            bytecodes: Some(all_bytecodes),
            deploy_bytecodes: Some(all_deploy_bytecodes),
        })
    }

    /// Run the lexer and collect tokens
    fn lex(&mut self) -> Result<Vec<Token>, CompileError> {
        // Clone source to avoid borrow conflict with session during error reporting
        let source = self.session.source.clone();
        let lexer = Lexer::new(&source);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        for result in lexer {
            match result {
                Ok(token) => tokens.push(token),
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            for e in errors {
                self.session.emit_error(
                    Diagnostic::error(format!("lexer error: {e:?}"))
                        .with_label(e.span, "invalid token here"),
                );
            }
            self.session.diagnostics.report_all(&self.session.source);
            return Err(CompileError::LexErrors);
        }

        Ok(tokens)
    }

    /// Run the parser and produce an AST
    fn parse(&mut self) -> Result<Program, CompileError> {
        let mut parser = Parser::new(&self.session.source).map_err(|e| {
            self.session
                .emit_error(Diagnostic::error(format!("parse error: {e}")));
            CompileError::ParseErrors
        })?;

        match parser.parse() {
            Ok(program) => Ok(program),
            Err(e) => {
                self.session
                    .emit_error(Diagnostic::error(format!("parse error: {e}")));
                self.session.diagnostics.report_all(&self.session.source);
                Err(CompileError::ParseErrors)
            }
        }
    }

    /// Convert an IR contract to codegen input
    fn ir_to_codegen(ir_contract: &edge_ir::IrContract) -> edge_codegen::ContractInput {
        let functions = ir_contract
            .functions
            .iter()
            .map(|ir_fn| {
                let body = ir_fn
                    .body
                    .iter()
                    .map(Self::ir_instruction_to_gen_instr)
                    .collect();
                edge_codegen::FunctionInput {
                    name: ir_fn.name.clone(),
                    selector: ir_fn.selector,
                    is_pub: ir_fn.is_pub,
                    body,
                }
            })
            .collect();

        edge_codegen::ContractInput {
            name: ir_contract.name.clone(),
            functions,
        }
    }

    /// Convert a single IR instruction to a `GenInstr`
    fn ir_instruction_to_gen_instr(instr: &edge_ir::IrInstruction) -> edge_codegen::GenInstr {
        match instr {
            edge_ir::IrInstruction::Push(bytes) => edge_codegen::GenInstr::Push(bytes.clone()),
            edge_ir::IrInstruction::Pop => edge_codegen::GenInstr::Pop,
            edge_ir::IrInstruction::Dup(n) => edge_codegen::GenInstr::Dup(*n),
            edge_ir::IrInstruction::Swap(n) => edge_codegen::GenInstr::Swap(*n),
            edge_ir::IrInstruction::Add => edge_codegen::GenInstr::Add,
            edge_ir::IrInstruction::Sub => edge_codegen::GenInstr::Sub,
            edge_ir::IrInstruction::Mul => edge_codegen::GenInstr::Mul,
            edge_ir::IrInstruction::Div => edge_codegen::GenInstr::Div,
            edge_ir::IrInstruction::Mod => edge_codegen::GenInstr::Mod,
            edge_ir::IrInstruction::Exp => edge_codegen::GenInstr::Exp,
            edge_ir::IrInstruction::Lt => edge_codegen::GenInstr::Lt,
            edge_ir::IrInstruction::Gt => edge_codegen::GenInstr::Gt,
            edge_ir::IrInstruction::Eq => edge_codegen::GenInstr::Eq,
            edge_ir::IrInstruction::IsZero => edge_codegen::GenInstr::IsZero,
            edge_ir::IrInstruction::And => edge_codegen::GenInstr::And,
            edge_ir::IrInstruction::Or => edge_codegen::GenInstr::Or,
            edge_ir::IrInstruction::Xor => edge_codegen::GenInstr::Xor,
            edge_ir::IrInstruction::Not => edge_codegen::GenInstr::Not,
            edge_ir::IrInstruction::Shl => edge_codegen::GenInstr::Shl,
            edge_ir::IrInstruction::Shr => edge_codegen::GenInstr::Shr,
            edge_ir::IrInstruction::SLoad => edge_codegen::GenInstr::SLoad,
            edge_ir::IrInstruction::SStore => edge_codegen::GenInstr::SStore,
            edge_ir::IrInstruction::MLoad => edge_codegen::GenInstr::MLoad,
            edge_ir::IrInstruction::MStore => edge_codegen::GenInstr::MStore,
            edge_ir::IrInstruction::CallDataLoad => edge_codegen::GenInstr::CallDataLoad,
            edge_ir::IrInstruction::CallDataSize => edge_codegen::GenInstr::CallDataSize,
            edge_ir::IrInstruction::Caller => edge_codegen::GenInstr::Caller,
            edge_ir::IrInstruction::CallValue => edge_codegen::GenInstr::CallValue,
            edge_ir::IrInstruction::Number => edge_codegen::GenInstr::Number,
            edge_ir::IrInstruction::Timestamp => edge_codegen::GenInstr::Timestamp,
            edge_ir::IrInstruction::Keccak256 => edge_codegen::GenInstr::Keccak256,
            edge_ir::IrInstruction::Log(n) => edge_codegen::GenInstr::Log(*n),
            edge_ir::IrInstruction::Jump => edge_codegen::GenInstr::Jump,
            edge_ir::IrInstruction::JumpI => edge_codegen::GenInstr::JumpI,
            edge_ir::IrInstruction::JumpDest(label) => {
                edge_codegen::GenInstr::JumpDest(label.clone())
            }
            edge_ir::IrInstruction::PushLabel(label) => {
                edge_codegen::GenInstr::PushLabel(label.clone())
            }
            edge_ir::IrInstruction::Return => edge_codegen::GenInstr::Return,
            edge_ir::IrInstruction::Revert => edge_codegen::GenInstr::Revert,
            edge_ir::IrInstruction::Stop => edge_codegen::GenInstr::Stop,
        }
    }

    /// Collect event declarations from the AST and compute their keccak256 signature hashes.
    fn collect_event_metas(program: &Program) -> Vec<EventMeta> {
        let mut metas = Vec::new();
        for stmt in &program.stmts {
            Self::collect_event_metas_from_stmt(stmt, &mut metas);
        }
        metas
    }

    fn collect_event_metas_from_stmt(stmt: &Stmt, metas: &mut Vec<EventMeta>) {
        match stmt {
            Stmt::EventDecl(event) => {
                let sig = Self::event_signature(&event.name.name, &event.fields);
                let sig_hash = Self::keccak256(sig.as_bytes());
                let indexed_count = event.fields.iter().filter(|f| f.indexed).count() as u8;
                metas.push(EventMeta {
                    name: event.name.name.clone(),
                    sig_hash,
                    indexed_count,
                    total_fields: event.fields.len(),
                });
            }
            Stmt::ContractDecl(contract) => {
                // Events declared inside contracts
                for fn_decl in &contract.functions {
                    for item in &fn_decl.body.stmts {
                        if let edge_ast::stmt::BlockItem::Stmt(s) = item {
                            Self::collect_event_metas_from_stmt(s, metas);
                        }
                    }
                }
            }
            Stmt::ModuleDecl(module) => {
                for s in &module.items {
                    Self::collect_event_metas_from_stmt(s, metas);
                }
            }
            _ => {}
        }
    }

    /// Build the ABI event signature string, e.g. "Transfer(address,address,uint256)".
    fn event_signature(name: &str, fields: &[EventField]) -> String {
        use edge_ast::ty::{PrimitiveType, TypeSig};
        fn type_str(ty: &TypeSig) -> String {
            match ty {
                TypeSig::Primitive(p) => match p {
                    PrimitiveType::UInt(n) => format!("uint{n}"),
                    PrimitiveType::Int(n) => format!("int{n}"),
                    PrimitiveType::FixedBytes(n) => format!("bytes{n}"),
                    PrimitiveType::Address => "address".to_string(),
                    PrimitiveType::Bool | PrimitiveType::Bit => "bool".to_string(),
                },
                TypeSig::Pointer(_, inner) => type_str(inner),
                _ => "uint256".to_string(),
            }
        }
        let params = fields
            .iter()
            .map(|f| type_str(&f.ty))
            .collect::<Vec<_>>()
            .join(",");
        format!("{name}({params})")
    }

    /// Compute keccak256 of a byte slice.
    fn keccak256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak::v256();
        hasher.update(data);
        let mut out = [0u8; 32];
        hasher.finalize(&mut out);
        out
    }

    /// Get a reference to the session
    pub const fn session(&self) -> &Session {
        &self.session
    }
}
