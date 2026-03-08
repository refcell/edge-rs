use std::path::PathBuf;

use edge_driver::{
    compiler::Compiler,
    config::{CompilerConfig, EmitKind},
};
use edge_typeck::{AbiEntry, StateMutability};

/// Helper: compile a file with `EmitKind::Abi` and return the ABI entries.
fn compile_abi(file: &str) -> Vec<AbiEntry> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(file);
    let mut config = CompilerConfig::new(path);
    config.emit = EmitKind::Abi;
    config.quiet = true;
    let mut compiler = Compiler::new(config).expect("failed to create compiler");
    let output = compiler.compile().expect("compilation failed");
    output.abi.expect("no ABI produced")
}

#[test]
fn counter_abi_has_four_functions() {
    let abi = compile_abi("examples/counter.edge");

    // Exactly 4 entries, all functions
    let functions: Vec<_> = abi
        .iter()
        .filter_map(|e| match e {
            AbiEntry::Function(f) => Some(f),
            _ => None,
        })
        .collect();
    assert_eq!(functions.len(), 4, "expected 4 function entries");

    let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"increment"), "missing increment");
    assert!(names.contains(&"decrement"), "missing decrement");
    assert!(names.contains(&"get"), "missing get");
    assert!(names.contains(&"reset"), "missing reset");

    // `get` returns a u256 output
    let get_fn = functions.iter().find(|f| f.name == "get").unwrap();
    assert_eq!(get_fn.outputs.len(), 1);
    assert_eq!(get_fn.outputs[0].ty, "uint256");
    // `get` is a view (reads state, no mutation keyword)
    assert_eq!(get_fn.state_mutability, StateMutability::View);

    // increment, decrement, reset have no `mut` keyword in counter.edge,
    // so they are also classified as view by the current parser/typeck.
    for name in &["increment", "decrement", "reset"] {
        let f = functions.iter().find(|f| f.name == *name).unwrap();
        assert_eq!(
            f.state_mutability,
            StateMutability::View,
            "{name} should be view (no mut keyword in source)"
        );
    }
}

#[test]
fn counter_abi_no_events() {
    let abi = compile_abi("examples/counter.edge");
    let event_count = abi
        .iter()
        .filter(|e| matches!(e, AbiEntry::Event(_)))
        .count();
    assert_eq!(event_count, 0, "counter should have no events");
}

#[test]
fn abi_serializes_to_valid_json() {
    let abi = compile_abi("examples/counter.edge");
    let json_str = serde_json::to_string_pretty(&abi).expect("serialization failed");
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("failed to parse JSON back");

    let arr = parsed.as_array().expect("ABI should be a JSON array");
    assert!(!arr.is_empty(), "ABI array should not be empty");

    for entry in arr {
        let ty = entry
            .get("type")
            .and_then(|v| v.as_str())
            .expect("each entry must have a \"type\" field");
        assert!(
            ty == "function" || ty == "event",
            "unexpected type: {ty}"
        );
    }
}

#[test]
fn erc20_abi_has_events() {
    let abi = compile_abi("examples/erc20.edge");

    let events: Vec<_> = abi
        .iter()
        .filter_map(|e| match e {
            AbiEntry::Event(ev) => Some(ev),
            _ => None,
        })
        .collect();
    assert!(
        !events.is_empty(),
        "erc20 ABI should contain at least one event"
    );

    // Transfer event should exist with an indexed field
    let transfer = events
        .iter()
        .find(|ev| ev.name == "Transfer")
        .expect("Transfer event not found");
    assert!(
        transfer.inputs.iter().any(|p| p.indexed),
        "Transfer event should have at least one indexed parameter"
    );

    // Approval event should also exist
    assert!(
        events.iter().any(|ev| ev.name == "Approval"),
        "Approval event not found"
    );
}
