//! EVM gas-cost extraction model for egglog optimizer.
//!
//! Annotates the egglog schema with `:cost N` on every datatype variant and
//! constructor so that the `TreeAdditiveCostModel` extractor picks the cheapest
//! equivalent program.

use std::collections::HashMap;

/// What metric the egglog extractor should minimize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizeFor {
    /// Minimize estimated EVM execution gas.
    #[default]
    Gas,
    /// Minimize code size (all nodes cost 1).
    Size,
}

/// Return the schema text with `:cost` annotations inserted on every datatype
/// variant and constructor line.
pub fn schema_with_costs(base: &str, optimize_for: OptimizeFor) -> String {
    let gas_costs = gas_cost_table();
    let mut out = String::with_capacity(base.len() + 1024);
    // Tracks paren depth inside a `(datatype ...)` block. 0 = outside any block.
    let mut dt_depth: i32 = 0;

    for line in base.lines() {
        let trimmed = line.trim();
        // Strip comments before counting parens (comments can contain parens).
        let code = strip_comment(trimmed);

        if dt_depth == 0 && trimmed.starts_with("(datatype ") {
            // Opening line of a datatype block.
            // Split into "(datatype Name" prefix and the rest (variants).
            let after_kw = &trimmed["(datatype ".len()..];
            let name_end = after_kw
                .find(|c: char| c.is_whitespace() || c == ')')
                .unwrap_or(after_kw.len());
            let type_name = &after_kw[..name_end];
            let rest = &after_kw[name_end..];

            let leading = leading_whitespace(line);
            out.push_str(leading);
            out.push_str("(datatype ");
            out.push_str(type_name);

            let rest_trimmed = rest.trim();
            if rest_trimmed.is_empty() || rest_trimmed == ")" {
                // No variants on this line (e.g. `(datatype EvmExpr)`)
                out.push_str(rest);
            } else {
                // Variants follow the name on the same line
                out.push_str(&annotate_variants(rest, optimize_for, &gas_costs));
            }

            dt_depth = paren_depth_delta(code);
        } else if dt_depth > 0 {
            // Continuation line inside a multi-line datatype block.
            if trimmed.starts_with(";;") || trimmed.is_empty() {
                out.push_str(line);
            } else {
                let leading = leading_whitespace(line);
                out.push_str(leading);
                out.push_str(&annotate_variants(trimmed, optimize_for, &gas_costs));
            }
            dt_depth += paren_depth_delta(code);
        } else if trimmed.starts_with("(constructor ") {
            out.push_str(&annotate_constructor(line, optimize_for, &gas_costs));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a map from variant/constructor name → gas cost.
fn gas_cost_table() -> HashMap<&'static str, u32> {
    let mut m = HashMap::new();

    // -- EvmBinaryOp variants --
    // Cheap arithmetic (Wverylow = 3)
    for op in &[
        "OpAdd", "OpSub", "OpLt", "OpGt", "OpSLt", "OpSGt", "OpEq", "OpAnd", "OpOr", "OpXor",
        "OpShl", "OpShr", "OpSar", "OpByte",
    ] {
        m.insert(*op, 3);
    }
    // Expensive arithmetic (Wlow = 5)
    for op in &["OpMul", "OpDiv", "OpSDiv", "OpMod", "OpSMod"] {
        m.insert(*op, 5);
    }
    m.insert("OpExp", 60); // 10 + ~50 per byte
                           // Checked arithmetic: higher cost than unchecked to prefer elision
    m.insert("OpCheckedAdd", 20);
    m.insert("OpCheckedSub", 20);
    m.insert("OpCheckedMul", 30);
    m.insert("OpKeccak256", 36); // 30 + 6*1word
    m.insert("OpLogAnd", 6); // ~2× ISZERO+JUMPI
    m.insert("OpLogOr", 6);
    // Storage / memory / calldata
    m.insert("OpSLoad", 2100); // Warm SLOAD
    m.insert("OpTLoad", 100); // EIP-1153
    m.insert("OpMLoad", 3);
    m.insert("OpCalldataLoad", 3);

    // -- EvmUnaryOp variants --
    m.insert("OpIsZero", 3);
    m.insert("OpNot", 3);
    m.insert("OpNeg", 3);
    m.insert("OpSignExtend", 5);
    m.insert("OpClz", 5);

    // -- EvmTernaryOp variants --
    m.insert("OpSStore", 5000);
    m.insert("OpTStore", 100);
    m.insert("OpMStore", 3);
    m.insert("OpMStore8", 3);
    m.insert("OpSelect", 10);
    m.insert("OpCalldataCopy", 9); // 3 base + 3*words (typically 1-2 words)

    // -- EvmEnvOp variants --
    for op in &[
        "EnvCaller",
        "EnvCallValue",
        "EnvCallDataSize",
        "EnvOrigin",
        "EnvGasPrice",
        "EnvCoinbase",
        "EnvTimestamp",
        "EnvNumber",
        "EnvGasLimit",
        "EnvChainId",
        "EnvSelfBalance",
        "EnvBaseFee",
        "EnvGas",
        "EnvAddress",
        "EnvReturnDataSize",
    ] {
        m.insert(*op, 2);
    }
    m.insert("EnvBlockHash", 20);
    m.insert("EnvBalance", 100);
    m.insert("EnvCodeSize", 100);

    // -- EvmConstant variants --
    m.insert("SmallInt", 3); // PUSH
    m.insert("LargeInt", 3);
    m.insert("ConstBool", 3);
    m.insert("ConstAddr", 3);

    // -- DataLocation variants --
    for loc in &[
        "LocStorage",
        "LocTransient",
        "LocMemory",
        "LocCalldata",
        "LocReturndata",
        "LocStack",
    ] {
        m.insert(*loc, 0);
    }

    // -- EvmBaseType / EvmType variants --
    for ty in &[
        "UIntT", "IntT", "BytesT", "AddrT", "BoolT", "UnitT", "StateT", "Base", "TupleT",
    ] {
        m.insert(*ty, 0);
    }

    // -- EvmContext variants --
    for ctx in &["InFunction", "InBranch", "InLoop"] {
        m.insert(*ctx, 0);
    }

    // -- ListExpr variants --
    m.insert("Cons", 0);
    m.insert("Nil", 0);

    // -- Constructors --
    m.insert("Bop", 0); // Wrapper — cost on op child
    m.insert("Uop", 0);
    m.insert("Top", 0);
    m.insert("Const", 3); // PUSH opcode
    m.insert("Selector", 3);
    m.insert("Arg", 0);
    m.insert("Empty", 0);
    m.insert("Concat", 0);
    m.insert("Get", 0);
    m.insert("If", 10);
    m.insert("DoWhile", 10);
    m.insert("Log", 375);
    m.insert("Revert", 0);
    m.insert("ReturnOp", 0);
    m.insert("ExtCall", 100);
    // Call should always lose to the inlined body during extraction.
    // The inline rule unions Call with the function body; a low Call cost
    // would make the extractor prefer the Call form over the body.
    m.insert("Call", 1_000_000);
    m.insert("LetBind", 3); // MSTORE cost
    m.insert("Var", 3); // MLOAD cost
    m.insert("VarStore", 6); // PUSH offset + MSTORE
    m.insert("Drop", 0); // No-op lifetime marker
    m.insert("Function", 0);
    m.insert("StorageField", 0);
    m.insert("Contract", 0);
    m.insert("EnvRead", 0); // Cost on the EvmEnvOp child
    m.insert("EnvRead1", 0);
    m.insert("TLNil", 0);
    m.insert("TLCons", 0);

    m
}

fn cost_for(name: &str, optimize_for: OptimizeFor, table: &HashMap<&str, u32>) -> u32 {
    match optimize_for {
        OptimizeFor::Size => 1,
        OptimizeFor::Gas => table.get(name).copied().unwrap_or(1),
    }
}

/// Annotate all `(VariantName ...)` occurrences in a string fragment.
///
/// Turns `(OpAdd)` into `(OpAdd :cost 3)`. Each top-level parenthesized
/// expression in the fragment is treated as a variant.
fn annotate_variants(text: &str, optimize_for: OptimizeFor, table: &HashMap<&str, u32>) -> String {
    let mut result = String::with_capacity(text.len() + 64);
    let mut chars = text.char_indices().peekable();

    while let Some(&(i, c)) = chars.peek() {
        if c == '(' {
            if let Some(name) = extract_variant_name(&text[i..]) {
                let cost = cost_for(name, optimize_for, table);
                if let Some(close) = find_matching_close(&text[i..]) {
                    let inner = &text[i + 1..i + close];
                    result.push('(');
                    result.push_str(inner);
                    result.push_str(&format!(" :cost {cost}"));
                    result.push(')');
                    // Advance past the closing paren
                    for _ in 0..=close {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        result.push(c);
        chars.next();
    }

    result
}

/// Annotate a `(constructor Name ...)` line: insert `:cost N` before the final `)`.
fn annotate_constructor(
    line: &str,
    optimize_for: OptimizeFor,
    table: &HashMap<&str, u32>,
) -> String {
    let trimmed = line.trim();
    let name = trimmed
        .strip_prefix("(constructor ")
        .and_then(|rest| rest.split_whitespace().next());

    let cost = name.map_or(1, |n| cost_for(n, optimize_for, table));

    line.rfind(')').map_or_else(
        || line.to_string(),
        |pos| {
            let mut result = String::with_capacity(line.len() + 16);
            result.push_str(&line[..pos]);
            result.push_str(&format!(" :cost {cost}"));
            result.push(')');
            if pos + 1 < line.len() {
                result.push_str(&line[pos + 1..]);
            }
            result
        },
    )
}

fn extract_variant_name(s: &str) -> Option<&str> {
    let rest = s.strip_prefix('(')?;
    let name = rest.split(|c: char| c.is_whitespace() || c == ')').next()?;
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn find_matching_close(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn paren_depth_delta(s: &str) -> i32 {
    let mut d = 0i32;
    for c in s.chars() {
        match c {
            '(' => d += 1,
            ')' => d -= 1,
            _ => {}
        }
    }
    d
}

fn leading_whitespace(s: &str) -> &str {
    let trimmed = s.trim_start();
    &s[..s.len() - trimmed.len()]
}

fn strip_comment(s: &str) -> &str {
    s.find(";;").map_or(s, |pos| &s[..pos])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotate_variant_line() {
        let table = gas_cost_table();
        let line = "  (OpAdd) (OpSub) (OpMul) (OpDiv) (OpSDiv) (OpMod) (OpSMod) (OpExp)";
        let result = annotate_variants(line, OptimizeFor::Gas, &table);
        assert!(result.contains("(OpAdd :cost 3)"), "got: {result}");
        assert!(result.contains("(OpMul :cost 5)"), "got: {result}");
        assert!(result.contains("(OpExp :cost 60)"), "got: {result}");
    }

    #[test]
    fn test_annotate_constructor_line() {
        let table = gas_cost_table();
        let line = "(constructor Bop (EvmBinaryOp EvmExpr EvmExpr) EvmExpr)";
        let result = annotate_constructor(line, OptimizeFor::Gas, &table);
        assert!(result.contains(":cost 0)"), "got: {result}");
    }

    #[test]
    fn test_size_mode_all_cost_1() {
        let table = gas_cost_table();
        let line = "  (OpAdd) (OpSStore)";
        let result = annotate_variants(line, OptimizeFor::Size, &table);
        assert!(result.contains("(OpAdd :cost 1)"), "got: {result}");
        assert!(result.contains("(OpSStore :cost 1)"), "got: {result}");
    }

    #[test]
    fn test_datatype_no_variants_unchanged() {
        let base = "(datatype EvmExpr)\n";
        let result = schema_with_costs(base, OptimizeFor::Gas);
        // Should NOT have :cost on the bare datatype declaration
        assert_eq!(result.trim(), "(datatype EvmExpr)");
    }

    #[test]
    fn test_datatype_single_line_with_variants() {
        let base = "(datatype ListExpr (Cons EvmExpr ListExpr) (Nil))\n";
        let result = schema_with_costs(base, OptimizeFor::Gas);
        assert!(
            result.contains("(Cons EvmExpr ListExpr :cost 0)"),
            "got: {result}"
        );
        assert!(result.contains("(Nil :cost 0)"), "got: {result}");
        // Should NOT annotate the datatype declaration itself
        assert!(result.starts_with("(datatype ListExpr"), "got: {result}");
    }

    #[test]
    fn test_schema_with_costs_parses() {
        let base = include_str!("schema.egg");
        let annotated = schema_with_costs(base, OptimizeFor::Gas);
        assert!(annotated.contains(":cost"));
        assert!(annotated.contains("EvmExpr"));
        assert!(annotated.contains("EvmBinaryOp"));
    }

    #[test]
    fn test_gas_annotated_schema_parses_egglog() {
        let base = include_str!("schema.egg");
        let annotated = schema_with_costs(base, OptimizeFor::Gas);
        let mut egraph = egglog::EGraph::default();
        let result = egraph.parse_and_run_program(None, &annotated);
        if let Err(e) = &result {
            eprintln!(
                "ANNOTATED SCHEMA (first 2000 chars):\n{}",
                &annotated[..annotated.len().min(2000)]
            );
            panic!("Gas-annotated schema failed to parse: {e}");
        }
    }

    #[test]
    fn test_size_annotated_schema_parses_egglog() {
        let base = include_str!("schema.egg");
        let annotated = schema_with_costs(base, OptimizeFor::Size);
        let mut egraph = egglog::EGraph::default();
        let result = egraph.parse_and_run_program(None, &annotated);
        if let Err(e) = &result {
            panic!("Size-annotated schema failed to parse: {e}");
        }
    }
}
