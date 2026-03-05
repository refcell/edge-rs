//! Egglog schema for bytecode-level instruction sequences.

use edge_ir::OptimizeFor;

use super::costs::inst_gas_cost;

/// All Inst constructor names that appear in the schema (nullary ones).
const NULLARY_INSTS: &[&str] = &[
    "IPush0",
    "IAdd",
    "IMul",
    "ISub",
    "IDiv",
    "ISDiv",
    "IMod",
    "ISMod",
    "IExp",
    "IAddMod",
    "IMulMod",
    "ISignExtend",
    "ILt",
    "IGt",
    "ISLt",
    "ISGt",
    "IEq",
    "IIsZero",
    "IAnd",
    "IOr",
    "IXor",
    "INot",
    "IByte",
    "IShl",
    "IShr",
    "ISar",
    "IKeccak256",
    "IAddress",
    "IBalance",
    "IOrigin",
    "ICaller",
    "ICallValue",
    "ICallDataLoad",
    "ICallDataSize",
    "ICodeSize",
    "IGasPrice",
    "IReturnDataSize",
    "IExtCodeSize",
    "IExtCodeHash",
    "IBlockHash",
    "ICoinbase",
    "ITimestamp",
    "INumber",
    "IPrevrandao",
    "IGasLimit",
    "IChainId",
    "ISelfBalance",
    "IBaseFee",
    "IPop",
    "IMLoad",
    "IMStore",
    "IMStore8",
    "ISLoad",
    "ISStore",
    "ITLoad",
    "ITStore",
    "IMCopy",
    "ICreate",
    "ICall",
    "ICallCode",
    "IReturn",
    "IDelegateCall",
    "ICreate2",
    "IStaticCall",
    "IRevert",
    "IInvalid",
    "ISelfDestruct",
    "IGas",
    "IPc",
    "IMSize",
    "IStop",
];

/// Parameterized Inst constructor names (take i64 arg).
const PARAM_INSTS: &[&str] = &["IDup", "ISwap", "ILog"];

/// Generate the full egglog schema with cost annotations based on optimization target.
pub(crate) fn generate_schema(optimize_for: OptimizeFor) -> String {
    let mut out = String::with_capacity(4096);

    // Inst datatype
    out.push_str("(datatype Inst\n");
    for &name in NULLARY_INSTS {
        let cost = match optimize_for {
            OptimizeFor::Size => 1,
            OptimizeFor::Gas => inst_gas_cost(name),
        };
        out.push_str(&format!("  ({name} :cost {cost})\n"));
    }
    for &name in PARAM_INSTS {
        let cost = match optimize_for {
            OptimizeFor::Size => 1,
            OptimizeFor::Gas => inst_gas_cost(name),
        };
        out.push_str(&format!("  ({name} i64 :cost {cost})\n"));
    }
    out.push_str(")\n\n");

    // PushVal datatype
    let (ps_cost, ph_cost) = match optimize_for {
        OptimizeFor::Size => (2, 5),
        OptimizeFor::Gas => (3, 3),
    };
    out.push_str(&format!(
        "(datatype PushVal\n  (PushSmall i64 :cost {ps_cost})\n  (PushHex String :cost {ph_cost}))\n\n"
    ));

    // ISeq datatype
    out.push_str(
        "(datatype ISeq\n  (INil :cost 0)\n  (ICons Inst ISeq :cost 0)\n  (IPushCons PushVal ISeq :cost 0))\n\n"
    );

    // Rulesets
    out.push_str("(ruleset bytecode-peepholes)\n");
    out.push_str("(ruleset bytecode-const-fold)\n");
    out.push_str("(ruleset bytecode-strength-red)\n");
    out.push_str("(ruleset bytecode-dead-push)\n");

    out
}
