#![allow(missing_docs)]

//! Negative tests for generics — things that should NOT compile.
//!
//! Each test compiles a .edge snippet from memory, asserts failure,
//! and checks that the diagnostic messages contain expected substrings.

use edge_driver::compiler::Compiler;

fn assert_compile_error(source: &str, expected: &[&str]) {
    let mut compiler = Compiler::from_source(source);
    let result = compiler.compile();
    assert!(
        result.is_err(),
        "Expected compilation to fail, but it succeeded.\nSource:\n{source}"
    );
    let messages = compiler.diagnostic_messages();
    let all_messages = messages.join("\n");
    for exp in expected {
        assert!(
            all_messages.contains(exp),
            "Expected error containing '{exp}', got:\n{all_messages}\nSource:\n{source}"
        );
    }
}

// =============================================================================
// Wrong number of turbofish type args
// =============================================================================

#[test]
fn turbofish_too_many_type_args() {
    assert_compile_error(
        r#"
fn identity<T>(x: T) -> (T) { return x; }
contract C {
    pub fn f() -> (u256) { return identity::<u256, bool>(42); }
}
"#,
        &["wrong number of type arguments", "expected 1, found 2"],
    );
}

#[test]
fn turbofish_too_few_type_args() {
    assert_compile_error(
        r#"
fn pair<A, B>(a: A, b: B) -> (A) { return a; }
contract C {
    pub fn f() -> (u256) { return pair::<u256>(1, 2); }
}
"#,
        &["wrong number of type arguments", "expected 2, found 1"],
    );
}

// =============================================================================
// Cannot infer type params (no args, no turbofish, no type hint)
// =============================================================================

#[test]
fn cannot_infer_type_params_no_args() {
    assert_compile_error(
        r#"
fn make_default<T>() -> (T) { return 0; }
contract C {
    pub fn f() -> (u256) { return make_default(); }
}
"#,
        &["cannot infer type"],
    );
}

// =============================================================================
// Missing trait method in impl
// =============================================================================

#[test]
fn missing_trait_method() {
    assert_compile_error(
        r#"
type MyType = { val: u256 };
trait Foo {
    fn bar(x: MyType) -> (u256);
    fn baz(x: MyType) -> (u256);
}
impl MyType: Foo {
    fn bar(x: MyType) -> (u256) { return x.val; }
}
contract C {
    pub fn f() -> (u256) { return 0; }
}
"#,
        &["not all trait items implemented", "baz"],
    );
}

// =============================================================================
// Operator overload without trait impl
// =============================================================================

#[test]
fn operator_overload_no_impl() {
    assert_compile_error(
        r#"
type Wrapper = { val: u256 };
contract C {
    pub fn f() -> (u256) {
        let a: Wrapper;
        a = Wrapper { val: 1 };
        let b: Wrapper;
        b = Wrapper { val: 2 };
        let c: Wrapper;
        c = a + b;
        return c.val;
    }
}
"#,
        &["cannot apply operator", "Wrapper"],
    );
}

// =============================================================================
// Method call on type with no impl block
// =============================================================================

#[test]
fn method_call_no_impl() {
    assert_compile_error(
        r#"
type Point = { x: u256, y: u256 };
contract C {
    pub fn f() -> (u256) {
        let p: Point;
        p = Point { x: 1, y: 2 };
        return p.sum();
    }
}
"#,
        &["no method named", "sum", "Point"],
    );
}

// =============================================================================
// Calling non-existent method on type with impl block
// =============================================================================

#[test]
fn method_not_found_in_impl() {
    assert_compile_error(
        r#"
type Point = { x: u256, y: u256 };
impl Point {
    fn sum(self: Point) -> (u256) { return self.x + self.y; }
}
contract C {
    pub fn f() -> (u256) {
        let p: Point;
        p = Point { x: 1, y: 2 };
        return p.nonexistent();
    }
}
"#,
        &["no method named", "nonexistent", "Point"],
    );
}

// =============================================================================
// Generic type used with wrong number of type params
// =============================================================================

#[test]
fn generic_type_wrong_param_count() {
    assert_compile_error(
        r#"
type Pair<A, B> = { first: A, second: B };
contract C {
    pub fn f() -> (u256) {
        let p: Pair<u256>;
        return 0;
    }
}
"#,
        &["type arguments"],
    );
}
