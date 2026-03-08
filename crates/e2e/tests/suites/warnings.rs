#![allow(missing_docs)]

//! Tests for compiler warnings (unused return values, etc.)

use edge_driver::compiler::Compiler;

/// Compile source, assert success, and check that expected warning substrings appear.
fn assert_warns(source: &str, expected_warnings: &[&str]) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_ok(),
        "Expected compilation to succeed, but it failed.\nDiagnostics:\n{}\nSource:\n{source}",
        compiler.render_diagnostics()
    );

    let messages = compiler.diagnostic_messages();
    let all_messages = messages.join("\n");
    for exp in expected_warnings {
        assert!(
            all_messages.contains(exp),
            "Expected warning containing '{exp}', got:\n{all_messages}\nSource:\n{source}"
        );
    }
}

/// Compile source, assert success, and check that NO warnings are emitted.
fn assert_no_warns(source: &str) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_ok(),
        "Expected compilation to succeed, but it failed.\nDiagnostics:\n{}\nSource:\n{source}",
        compiler.render_diagnostics()
    );

    let messages = compiler.diagnostic_messages();
    assert!(
        messages.is_empty(),
        "Expected no warnings, but got:\n{}\nSource:\n{source}",
        messages.join("\n")
    );
}

// =============================================================================
// Unused return value warnings
// =============================================================================

#[test]
fn warn_unused_return_value_free_fn() {
    assert_warns(
        r#"
fn helper(x: u256) -> (u256) {
    return x + 1;
}

contract C {
    pub fn f() -> (u256) {
        let a: u256 = 10;
        helper(a);
        return a;
    }
}
"#,
        &["unused return value", "helper"],
    );
}

#[test]
fn warn_unused_return_value_method_call() {
    assert_warns(
        r#"
type Wrapper = { val: u256 };

impl Wrapper {
    fn get_val(self) -> (u256) {
        return self.val;
    }
}

contract C {
    pub fn f() -> (u256) {
        let w: Wrapper = Wrapper { val: 42 };
        w.get_val();
        return 0;
    }
}
"#,
        &["unused return value", "get_val"],
    );
}

#[test]
fn no_warn_when_return_used() {
    assert_no_warns(
        r#"
fn helper(x: u256) -> (u256) {
    return x + 1;
}

contract C {
    pub fn f() -> (u256) {
        let a: u256 = helper(10);
        return a;
    }
}
"#,
    );
}

#[test]
fn no_warn_void_function() {
    assert_no_warns(
        r#"
fn do_nothing(x: u256) {
    let _: u256 = x;
}

contract C {
    pub fn f() -> (u256) {
        do_nothing(5);
        return 0;
    }
}
"#,
    );
}
