#![allow(missing_docs)]

use std::collections::BTreeMap;

use edge_driver::standard_json::{
    compile_standard_json, SourceFile, StandardJsonInput, StandardJsonOutput,
};

/// Helper: build a [`StandardJsonInput`] from a single source entry.
fn single_source_input(path: &str, content: &str) -> StandardJsonInput {
    let mut sources = BTreeMap::new();
    sources.insert(
        path.to_string(),
        SourceFile {
            content: Some(content.into()),
        },
    );
    StandardJsonInput {
        language: "Edge".into(),
        sources,
        settings: Default::default(),
    }
}

/// Helper: compile examples/counter.edge through standard JSON.
fn compile_counter() -> StandardJsonOutput {
    let source = std::fs::read_to_string("../../examples/counter.edge")
        .expect("failed to read counter.edge");
    let input = single_source_input("counter.edge", &source);
    compile_standard_json(input)
}

// ============================================================
// 1. Counter compiles successfully via standard JSON
// ============================================================

#[test]
fn standard_json_counter_compiles() {
    let output = compile_counter();

    assert!(
        output.errors.is_empty(),
        "expected no errors, got: {:?}",
        output.errors
    );

    assert!(
        output.contracts.contains_key("counter.edge"),
        "missing source key in contracts map"
    );

    let contracts = &output.contracts["counter.edge"];
    assert!(
        contracts.contains_key("Counter"),
        "missing Counter contract"
    );

    let counter = &contracts["Counter"];
    assert!(
        !counter.evm.bytecode.object.is_empty(),
        "bytecode object should not be empty"
    );
    assert!(
        !counter.evm.bytecode.object.starts_with("0x"),
        "bytecode object should not have 0x prefix"
    );
    assert!(!counter.abi.is_empty(), "ABI should not be empty");
}

// ============================================================
// 2. Invalid source produces an error
// ============================================================

#[test]
fn standard_json_invalid_source_produces_error() {
    let input = single_source_input("bad.edge", "not valid edge !!!@@@");
    let output = compile_standard_json(input);

    assert!(
        !output.errors.is_empty(),
        "expected at least one error for invalid source"
    );
    assert_eq!(
        output.errors[0].severity, "error",
        "first error should have severity 'error'"
    );
}

// ============================================================
// 3. Output serializes to valid JSON with expected top-level keys
// ============================================================

#[test]
fn standard_json_output_is_valid_json() {
    let output = compile_counter();
    let json_str = serde_json::to_string(&output).expect("failed to serialize output");
    let value: serde_json::Value =
        serde_json::from_str(&json_str).expect("failed to parse output JSON");

    let obj = value.as_object().expect("top-level should be an object");
    // When there are no errors, the "errors" key is skipped (skip_serializing_if),
    // so we check for "sources" and "contracts" which must be present on success.
    assert!(
        obj.contains_key("sources"),
        "JSON output missing 'sources' key"
    );
    assert!(
        obj.contains_key("contracts"),
        "JSON output missing 'contracts' key"
    );
}

// ============================================================
// 4. Bytecode has no 0x prefix and is pure hex
// ============================================================

#[test]
fn standard_json_bytecode_no_0x_prefix() {
    let output = compile_counter();
    let obj = &output.contracts["counter.edge"]["Counter"]
        .evm
        .bytecode
        .object;

    assert!(!obj.starts_with("0x"), "bytecode should not start with 0x");
    assert!(
        obj.chars().all(|c| c.is_ascii_hexdigit()),
        "bytecode should contain only hex digits, got: {obj}"
    );
}
