//! EVM gas costs for bytecode-level instruction constructors.
//!
//! Used by the schema generator when `OptimizeFor::Gas` is selected.
//! Costs are approximate and based on the Yellow Paper / EIP gas schedules.

/// Return the gas cost for an Inst constructor name.
///
/// This is the per-opcode execution gas cost. Used as `:cost` annotation
/// in the egglog schema when optimizing for gas.
pub(crate) fn inst_gas_cost(name: &str) -> u32 {
    match name {
        // Gbase (2)
        "IPop" | "IAddress" | "IOrigin" | "ICaller" | "ICallValue" | "ICallDataSize"
        | "ICodeSize" | "IGasPrice" | "ICoinbase" | "ITimestamp" | "INumber" | "IPrevrandao"
        | "IGasLimit" | "IChainId" | "ISelfBalance" | "IBaseFee" | "IReturnDataSize" | "IPc"
        | "IMSize" | "IGas" => 2,

        // Glow (5) — mul, div, mod, signextend, clz
        "IMul" | "IDiv" | "ISDiv" | "IMod" | "ISMod" | "ISignExtend" | "IClz" => 5,

        // Medium (8)
        "IAddMod" | "IMulMod" => 8,

        // Exp (10 base + ~50 per byte)
        "IExp" => 60,

        // Keccak256 (30 + 6 per word)
        "IKeccak256" => 36,

        // Balance / ext / transient storage / system calls (warm = 100)
        "IBalance" | "IExtCodeSize" | "IExtCodeHash" | "ITLoad" | "ITStore" | "ICall"
        | "ICallCode" | "IDelegateCall" | "IStaticCall" => 100,

        // Block hash (20)
        "IBlockHash" => 20,

        // CallDataCopy / MCopy (3 base + 3 per word — approximate)
        "ICallDataCopy" | "IMCopy" => 6,

        // Storage (warm)
        "ISLoad" => 2100,
        "ISStore" | "ISelfDestruct" => 5000,

        // LOG (375 base + 375 per topic + 8 per data byte)
        "ILog" => 750, // approximate: log1

        // System ops
        "ICreate" | "ICreate2" => 32000,

        // Gzero (0)
        "IStop" | "IReturn" | "IRevert" | "IInvalid" => 0,

        // Default — Gverylow (3): arithmetic, comparison, bitwise, memory, stack
        _ => 3,
    }
}
