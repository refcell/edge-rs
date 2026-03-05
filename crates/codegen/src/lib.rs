//! Edge Language EVM Bytecode Code Generator.
//!
//! Converts an optimized `EvmProgram` from the IR crate into
//! EVM bytecode suitable for deployment.
//!
//! # Architecture
//!
//! ```text
//! EvmProgram
//!   -> contract::generate_contract_bytecode()
//!     -> dispatcher::generate_dispatcher()
//!     -> expr_compiler::ExprCompiler::compile_expr()
//!     -> assembler::Assembler::assemble()
//!   -> Vec<u8>  (final bytecode)
//! ```

#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]
#![allow(
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::match_same_arms,
    clippy::option_if_let_else,
    clippy::needless_return,
    clippy::unnecessary_wraps,
    clippy::branches_sharing_code,
    clippy::needless_range_loop,
    clippy::implicit_clone,
    clippy::missing_transmute_annotations,
    clippy::undocumented_unsafe_blocks
)]

pub mod assembler;
pub mod bytecode_opt;
pub mod contract;
pub mod dispatcher;
pub mod expr_compiler;
pub mod opcode;
pub mod subroutine_extract;

use edge_ir::{schema::EvmProgram, OptimizeFor};

/// Errors that can occur during code generation.
#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    /// No contracts found in the program
    #[error("no contracts found in the program")]
    NoContracts,
    /// Internal compiler error
    #[error("internal codegen error: {0}")]
    Internal(String),
}

/// Compile an IR program to EVM bytecode.
///
/// If the program contains contracts, compiles the first contract.
/// Returns the deployment bytecode (constructor + runtime).
///
/// `optimization_level` controls the bytecode peephole optimizer:
/// - 0: no bytecode optimization
/// - 1: peepholes + dead push removal
/// - 2: + constant folding + strength reduction
/// - 3+: aggressive iteration
///
/// `optimize_for` controls the cost model for extraction:
/// - `Gas`: minimize estimated EVM execution gas.
/// - `Size`: minimize bytecode byte-size.
pub fn compile(
    program: &EvmProgram,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<Vec<u8>, CodegenError> {
    if let Some(contract) = program.contracts.first() {
        tracing::info!("Compiling contract: {}", contract.name);
        let bytecode =
            contract::generate_contract_bytecode(contract, optimization_level, optimize_for)?;
        tracing::info!("Generated {} bytes of deployment bytecode", bytecode.len());
        Ok(bytecode)
    } else if !program.free_functions.is_empty() {
        // Compile free functions as a simple program (no dispatcher)
        tracing::info!(
            "Compiling {} free function(s)",
            program.free_functions.len()
        );
        let mut asm = assembler::Assembler::new();
        let mut compiler = expr_compiler::ExprCompiler::new(&mut asm);
        for func in &program.free_functions {
            compiler.compile_expr(func);
        }
        let instructions = asm.take_instructions();
        let optimized = bytecode_opt::optimize(instructions, optimization_level, optimize_for)?;
        let asm = assembler::Assembler::from_instructions(optimized);
        Ok(asm.assemble())
    } else {
        Err(CodegenError::NoContracts)
    }
}
