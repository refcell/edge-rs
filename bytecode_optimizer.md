Comprehensive EVM Bytecode Optimization Report for edge-rs

       Table of Contents

       1. #1-evm-memory-cost-model
       2. #2-stack-vs-memory-for-temporaries
       3. #3-calldatacopy-vs-calldataload
       4. #4-codecopy-for-data-embedding
       5. #5-returndatacopy-patterns
       6. #6-memory-zeroing-techniques
       7. #7-storage-access-patterns
       8. #8-transient-storage-eip-1153
       9. #9-real-compiler-techniques
       10. #10-dispatcher-optimization-patterns
       11. #11-advanced-stack-scheduling
       12. #12-code-size-optimizations
       13. #13-actionable-recommendations-for-edge-rs

       ---
       1. EVM Memory Cost Model

       Memory Expansion Formula (Quadratic)

       The memory cost function is:

       memory_size_word = (memory_byte_size + 31) / 32
       memory_cost = (memory_size_word^2 / 512) + (3 * memory_size_word)

       The cost you pay is the delta between the new memory cost and the previous memory cost:

       expansion_cost = C_mem(new_state) - C_mem(old_state)

       Critical Thresholds

       ┌─────────────┬───────┬────────────┬───────────────────────────────────────┐
       │ Memory Size │ Words │ Total Cost │       Marginal Cost (per word)        │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 32 bytes    │ 1     │ 3 gas      │ 3 gas/word                            │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 128 bytes   │ 4     │ 12 gas     │ 3 gas/word                            │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 724 bytes   │ 22.6  │ ~69 gas    │ ~3 gas/word (quadratic kicks in here) │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 1 KB        │ 32    │ 98 gas     │ ~3.06 gas/word                        │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 4 KB        │ 128   │ 416 gas    │ ~3.25 gas/word                        │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 32 KB       │ 1024  │ 5120 gas   │ ~5 gas/word                           │
       ├─────────────┼───────┼────────────┼───────────────────────────────────────┤
       │ 64 KB       │ 2048  │ 14336 gas  │ ~7 gas/word                           │
       └─────────────┴───────┴────────────┴───────────────────────────────────────┘

       Key insight: Below 724 bytes, memory is essentially linear at 3 gas per word. Above that, the quadratic
       term starts dominating. For edge-rs, most contracts use well under 1KB of memory, so the quadratic cost is
        rarely a concern in practice, but the compiler should still minimize peak memory usage.

       When Memory Gets Expanded vs Reused

       Memory expands whenever any opcode accesses a byte offset higher than the current high-water mark. The
       opcodes that can trigger expansion are: MLOAD, MSTORE, MSTORE8, CALLDATACOPY, CODECOPY, RETURNDATACOPY,
       EXTCODECOPY, MCOPY, LOG0-LOG4, CREATE, CREATE2, CALL, CALLCODE, DELEGATECALL, STATICCALL, RETURN, REVERT,
       and KECCAK256.

       Memory is never freed within a call frame. Once expanded, it stays expanded for the duration of the call.
       This means the compiler should plan memory layout to minimize the high-water mark.

       Base Costs for Memory Operations

       ┌─────────┬──────────┬──────────────────────────┬───────────────┐
       │ Opcode  │ Base Gas │        Additional        │ Bytecode Size │
       ├─────────┼──────────┼──────────────────────────┼───────────────┤
       │ MLOAD   │ 3        │ + memory expansion       │ 1 byte        │
       ├─────────┼──────────┼──────────────────────────┼───────────────┤
       │ MSTORE  │ 3        │ + memory expansion       │ 1 byte        │
       ├─────────┼──────────┼──────────────────────────┼───────────────┤
       │ MSTORE8 │ 3        │ + memory expansion       │ 1 byte        │
       ├─────────┼──────────┼──────────────────────────┼───────────────┤
       │ MCOPY   │ 3        │ + 3 per word + expansion │ 1 byte        │
       └─────────┴──────────┴──────────────────────────┴───────────────┘

       Solidity's Memory Layout Convention

       0x00 - 0x3f  (64 bytes)  : Scratch space (safe for temporary use between statements)
       0x40 - 0x5f  (32 bytes)  : Free memory pointer (points to next allocation)
       0x60 - 0x7f  (32 bytes)  : Zero slot (should not be written to)
       0x80+                     : User-allocated memory

       Recommendation for edge-rs: Your current LET_BIND_BASE_OFFSET of 0x80 in expr_compiler.rs follows the
       Solidity convention correctly. However, the scratch space at 0x00-0x3f is currently underutilized. Since
       edge-rs does not maintain Solidity ABI compatibility at the memory layout level, you can safely use a
       simpler memory model.

       Scratch Space Safety Rules

       The 64 bytes at 0x00-0x3f can be used freely between statements. Specifically:
       - Safe to use for KECCAK256 input data (which you already do for mapping slots)
       - Safe for temporary values that don't need to persist across function calls
       - Not safe if a sub-call might overwrite them (but within a single function, they are safe)
       - Some Solidity operations need more than 64 bytes of temporary space, but since edge-rs controls its own
       codegen, this is not a concern

       ---
       2. Stack vs Memory for Temporaries

       Gas Cost Comparison

       ┌───────────────────────┬──────────┬───────────────┬───────────────────────┐
       │       Operation       │ Gas Cost │ Bytecode Size │         Notes         │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ DUP1                  │ 3        │ 1 byte        │ Copy top of stack     │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ DUP2-DUP16            │ 3        │ 1 byte        │ Copy from depth N     │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ SWAP1-SWAP16          │ 3        │ 1 byte        │ Swap top with depth N │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ PUSH0                 │ 2        │ 1 byte        │ Push zero             │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ PUSH1 val             │ 3        │ 2 bytes       │ Push 1-byte value     │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ PUSH2 val             │ 3        │ 3 bytes       │ Push 2-byte value     │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ POP                   │ 2        │ 1 byte        │ Discard top           │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ MSTORE (no expansion) │ 3        │ 1 byte        │ Store to memory       │
       ├───────────────────────┼──────────┼───────────────┼───────────────────────┤
       │ MLOAD (no expansion)  │ 3        │ 1 byte        │ Load from memory      │
       └───────────────────────┴──────────┴───────────────┴───────────────────────┘

       Stack Round-Trip vs Memory Round-Trip

       Keeping a value via DUP (best case):
       DUP1          ;; 3 gas, 1 byte — value is now duplicated on stack
       Total: 3 gas, 1 byte

       Saving to memory and loading back (common case for LetBind):
       PUSH1 offset  ;; 3 gas, 2 bytes
       MSTORE        ;; 3 gas, 1 byte
       ... use ...
       PUSH1 offset  ;; 3 gas, 2 bytes
       MLOAD         ;; 3 gas, 1 byte
       Total: 12 gas, 6 bytes (per save+load cycle)

       This is a 4x gas penalty and 6x code size penalty for using memory instead of stack.

       Current edge-rs Problem: LetBind Always Spills to Memory

       Looking at /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/expr_compiler.rs lines 147-166, the
       LetBind implementation always stores to memory:

       EvmExpr::LetBind(name, value, body) => {
           self.compile_expr(value);
           let offset = self.next_let_offset;
           self.next_let_offset += 32;
           self.asm.emit_push_usize(offset);
           self.asm.emit_op(Opcode::MStore);
           // ...
           self.compile_expr(body);
       }

       This is the single biggest optimization opportunity in edge-rs. Every variable binding costs an
       unnecessary 12+ gas round-trip when it could potentially be a 3-gas DUP operation.

       When Stack Is Better (Always, If Depth Permits)

       Stack is strictly superior when:
       - The value is used 1-3 times within the same basic block
       - Stack depth after binding will remain <= 16
       - No intervening operations push more than ~14 values onto the stack

       Memory is required when:
       - Stack depth would exceed 16 (DUP/SWAP limit)
       - The value needs to survive across a JUMP boundary
       - The value is needed inside a loop body that might grow the stack unpredictably

       Maximum Stack Depth Considerations

       - Hard limit: 1024 items total
       - Practical limit: DUP1-DUP16 and SWAP1-SWAP16 can only reach 16 elements deep
       - Typical function: 3-8 live values at any point
       - Rule of thumb: If fewer than 12 variables are live simultaneously, keep everything on stack

       ---
       3. CALLDATACOPY vs Individual CALLDATALOAD

       Gas Cost Analysis

       Per-parameter CALLDATALOAD approach:
       PUSH1 offset     ;; 3 gas, 2 bytes
       CALLDATALOAD     ;; 3 gas, 1 byte
       Per parameter: 6 gas, 3 bytes (value ends up on stack, ready to use)

       CALLDATACOPY batch approach (to load N words into memory):
       PUSH1 N*32       ;; 3 gas, 2 bytes (size)
       PUSH1 4          ;; 3 gas, 2 bytes (calldata offset, after selector)
       PUSH1 dest       ;; 3 gas, 2 bytes (memory dest)
       CALLDATACOPY     ;; 3 + 3*N gas, 1 byte (base + per-word)
       Total: 12 + 3N gas, 7 bytes for copying N words to memory, but you still need MLOAD to access them: +6
       gas, +3 bytes per access.

       Break-Even Analysis

       ┌────────────┬────────────────────┬────────────────────┬──────────────┐
       │ Parameters │ CALLDATALOAD Total │ CALLDATACOPY Total │    Winner    │
       ├────────────┼────────────────────┼────────────────────┼──────────────┤
       │ 1          │ 6 gas              │ 21 gas             │ CALLDATALOAD │
       ├────────────┼────────────────────┼────────────────────┼──────────────┤
       │ 2          │ 12 gas             │ 30 gas             │ CALLDATALOAD │
       ├────────────┼────────────────────┼────────────────────┼──────────────┤
       │ 3          │ 18 gas             │ 39 gas             │ CALLDATALOAD │
       ├────────────┼────────────────────┼────────────────────┼──────────────┤
       │ 4          │ 24 gas             │ 48 gas             │ CALLDATALOAD │
       └────────────┴────────────────────┴────────────────────┴──────────────┘

       CALLDATALOAD is always better for discrete parameters because the values end up directly on the stack,
       ready for use, whereas CALLDATACOPY puts them in memory requiring additional MLOAD operations.

       CALLDATACOPY is only better when:
       - You need the calldata in memory anyway (e.g., for forwarding to another CALL)
       - You need to hash the calldata with KECCAK256 (which reads from memory)
       - You have dynamic-length data (bytes, string) that must be in memory

       How Solidity Handles Function Argument Decoding

       Solidity uses individual CALLDATALOAD operations for each fixed-size parameter. For example, a function
       transfer(address,uint256) generates:
       PUSH1 0x04  CALLDATALOAD  ;; load 'to' address
       PUSH1 0x24  CALLDATALOAD  ;; load 'amount'

       Dynamic types (bytes, string, arrays) use CALLDATACOPY because they need to be in memory for further
       processing.

       Recommendation for edge-rs

       Your current approach of using CalldataLoad per parameter is correct and optimal. Do not switch to
       CALLDATACOPY for fixed-size parameters.

       ---
       4. CODECOPY for Data Embedding

       Gas Cost

       CODECOPY(destOffset, offset, size):
         gas_cost = 3 + 3 * ceil(size/32) + memory_expansion_cost

       When CODECOPY Beats Individual PUSHes

       Individual PUSH approach for N bytes of constant data:
       PUSH32 first_word   ;; 3 gas, 33 bytes
       PUSH1 offset        ;; 3 gas, 2 bytes
       MSTORE              ;; 3 gas, 1 byte
       Per 32-byte word: 9 gas, 36 bytes (to get into memory)

       CODECOPY approach for N bytes:
       PUSH1 size          ;; 3 gas, 2 bytes
       PUSH1 code_offset   ;; 3 gas, 2 bytes
       PUSH1 mem_offset    ;; 3 gas, 2 bytes
       CODECOPY            ;; 3 + 3*words gas, 1 byte
       Plus the data in the code section: N bytes of code size

       ┌───────────┬───────────────────┬───────────────────┬──────────┐
       │ Data Size │   PUSH approach   │ CODECOPY approach │  Winner  │
       ├───────────┼───────────────────┼───────────────────┼──────────┤
       │ 32 bytes  │ 9 gas, 36 bytes   │ 15 gas, 39 bytes  │ PUSH     │
       ├───────────┼───────────────────┼───────────────────┼──────────┤
       │ 64 bytes  │ 18 gas, 72 bytes  │ 18 gas, 71 bytes  │ Tie      │
       ├───────────┼───────────────────┼───────────────────┼──────────┤
       │ 96 bytes  │ 27 gas, 108 bytes │ 21 gas, 103 bytes │ CODECOPY │
       ├───────────┼───────────────────┼───────────────────┼──────────┤
       │ 256 bytes │ 72 gas, 288 bytes │ 33 gas, 263 bytes │ CODECOPY │
       └───────────┴───────────────────┴───────────────────┴──────────┘

       Break-even: ~64-96 bytes of constant data (2-3 words). Above that, CODECOPY wins on both gas and code
       size.

       Solidity's Immutable Pattern

       Solidity handles immutable variables by:
       1. During constructor execution, computing the immutable value
       2. The constructor rewrites specific placeholder locations in the runtime bytecode with the computed
       values
       3. At runtime, these values appear as inline PUSH32 instructions

       This is a deploy-time code rewriting technique, not a CODECOPY pattern.

       Reading Past Contract Size for Zero Bytes

       When CODECOPY references bytes beyond the actual contract code, the EVM fills those positions with zeros.
       This can be exploited for cheap memory zeroing:

       PUSH1 size          ;; size to zero
       PUSH2 past_code     ;; offset past contract end
       PUSH1 mem_dest      ;; destination in memory
       CODECOPY            ;; copies zeros!

       Cost: 3 + 3*words + expansion gas, which is cheaper than explicit PUSH0+MSTORE loops for large regions.

       Recommendation for edge-rs

       For now, individual PUSH instructions are sufficient since edge-rs contracts are small. When you add
       support for large constant arrays or string literals, implement a CODECOPY-based data section. The
       generate_constructor in /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/contract.rs already uses
       CODECOPY correctly for deploying runtime code.

       ---
       5. RETURNDATACOPY Patterns

       Gas Cost

       Same formula as CALLDATACOPY/CODECOPY:
       gas_cost = 3 + 3 * ceil(size/32) + memory_expansion_cost

       Patterns

       Fixed-size return data (known at compile time):
       ;; After CALL/STATICCALL that returns 32 bytes
       PUSH1 0x20       ;; size = 32
       PUSH0            ;; source offset in returndata
       PUSH1 dest       ;; memory destination
       RETURNDATACOPY   ;; 9 gas + expansion
       PUSH1 dest
       MLOAD            ;; 3 gas — now value is on stack
       Total: ~18 gas to get return data onto the stack.

       Unknown-size return data (pattern for generic forwarding):
       RETURNDATASIZE   ;; 2 gas — get actual size
       PUSH0            ;; source offset
       PUSH1 dest       ;; memory destination
       RETURNDATACOPY   ;; variable cost

       Recommendation for edge-rs

       For external calls, pre-specify the return data area in the CALL arguments (retOffset, retLen) when the
       return size is known. This avoids needing RETURNDATACOPY entirely since the return data is already placed
       in memory by the CALL itself. Only use RETURNDATACOPY when the return size is genuinely unknown at compile
        time.

       ---
       6. Memory Zeroing Techniques

       Cost Comparison

       ┌────────────────────────────┬──────────────────┬───────────────┬─────────────────────────────┐
       │         Technique          │ Gas per 32 bytes │   Code size   │            Notes            │
       ├────────────────────────────┼──────────────────┼───────────────┼─────────────────────────────┤
       │ PUSH0 PUSH1 off MSTORE     │ 8 gas            │ 4 bytes       │ Simple, explicit            │
       ├────────────────────────────┼──────────────────┼───────────────┼─────────────────────────────┤
       │ CODECOPY past contract end │ 6 gas/word       │ 7 bytes setup │ Amortized for large regions │
       ├────────────────────────────┼──────────────────┼───────────────┼─────────────────────────────┤
       │ CALLDATALOAD past calldata │ 3 gas            │ 3 bytes       │ Only for stack values       │
       ├────────────────────────────┼──────────────────┼───────────────┼─────────────────────────────┤
       │ Memory starts zeroed       │ 0 gas            │ 0 bytes       │ Best: just don't write      │
       └────────────────────────────┴──────────────────┴───────────────┴─────────────────────────────┘

       Key Insight: Memory Starts Zeroed

       EVM memory is initialized to all zeros. The cheapest way to "zero" memory is to never write to it in the
       first place. If you allocate a fresh memory region (beyond the current high-water mark), it is already
       zero.

       CALLDATALOAD Past Calldata End

       A subtle optimization: CALLDATALOAD at an offset beyond the actual calldata returns zero-padded data. This
        means:
       PUSH2 0xFFFF    ;; offset way past calldata
       CALLDATALOAD    ;; returns 0 on the stack, costs only 3 gas
       This is the cheapest way to get a zero onto the stack after PUSH0 (2 gas), but PUSH0 is still better.

       Recommendation for edge-rs

       Since memory is zero-initialized, avoid explicit zeroing when allocating fresh memory. When you need to
       clear previously-written memory, use PUSH0 + MSTORE for individual words. For large zeroing (64+ bytes),
       consider CODECOPY past the contract end.

       ---
       7. Storage Access Patterns

       Complete Gas Cost Table

       ┌───────────┬─────────────────────────────────────┬──────────┬────────┐
       │ Operation │              Condition              │ Gas Cost │ Refund │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SLOAD     │ Cold (first access)                 │ 2,100    │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SLOAD     │ Warm (subsequent)                   │ 100      │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Cold + zero -> nonzero              │ 22,100   │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Cold + nonzero -> different nonzero │ 5,000    │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Cold + nonzero -> zero              │ 5,000    │ 4,800  │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Warm + zero -> nonzero              │ 20,000   │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Warm + nonzero -> different nonzero │ 2,900    │ -      │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Warm + nonzero -> zero              │ 2,900    │ 4,800  │
       ├───────────┼─────────────────────────────────────┼──────────┼────────┤
       │ SSTORE    │ Warm + same value (no-op)           │ 100      │ -      │
       └───────────┴─────────────────────────────────────┴──────────┴────────┘

       Critical Optimizations

       1. Cache SLOAD Results (Already in IR storage.egg)

       Your IR already has SLoad(slot, SStore(slot, val, state)) -> val which eliminates redundant loads after
       stores. However, the codegen should also ensure that multiple reads of the same slot within a function are
        cached:

       ;; BAD: 2100 + 100 = 2200 gas
       PUSH slot SLOAD    ;; first read (cold: 2100)
       ... use value ...
       PUSH slot SLOAD    ;; second read (warm: 100)

       ;; GOOD: 2100 + 3 = 2103 gas
       PUSH slot SLOAD    ;; first read (cold: 2100)
       DUP1               ;; duplicate on stack (3 gas)
       ... use value ...
       ;; second use already on stack

       Savings: 97 gas per cached read (100 warm SLOAD vs 3 DUP).

       2. Avoid Redundant SSTORE

       Writing the same value back costs 100 gas (warm no-op write) but this is still wasted. Your IR rule
       SStore(slot, SLoad(slot, state), state) -> state handles this.

       3. EIP-2929 Access List Optimization

       Pre-declaring storage slots in the transaction access list costs 1,900 gas per slot but reduces the first
       SLOAD from 2,100 to 100 gas, a net saving of only 100 gas per slot. This is rarely worth it for single-use
        slots, but for slots accessed in multiple transactions, the savings compound.

       Recommendation for edge-rs

       The existing IR-level storage optimizations in
       /Users/brockelmore/git_pkgs/edge-rs/crates/ir/src/optimizations/storage.egg are solid. The next step is to
        add SLOAD caching across multiple reads in the same function body, which should be done at the IR level
       by recognizing that multiple SLoad(same_slot, state) expressions where state hasn't been modified by an
       SStore to that slot can share the same result.

       ---
       8. Transient Storage (EIP-1153)

       Gas Costs

       ┌────────┬──────────┬─────────────────────────┐
       │ Opcode │ Gas Cost │          Notes          │
       ├────────┼──────────┼─────────────────────────┤
       │ TLOAD  │ 100      │ Read transient storage  │
       ├────────┼──────────┼─────────────────────────┤
       │ TSTORE │ 100      │ Write transient storage │
       └────────┴──────────┴─────────────────────────┘

       This is 21-220x cheaper than regular SSTORE, and the same cost as warm SLOAD.

       Optimization Opportunities

       Reentrancy guards: 200 gas total (TLOAD check + TSTORE set + TSTORE clear) vs ~6,900 gas with SSTORE.

       Cross-function temporary data within a transaction: Instead of writing to regular storage and paying
       5,000-22,100 gas, use transient storage for 100 gas each way.

       Transient approval patterns: ERC20 approve + transferFrom in one transaction can use transient storage for
        the approval, avoiding the 20,000 gas SSTORE cost.

       Recommendation for edge-rs

       Add transient storage support at the language level (you already have TLOAD/TSTORE in the IR). Consider
       adding a transient storage qualifier:

       // In the edge language:
       transient reentrancy_lock: bool;

       This would compile to TLOAD/TSTORE instead of SLOAD/SSTORE, saving ~5,000+ gas per access.

       ---
       9. Real Compiler Techniques

       Solidity Yul Optimizer (26 Optimization Passes)

       The Solidity compiler's Yul optimizer performs these passes, organized by category:

       Preprocessing: Disambiguator, FunctionHoister, FunctionGrouper, ForLoopConditionIntoBody,
       ForLoopInitRewriter, VarDeclInitializer

       Pseudo-SSA: ExpressionSplitter, SSATransform, UnusedAssignEliminator

       Expression Simplification: CommonSubexpressionEliminator, ExpressionSimplifier, LiteralRematerialiser,
       LoadResolver

       Statement Simplification: CircularReferencesPruner, ConditionalSimplifier, ConditionalUnsimplifier,
       ControlFlowSimplifier, DeadCodeEliminator, EqualStoreEliminator, UnusedPruner, StructuralSimplifier,
       BlockFlattener, LoopInvariantCodeMotion

       Function-Level: FunctionSpecializer, UnusedFunctionParameterPruner, UnusedStoreEliminator,
       EquivalentFunctionCombiner

       Function Inlining: ExpressionInliner, FullInliner

       Cleanup: ExpressionJoiner, SSAReverser, StackCompressor, Rematerialiser, ForLoopConditionOutOfBody

       Key for edge-rs: The LoadResolver pass (which replaces sload(x) and mload(x) with known values) and
       EqualStoreEliminator (which removes redundant stores) are particularly relevant. edge-rs already has
       equivalent rules in its egglog IR optimization, which is a strength of the equality saturation approach.

       The --optimize-runs parameter: Controls the trade-off between deployment cost and runtime cost. A value of
        1 produces small code; a value of 10000+ produces fast code. edge-rs could implement something similar.

       Vyper's Venom IR

       Vyper's new Venom IR draws from LLVM design:
       - SSA form for sophisticated analysis
       - CSE elimination pass
       - Dead-store elimination
       - Moving calling convention items to the stack
       - Benchmark contracts are typically 5% smaller than previous output
       - Vyper allocates memory at compile time (no free memory pointer) because it forbids dynamic memory types

       Key insight for edge-rs: Vyper's approach of compile-time memory allocation is viable because edge-rs,
       like Vyper, could choose to not support dynamic-length memory types. This eliminates the free memory
       pointer overhead entirely.

       Huff's Approach

       Huff is essentially a macro assembler with no optimization passes. It relies on the programmer to write
       optimal bytecode. Relevant techniques:
       - Jump tables: Store function destinations directly in bytecode
       - Macro inlining: Everything is inlined, no function call overhead
       - Direct stack manipulation: Programmer manages the stack explicitly

       Fe Compiler

       Fe is still in early stages but aims for memory safety with EVM targeting. Not yet mature enough to learn
       from.

       ---
       10. Dispatcher Optimization Patterns

       Current edge-rs Approach

       Looking at /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/dispatcher.rs, the dispatcher compiles
       the IR's if-else chain directly, which produces a linear scan pattern.

       Pattern Comparison

       Linear Scan (Current)

       PUSH4 selector_a   ;; 3 gas, 5 bytes
       CALLDATALOAD(0)     ;; 6 gas total
       EQ                  ;; 3 gas
       JUMPI func_a        ;; 10 gas
       PUSH4 selector_b
       ... repeat ...
       Per function checked: ~22 gas, ~12 bytes
       Average cost for N functions: ~11N gas (checks N/2 on average)

       Binary Search

       CALLDATALOAD(0)     ;; load selector once
       PUSH4 mid_selector
       GT
       JUMPI right_half
       ;; left half: recurse
       Average cost for N functions: ~22 * log2(N) gas

       Jump Table (Constant Gas)

       CALLDATALOAD(0)         ;; load selector
       SHR 0xE0               ;; extract top 4 bytes
       ;; index into table
       PUSH32 packed_table
       SWAP1
       BYTE                    ;; extract jump dest
       JUMP
       Cost: ~30-60 gas regardless of N

       When to Use Each

       ┌───────────┬───────────────┬──────────────┬───────────────────────────┐
       │ Functions │ Best Approach │ Avg Gas Cost │           Notes           │
       ├───────────┼───────────────┼──────────────┼───────────────────────────┤
       │ 1-3       │ Linear scan   │ ~22-66 gas   │ Simplest, lowest overhead │
       ├───────────┼───────────────┼──────────────┼───────────────────────────┤
       │ 4-6       │ Linear scan   │ ~44-132 gas  │ Still acceptable          │
       ├───────────┼───────────────┼──────────────┼───────────────────────────┤
       │ 7-15      │ Binary search │ ~44-88 gas   │ log2(15) = ~4 levels      │
       ├───────────┼───────────────┼──────────────┼───────────────────────────┤
       │ 16+       │ Jump table    │ ~50-60 gas   │ Constant time             │
       └───────────┴───────────────┴──────────────┴───────────────────────────┘

       Concrete Recommendation for edge-rs

       For the typical ERC20 contract (6 functions), the current linear scan costs ~66 gas average. Binary search
        would cost ~44 gas (3 levels). This saves 22 gas per call, which is meaningful but not critical.

       Priority: Medium. Implement binary search dispatch for contracts with 6+ functions. The IR-level optimizer
        could sort selectors and generate a balanced binary search tree.

       Implementation Sketch for Binary Search Dispatch

       ;; Load selector once
       PUSH0 CALLDATALOAD
       PUSH1 0xE0 SHR          ;; extract 4-byte selector

       ;; Level 1: compare with middle selector
       DUP1
       PUSH4 mid_selector
       GT
       PUSH2 right_half
       JUMPI

       ;; Left half
       DUP1 PUSH4 sel_a EQ PUSH2 func_a JUMPI
       DUP1 PUSH4 sel_b EQ PUSH2 func_b JUMPI
       PUSH0 PUSH0 REVERT      ;; no match

       ;; Right half
       JUMPDEST
       DUP1 PUSH4 sel_c EQ PUSH2 func_c JUMPI
       DUP1 PUSH4 sel_d EQ PUSH2 func_d JUMPI
       PUSH0 PUSH0 REVERT

       ---
       11. Advanced Stack Scheduling

       The Problem

       The EVM is a stack machine with only 16 elements reachable via DUP/SWAP. When compiling expressions, the
       order of evaluation determines stack layout, and suboptimal ordering leads to excessive SWAP operations.

       Key Algorithms

       1. Postorder Traversal with Stack Depth Tracking

       Your current approach in expr_compiler.rs uses postorder traversal. This is correct for simple expressions
        but doesn't minimize stack depth for complex expressions.

       2. Treegraph-Based Instruction Scheduling

       This approach builds a dependency graph of operations and schedules them to minimize stack depth:
       - Evaluate subtrees that produce values needed deepest first
       - This minimizes the maximum stack depth during evaluation
       - For a binary operation ADD(a, b) where a is more complex than b, evaluate a first (it will be deeper on
       the stack) then b

       3. Stack-to-Register Mapping

       Treat the top 16 stack positions as "registers" and apply register allocation:
       - Build interference graph of live variables
       - Use graph coloring to assign stack positions
       - Insert SWAP/DUP operations to bring values to correct positions

       When to Spill to Memory vs Keep on Stack

       Decision criteria:
       1. Liveness analysis: If a variable is live across more than ~6 intervening computations, it is at risk of
        being pushed beyond DUP16 range. Spill it.
       2. Frequency of use: If a variable is used 3+ times, keeping it on stack via DUP saves gas vs memory
       round-trips.
       3. Stack depth at point of use: If current depth + new computations > 14, spill the oldest live variable.

       Cost of Spilling

       MSTORE to spill:     PUSH offset (3 gas, 2 bytes) + MSTORE (3 gas, 1 byte) = 6 gas, 3 bytes
       MLOAD to unspill:    PUSH offset (3 gas, 2 bytes) + MLOAD (3 gas, 1 byte)  = 6 gas, 3 bytes
       Total round-trip:    12 gas, 6 bytes

       Compare with DUP at depth N: 3 gas, 1 byte.

       Recommendation for edge-rs

       The most impactful change would be to convert LetBind to use DUP instructions when the binding is used
       within the same basic block and stack depth permits. This requires:

       1. Liveness analysis: Determine when each variable is last used
       2. Stack depth tracking: Maintain a model of the stack during compilation
       3. Spill decision: Only spill to memory when stack depth would exceed 14

       This optimization alone could save 9 gas per variable access (12 gas memory round-trip vs 3 gas DUP).

       ---
       12. Code Size Optimizations

       PUSH0 (EIP-3855)

       ┌─────────────┬─────┬───────────────┐
       │ Instruction │ Gas │ Bytecode Size │
       ├─────────────┼─────┼───────────────┤
       │ PUSH0       │ 2   │ 1 byte        │
       ├─────────────┼─────┼───────────────┤
       │ PUSH1 0x00  │ 3   │ 2 bytes       │
       └─────────────┴─────┴───────────────┘

       edge-rs already uses PUSH0 correctly in compile_const (line 190-192 of expr_compiler.rs). Savings: 1 gas +
        1 byte per zero push.

       Deduplication of Common Sequences

       When the same bytecode sequence appears multiple times, it can be extracted into a subroutine:

       Inline (current):
       ;; Sequence appears 3 times, each 10 bytes
       ;; Total: 30 bytes

       Subroutine:
       ;; Subroutine: 10 bytes + JUMPDEST(1) + JUMP-back(4) = 15 bytes
       ;; Each call: PUSH2 addr(3) + JUMP(1) + ... + PUSH2 ret(3) + JUMP(1) = 8 bytes
       ;; Total: 15 + 3*8 = 39 bytes

       Break-even: Subroutines save code size only when the sequence is long (>15 bytes) and repeated many times
       (4+). For gas, each call adds 8+8+1 = 17 gas overhead (PUSH2+JUMP there, PUSH2+JUMP back, JUMPDEST).

       When Function Call Is Cheaper Than Inlining

       ┌───────────┬───────────────────┬────────────────────────────┬──────────────────────────────────────┐
       │  Metric   │ Inline (per call) │ JUMP Subroutine (per call) │              Break-even              │
       ├───────────┼───────────────────┼────────────────────────────┼──────────────────────────────────────┤
       │ Gas       │ N gas             │ N + 17 gas                 │ Never (inline always cheaper on gas) │
       ├───────────┼───────────────────┼────────────────────────────┼──────────────────────────────────────┤
       │ Code size │ N bytes           │ 8 bytes                    │ N > 8 bytes AND repeated 3+ times    │
       └───────────┴───────────────────┴────────────────────────────┴──────────────────────────────────────┘

       Conclusion: Inlining is always better for gas. Subroutines save code size when the duplicated sequence is
       >8 bytes and appears 3+ times.

       Other Code Size Optimizations

       1. Minimal PUSH encoding: Always use the smallest PUSHn that fits the value. edge-rs already does this via
        minimal_be_bytes_u64.
       2. Jump address encoding: Use PUSH1 when the contract is < 256 bytes, PUSH2 up to 65535 bytes. Currently
       edge-rs always uses PUSH2 (see assembler.rs line 36). For small contracts this wastes 1 byte per jump.
       3. Deployment cost: Each byte of deployed bytecode costs 200 gas during deployment. For a 1000-byte
       contract, that is 200,000 gas. Reducing the contract by 100 bytes saves 20,000 gas at deployment.

       ---
       13. Actionable Recommendations for edge-rs

       Ordered by impact (estimated gas savings and implementation difficulty):

       TIER 1: High Impact, Moderate Effort

       1. Stack-Based LetBind Variables (Estimated: 9 gas savings per variable access)

       Current: All LetBind variables spill to memory (12 gas per save+load).
       Proposed: Keep variables on stack using DUP when depth permits.

       Implementation:
       - Track stack depth during compilation in ExprCompiler
       - For LetBind, if the variable is used only 1-2 times and stack depth < 14, keep on stack
       - Use DUP to copy values when needed, SWAP to position them
       - Fall back to memory spill only when stack depth would exceed limits

       Gas calculation: A typical function has 3-5 LetBind variables, each accessed 1-2 times. Savings: 5 vars *
       2 accesses * 9 gas = 90 gas per function call.

       Files to modify: /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/expr_compiler.rs

       2. Scratch Space Memory Layout (Estimated: 3-6 gas per MSTORE/MLOAD)

       Current: LetBind uses offsets starting at 0x80, potentially triggering memory expansion.
       Proposed: Use 0x00-0x3f scratch space for the first 2 temporary variables, avoiding the MSTORE to 0x40 for
        the free memory pointer setup.

       If you go with stack-based LetBind (recommendation 1), this becomes less important. But for the cases that
        do spill to memory, using lower addresses avoids memory expansion costs.

       3. SLOAD Result Caching (Estimated: 97 gas per cached read)

       Current: The IR optimizes load-after-store, but multiple reads of the same slot without intervening writes
        still generate multiple SLOAD ops.
       Proposed: Add an IR rule to recognize SLoad(slot, state) where state has not been modified by any SStore
       to slot, and replace with a cached value.

       Files to modify: /Users/brockelmore/git_pkgs/edge-rs/crates/ir/src/optimizations/storage.egg

       TIER 2: Medium Impact, Low-Medium Effort

       4. Binary Search Dispatch (Estimated: 10-30 gas per call for 6+ functions)

       Current: Linear if-else chain.
       Proposed: Sort selectors and emit a binary search tree. For contracts with 7+ functions, this saves ~10-30
        gas per call.

       Implementation: Modify the IR lowering to produce a balanced binary search tree instead of a linear
       if-else chain when the number of external functions exceeds 6.

       Files to modify: /Users/brockelmore/git_pkgs/edge-rs/crates/ir/src/to_egglog.rs (dispatcher generation),
       /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/dispatcher.rs

       5. PUSH1 for Short Jump Addresses (Estimated: 1 byte per jump for small contracts)

       Current: All jump addresses use PUSH2 (3 bytes total).
       Proposed: For contracts under 256 bytes of runtime bytecode, use PUSH1 (2 bytes total).

       Implementation: Two-pass assembly where the first pass determines if any label exceeds 255. If not, use
       PUSH1 for all jumps.

       Files to modify: /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/assembler.rs

       6. Eliminate Free Memory Pointer for Simple Contracts (Estimated: 9 gas at entry)

       Current: Always emits PUSH1 0x80 PUSH1 0x40 MSTORE at runtime start (line 119-121 of contract.rs).
       Proposed: If the contract doesn't use dynamic memory allocation (no arrays, no ABI encoding of dynamic
       types), skip the free memory pointer setup entirely. Use a simpler memory model where all offsets are
       compile-time known.

       Files to modify: /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/contract.rs

       TIER 3: Lower Impact or Longer-Term

       7. Strength Reduction for MUL/DIV by Powers of 2 (Already Partially Implemented)

       Your bytecode optimizer already converts MUL 2 -> SHL 1 etc. in
       /Users/brockelmore/git_pkgs/edge-rs/crates/codegen/src/bytecode_opt/rules.rs. The savings are: MUL costs 5
        gas, SHL costs 3 gas, saving 2 gas per operation.

       Extension: Add rules for PUSH 2^N MOD -> PUSH (2^N - 1) AND (MOD costs 5 gas, AND costs 3 gas).

       8. MCOPY Usage (EIP-5656)

       For memory-to-memory copies (e.g., when building calldata for external calls), MCOPY is ~70% cheaper than
       MLOAD+MSTORE loops. Add MCOPY support when handling external call argument encoding.

       9. Dead Code Elimination After RETURN/REVERT (Already Implemented)

       Your bytecode optimizer already handles this in eliminate_dead_code (line 94 of bytecode_opt/mod.rs). This
        is correct and complete.

       10. Compile-Time Memory Allocation (Vyper-Style)

       Since edge-rs controls the full pipeline, you can allocate all memory at compile time:
       - Compute the maximum memory needed for each function
       - Assign non-overlapping memory regions to variables whose lifetimes don't overlap
       - Reuse memory regions for variables whose lifetimes are disjoint

       This minimizes the memory high-water mark and avoids quadratic expansion costs.

       11. Label Aliasing and Dead JUMPDEST Elimination

       The remove_consecutive_labels function (line 142 of bytecode_opt/mod.rs) is currently a no-op.
       Implementing proper label aliasing would save 1 byte per redundant JUMPDEST.

       12. Transient Storage Language Support

       Add a transient keyword for storage variables that are only needed within a single transaction. This would
        compile to TLOAD/TSTORE (100 gas each) instead of SLOAD/SSTORE (2100+/5000+ gas). The IR already supports
        this.

       ---
       Complete Gas Cost Reference Table

       For quick reference during implementation:

       Category              Opcode          Gas    Bytes   Notes
       ─────────────────────────────────────────────────────────────
       Stack: Push           PUSH0           2      1       EIP-3855
                             PUSH1-PUSH32    3      2-33    PUSHn = n+1 bytes
                             DUP1-DUP16      3      1
                             SWAP1-SWAP16    3      1
                             POP             2      1

       Arithmetic            ADD/SUB         3      1       Gverylow
                             MUL/DIV/MOD     5      1       Glow
                             EXP             10+50/byte  1   Dynamic
                             SHL/SHR/SAR     3      1       Gverylow (cheaper than MUL/DIV!)

       Comparison            LT/GT/EQ        3      1
                             ISZERO          3      1

       Bitwise               AND/OR/XOR/NOT  3      1

       Memory                MLOAD           3+exp  1       + memory expansion
                             MSTORE          3+exp  1       + memory expansion
                             MCOPY           3+3/w+exp 1    EIP-5656

       Storage               SLOAD cold      2100   1       First access
                             SLOAD warm      100    1       Subsequent
                             SSTORE (0->N)   22100  1       Cold, zero to nonzero
                             SSTORE (N->M)   5000   1       Cold, nonzero to different
                             SSTORE (N->0)   5000   1       Cold, nonzero to zero (+4800 refund)
                             SSTORE warm no-op 100   1       Same value

       Transient             TLOAD           100    1       EIP-1153
                             TSTORE          100    1       EIP-1153

       Control Flow          JUMP            8      1
                             JUMPI           10     1
                             JUMPDEST        1      1

       Hashing               KECCAK256       30+6/w 1       + memory expansion

       Copy                  CALLDATALOAD    3      1
                             CALLDATACOPY    3+3/w  1       + memory expansion
                             CODECOPY        3+3/w  1       + memory expansion
                             RETURNDATACOPY  3+3/w  1       + memory expansion

       Logging               LOG0            375    1       + 8/byte data
                             LOG1            750    1       + 8/byte data
                             LOG2            1125   1       + 8/byte data
                             LOG3            1500   1       + 8/byte data
                             LOG4            1875   1       + 8/byte data

       Environment           CALLER          2      1
                             CALLVALUE       2      1
                             GAS             2      1

       Calls                 CALL            warm:100+9000(if value)  1
                             STATICCALL      warm:100  1
                             DELEGATECALL    warm:100  1

       Termination           RETURN          0+exp  1
                             REVERT          0+exp  1
                             STOP            0      1

       Deployment            Per byte        200    -       Code deposit cost

       ---
       Sources

       - https://www.evm.codes/
       - https://www.evm.codes/about
       - https://github.com/wolflo/evm-opcodes/blob/main/gas.md
       - https://ethereum.org/developers/docs/evm/opcodes/
       - https://docs.soliditylang.org/en/latest/internals/optimizer.html
       - https://docs.soliditylang.org/en/latest/internals/layout_in_memory.html
       - https://philogy.github.io/posts/selector-switches/
       - https://github.com/ethereum/solidity/issues/4760
       - https://rareskills.io/post/gas-optimization
       - https://eips.ethereum.org/EIPS/eip-2929
       - https://hackmd.io/@fvictorio/gas-costs-after-berlin
       - https://eips.ethereum.org/EIPS/eip-1153
       - https://www.soliditylang.org/blog/2024/01/26/transient-storage/
       -
       https://blog.blockmagnates.com/transient-storage-is-here-patterns-and-use-cases-for-eip-1153-6e74116bb586
       - https://eips.ethereum.org/EIPS/eip-7609
       - https://eips.ethereum.org/EIPS/eip-3855
       - https://eips.ethereum.org/EIPS/eip-5656
       - https://eips.ethereum.org/EIPS/eip-7923
       - https://notes.ethereum.org/@vbuterin/proposals_to_adjust_memory_gas_costs
       - https://flawsomedev.com/blog/gas-refunds-and-memory-expansion-cost
       - https://frang.io/blog/codegen-for-stack-machines/
       - https://frang.io/blog/spilling-in-evm/
       - https://users.ece.cmu.edu/~koopman/stack_compiler/stack_co.html
       - https://www.researchgate.net/publication/220369290_Treegraph-based_Instruction_Scheduling_for_Stack-base
       d_Virtual_Machines
       - https://www.certik.com/resources/blog/gas-optimization-in-ethereum-smart-contracts-10-best-practices
       - https://github.com/vyperlang/vyper/blob/master/vyper/venom/README.md
       - https://github.com/charles-cooper/venom-design
       - https://docs.huff.sh/tutorial/evm-basics/
       - https://medium.com/@jtriley15/huff-vs-yul-for-evm-smart-contracts-620d1d618197
       - https://github.com/ethereum/solidity/issues/13089
       - https://learnevm.com/chapters/fn/calldata
       - https://ethereum.org/developers/tutorials/short-abi/
       - https://rareskills.io/post/ethereum-contract-creation-code
       - https://github.com/etclabscore/evm_llvm/issues/48
       - https://www.chainsecurity.com/blog/tstore-low-gas-reentrancy
       - https://github.com/ethereum/solidity/pull/12978
