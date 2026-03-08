#![allow(missing_docs)]

//! Negative tests for trait system — things that should NOT compile.

use edge_driver::compiler::Compiler;

/// Compile source, assert failure, and check both diagnostic messages
/// and rendered ariadne output against expected substrings.
fn assert_compile_error(source: &str, expected_messages: &[&str], expected_rendered: &[&str]) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_err(),
        "Expected compilation to fail, but it succeeded.\nSource:\n{source}"
    );

    let messages = compiler.diagnostic_messages();
    let all_messages = messages.join("\n");
    for exp in expected_messages {
        assert!(
            all_messages.contains(exp),
            "Expected message containing '{exp}', got:\n{all_messages}\nSource:\n{source}"
        );
    }

    let rendered = compiler.render_diagnostics();
    for exp in expected_rendered {
        assert!(
            rendered.contains(exp),
            "Expected rendered output containing '{exp}', got:\n{rendered}\nSource:\n{source}"
        );
    }
}

// =============================================================================
// Trait bound violation
// =============================================================================

#[test]
fn trait_bound_not_satisfied() {
    assert_compile_error(
        r#"
type Wrapper = { val: u256 };
type Other = { val: u256 };

trait Addable {
    fn add_vals(self, rhs: Self) -> (Self);
}

impl Wrapper: Addable {
    fn add_vals(self, rhs: Self) -> (Wrapper) {
        return Wrapper { val: self.val + rhs.val };
    }
}

fn combine<T: Addable>(a: T, b: T) -> (T) {
    return Addable::add_vals(a, b);
}

contract C {
    pub fn f() -> (u256) {
        let a: Other = Other { val: 1 };
        let b: Other = Other { val: 2 };
        let c: Other = combine::<Other>(a, b);
        return c.val;
    }
}
"#,
        &["trait bound", "Addable", "not satisfied"],
        &["not satisfied"],
    );
}

#[test]
fn trait_bound_multiple_constraints_one_missing() {
    assert_compile_error(
        r#"
type Wrapper = { val: u256 };

trait Foo {
    fn foo(self) -> (u256);
}

trait Bar {
    fn bar(self) -> (u256);
}

impl Wrapper: Foo {
    fn foo(self) -> (u256) { return self.val; }
}

fn needs_both<T: Foo & Bar>(x: T) -> (u256) {
    return Foo::foo(x);
}

contract C {
    pub fn f() -> (u256) {
        let w: Wrapper = Wrapper { val: 1 };
        return needs_both::<Wrapper>(w);
    }
}
"#,
        &["trait bound", "Bar", "not satisfied"],
        &["not satisfied"],
    );
}

// =============================================================================
// Supertrait not implemented
// =============================================================================

#[test]
fn supertrait_not_implemented() {
    assert_compile_error(
        r#"
type MyVal = { val: u256 };

trait Base {
    fn base_value(self) -> (u256);
}

trait Extended: Base {
    fn extended_value(self) -> (u256);
}

impl MyVal: Extended {
    fn extended_value(self) -> (u256) {
        return self.val * 2;
    }
}

contract C {
    pub fn f() -> (u256) { return 0; }
}
"#,
        &["requires", "Base"],
        &["Base"],
    );
}

// =============================================================================
// Trait bound on generic type
// =============================================================================

#[test]
fn generic_type_bound_not_satisfied() {
    assert_compile_error(
        r#"
trait Hashable {
    fn hash(self) -> (u256);
}

type Container<T: Hashable> = { item: T };

type Plain = { val: u256 };

contract C {
    pub fn f() -> (u256) {
        let c: Container<Plain> = Container { item: Plain { val: 1 } };
        return 0;
    }
}
"#,
        &["trait bound", "Hashable", "not satisfied"],
        &["not satisfied"],
    );
}

// =============================================================================
// Trait bound satisfied but wrong trait
// =============================================================================

#[test]
fn trait_bound_wrong_trait() {
    assert_compile_error(
        r#"
type Wrapper = { val: u256 };

trait Alpha {
    fn alpha(self) -> (u256);
}

trait Beta {
    fn beta(self) -> (u256);
}

impl Wrapper: Alpha {
    fn alpha(self) -> (u256) { return self.val; }
}

fn needs_beta<T: Beta>(x: T) -> (u256) {
    return 0;
}

contract C {
    pub fn f() -> (u256) {
        let w: Wrapper = Wrapper { val: 1 };
        return needs_beta::<Wrapper>(w);
    }
}
"#,
        &["trait bound", "Beta", "not satisfied"],
        &["does not implement `Beta`"],
    );
}

// =============================================================================
// Multiple supertraits, one missing
// =============================================================================

#[test]
fn supertrait_chain_missing() {
    assert_compile_error(
        r#"
type MyVal = { val: u256 };

trait A {
    fn a(self) -> (u256);
}

trait B {
    fn b(self) -> (u256);
}

trait C: A & B {
    fn c(self) -> (u256);
}

impl MyVal: A {
    fn a(self) -> (u256) { return self.val; }
}

impl MyVal: C {
    fn c(self) -> (u256) { return self.val; }
}

contract D {
    pub fn f() -> (u256) { return 0; }
}
"#,
        &["requires", "B"],
        &["B"],
    );
}

// =============================================================================
// Default method: missing required method (not default)
// =============================================================================

#[test]
fn default_method_still_requires_non_default() {
    assert_compile_error(
        r#"
type Wrapper = { val: u256 };

trait HasDefault {
    fn required(self) -> (u256);
    fn optional(self) -> (u256) {
        return 0;
    }
}

impl Wrapper: HasDefault {
    // Missing 'required' — should error even though 'optional' has a default
}

contract C {
    pub fn f() -> (u256) { return 0; }
}
"#,
        &["not all trait items implemented", "required"],
        &["missing `required`"],
    );
}
