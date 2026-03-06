//! Contract bytecode generation.
//!
//! Generates the two-part EVM bytecode for a contract:
//! 1. **Constructor** (init code): Runs once during deployment, copies
//!    the runtime bytecode to memory and returns it.
//! 2. **Runtime**: The persistent bytecode that handles function calls
//!    via the dispatcher.

use edge_ir::{schema::EvmContract, var_opt, OptimizeFor};

use crate::{
    assembler::Assembler, bytecode_opt, dispatcher, expr_compiler::ExprCompiler, opcode::Opcode,
    CodegenError,
};

/// Generate complete deployment bytecode for a contract.
///
/// Returns the constructor bytecode which, when executed, deploys the
/// runtime bytecode.
pub fn generate_contract_bytecode(
    contract: &EvmContract,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<Vec<u8>, CodegenError> {
    // 1. Generate runtime bytecode
    let runtime_bytecode = generate_runtime_bytecode(contract, optimization_level, optimize_for)?;

    // 2. Generate constructor that deploys the runtime
    generate_constructor(
        contract,
        &runtime_bytecode,
        optimization_level,
        optimize_for,
    )
}

/// Generate the constructor (init code) that deploys the runtime.
fn generate_constructor(
    contract: &EvmContract,
    runtime: &[u8],
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<Vec<u8>, CodegenError> {
    let mut asm = Assembler::new();

    // Execute constructor body (storage initializations, etc.)
    let allocations = var_opt::analyze_allocations(&contract.constructor);
    let mut compiler =
        ExprCompiler::with_allocations_and_base(&mut asm, allocations, contract.memory_high_water);
    compiler.collect_fn_info(&contract.constructor);
    compiler.compile_expr(&contract.constructor);
    compiler.emit_overflow_revert_trampoline();

    // Optimize constructor body
    let instructions = asm.take_instructions();
    let optimized = bytecode_opt::optimize(instructions, optimization_level, optimize_for)?;
    let asm = Assembler::from_instructions(optimized);

    // Deploy: copy runtime code to memory and RETURN it
    //
    // The runtime code is appended after the constructor bytecode.
    // We need to:
    // 1. PUSH runtime_size
    // 2. DUP1
    // 3. PUSH runtime_offset (= constructor_size)
    // 4. PUSH 0 (memory destination)
    // 5. CODECOPY
    // 6. PUSH 0
    // 7. RETURN

    let runtime_size = runtime.len();

    // First, assemble what we have so far to know the constructor size
    let constructor_prefix = asm.assemble();

    // Calculate the total size of the deployment code:
    // prefix + deploy_sequence_size
    // Deploy sequence: PUSH runtime_size + DUP1 + PUSH runtime_offset + PUSH0 + CODECOPY + PUSH0 + RETURN
    // Size depends on how many bytes needed for runtime_size and runtime_offset
    // runtime_offset = constructor_prefix.len() + deploy_sequence_size
    // This is a bit circular, so we calculate iteratively

    // Conservative estimate: the deploy sequence is at most ~20 bytes
    // Let's compute it precisely
    let deploy_seq_size = estimate_deploy_sequence_size(runtime_size, constructor_prefix.len());
    let runtime_offset = constructor_prefix.len() + deploy_seq_size;

    let mut deploy_asm = Assembler::new();

    // PUSH runtime_size
    deploy_asm.emit_push_usize(runtime_size);
    // DUP1 (for RETURN later)
    deploy_asm.emit_op(Opcode::Dup1);
    // PUSH runtime_offset
    deploy_asm.emit_push_usize(runtime_offset);
    // PUSH 0 (memory offset)
    deploy_asm.emit_op(Opcode::Push0);
    // CODECOPY
    deploy_asm.emit_op(Opcode::CodeCopy);
    // PUSH 0 (memory offset for RETURN)
    deploy_asm.emit_op(Opcode::Push0);
    // RETURN
    deploy_asm.emit_op(Opcode::Return);

    let deploy_bytecode = deploy_asm.assemble();

    // Combine: constructor_prefix + deploy_sequence + runtime
    let mut result = constructor_prefix;
    result.extend_from_slice(&deploy_bytecode);
    result.extend_from_slice(runtime);
    Ok(result)
}

/// Generate the runtime bytecode (dispatcher + function bodies).
fn generate_runtime_bytecode(
    contract: &EvmContract,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<Vec<u8>, CodegenError> {
    let mut asm = Assembler::new();

    // NOTE: We don't emit the Solidity free memory pointer (PUSH 0x80,
    // PUSH 0x40, MSTORE). Our codegen uses fixed memory offsets — no code
    // reads from 0x40. This saves 6 bytes + 9 gas per call.

    // Function dispatcher
    dispatcher::generate_dispatcher(&mut asm, contract);

    // 4. Function bodies are compiled inline within the dispatcher's
    //    call targets. For now, the dispatcher already contains the
    //    function call IR which will be compiled.

    // 5. Optimize runtime bytecode
    let instructions = asm.take_instructions();
    let optimized = bytecode_opt::optimize(instructions, optimization_level, optimize_for)?;

    // 6. Extract repeated sequences into subroutines (size mode only).
    // Subroutine extraction trades ~30 gas per call for significant code size
    // reduction. Only applied when optimizing for size.
    let optimized = if optimize_for == OptimizeFor::Size {
        crate::subroutine_extract::extract_subroutines(optimized)
    } else {
        optimized
    };
    let asm = Assembler::from_instructions(optimized);

    Ok(asm.assemble())
}

/// Generate assembly instructions for both constructor and runtime.
///
/// Returns post-optimization instruction lists (no final bytecode assembly).
pub fn generate_contract_asm(
    contract: &EvmContract,
    optimization_level: u8,
    optimize_for: OptimizeFor,
) -> Result<crate::AsmOutput, CodegenError> {
    // Constructor body
    let mut asm = Assembler::new();
    let allocations = var_opt::analyze_allocations(&contract.constructor);
    let mut compiler =
        ExprCompiler::with_allocations_and_base(&mut asm, allocations, contract.memory_high_water);
    compiler.collect_fn_info(&contract.constructor);
    compiler.compile_expr(&contract.constructor);
    compiler.emit_overflow_revert_trampoline();
    let instructions = asm.take_instructions();
    let constructor = bytecode_opt::optimize(instructions, optimization_level, optimize_for)?;

    // Runtime
    let mut asm = Assembler::new();
    dispatcher::generate_dispatcher(&mut asm, contract);
    let instructions = asm.take_instructions();
    let mut runtime = bytecode_opt::optimize(instructions, optimization_level, optimize_for)?;
    if optimize_for == OptimizeFor::Size {
        runtime = crate::subroutine_extract::extract_subroutines(runtime);
    }

    Ok(crate::AsmOutput {
        constructor,
        runtime,
    })
}

/// Estimate the byte size of the deploy sequence.
fn estimate_deploy_sequence_size(runtime_size: usize, prefix_size: usize) -> usize {
    let size_bytes = minimal_byte_count(runtime_size);
    // Rough estimate of offset bytes (prefix + deploy sequence itself)
    let offset_estimate = prefix_size + 20; // conservative
    let offset_bytes = minimal_byte_count(offset_estimate);

    // PUSH runtime_size: 1 + size_bytes
    // DUP1: 1
    // PUSH runtime_offset: 1 + offset_bytes
    // PUSH0: 1
    // CODECOPY: 1
    // PUSH0: 1
    // RETURN: 1
    (1 + size_bytes) + 1 + (1 + offset_bytes) + 1 + 1 + 1 + 1
}

/// How many bytes needed to represent a usize value.
fn minimal_byte_count(val: usize) -> usize {
    if val == 0 {
        return 0; // PUSH0
    }
    let bytes = val.to_be_bytes();
    let start = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    bytes.len() - start
}
