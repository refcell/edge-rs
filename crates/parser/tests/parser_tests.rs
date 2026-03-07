//! Integration tests for the Edge language parser.

use edge_parser::parse;

// ─── Empty Program ──────────────────────────────────────────────────

#[test]
fn parse_empty_source() {
    let result = parse("");
    assert!(result.is_ok(), "parse(\"\") should succeed");
    let program = result.unwrap();
    assert!(
        program.stmts.is_empty(),
        "empty source should produce zero statements"
    );
}

// ─── Variable Declarations ──────────────────────────────────────────

#[test]
fn parse_single_var_decl() {
    let result = parse("let x: u256;");
    assert!(
        result.is_ok(),
        "parse(\"let x: u256;\") failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1, "expected 1 statement");
    assert!(
        matches!(program.stmts[0], edge_ast::Stmt::VarDecl(..)),
        "expected VarDecl, got {:?}",
        program.stmts[0]
    );
}

#[test]
fn parse_var_decl_has_correct_name() {
    let program = parse("let myVar: u256;").unwrap();
    if let edge_ast::Stmt::VarDecl(ref ident, _, _, _) = program.stmts[0] {
        assert_eq!(ident.name, "myVar");
    } else {
        panic!("expected VarDecl");
    }
}

#[test]
fn parse_two_var_decls() {
    let result = parse("let x: u256;\nlet y: addr;");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 2, "expected 2 statements");
    assert!(matches!(program.stmts[0], edge_ast::Stmt::VarDecl(..)));
    assert!(matches!(program.stmts[1], edge_ast::Stmt::VarDecl(..)));
}

// ─── Function Declarations ──────────────────────────────────────────

#[test]
fn parse_empty_fn() {
    let result = parse("fn foo() {}");
    assert!(
        result.is_ok(),
        "parse(\"fn foo() {{}}\") failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1, "expected 1 statement");
    assert!(
        matches!(program.stmts[0], edge_ast::Stmt::FnAssign(..)),
        "expected FnAssign, got {:?}",
        program.stmts[0]
    );
}

#[test]
fn parse_fn_has_correct_name() {
    let program = parse("fn bar() {}").unwrap();
    if let edge_ast::Stmt::FnAssign(ref decl, _) = program.stmts[0] {
        assert_eq!(decl.name.name, "bar");
    } else {
        panic!("expected FnAssign");
    }
}

#[test]
fn parse_fn_with_params() {
    let result = parse("fn add(a: u256, b: u256) {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    if let edge_ast::Stmt::FnAssign(ref decl, _) = program.stmts[0] {
        assert_eq!(decl.params.len(), 2);
        assert_eq!(decl.params[0].0.name, "a");
        assert_eq!(decl.params[1].0.name, "b");
    } else {
        panic!("expected FnAssign");
    }
}

#[test]
fn parse_fn_with_return_type() {
    // Note: the parser currently requires no whitespace before `->`.
    let result = parse("fn get()-> u256 {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    if let edge_ast::Stmt::FnAssign(ref decl, _) = program.stmts[0] {
        assert_eq!(decl.returns.len(), 1);
    } else {
        panic!("expected FnAssign");
    }
}

// ─── Binary Expressions ────────────────────────────────────────────

#[test]
fn parse_binary_add_expression() {
    // Expression statements are parsed as Stmt::Expr
    let result = parse("1 + 2;");
    assert!(
        result.is_ok(),
        "parse(\"1 + 2;\") failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);

    // The expression statement is wrapped in Expr by the parser
    if let edge_ast::Stmt::Expr(ref expr) = program.stmts[0] {
        // The expr should be a Binary expression (1 + 2)
        assert!(
            matches!(expr, edge_ast::Expr::Binary(..)),
            "expected Binary expression, got {expr:?}"
        );
        if let edge_ast::Expr::Binary(_, ref op, _, _) = expr {
            assert_eq!(*op, edge_ast::BinOp::Add);
        }
    } else {
        panic!("expected Expr, got {:?}", program.stmts[0]);
    }
}

// ─── Multiple Statements ───────────────────────────────────────────

#[test]
fn parse_mixed_statements() {
    let source = "let x: u256;\nfn foo() {}";
    let result = parse(source);
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 2);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::VarDecl(..)));
    assert!(matches!(program.stmts[1], edge_ast::Stmt::FnAssign(..)));
}

// ─── Type Assignments ──────────────────────────────────────────────

#[test]
fn parse_type_assignment() {
    let result = parse("type MyInt = u256;");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::TypeAssign(..)));
}

// ─── Const Assignments ─────────────────────────────────────────────

#[test]
fn parse_const_assignment() {
    let result = parse("const MAX: u256 = 100;");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::ConstAssign(..)));
}

// ─── If Else Chains ─────────────────────────────────────────────────

#[test]
fn parse_if_empty() {
    let result = parse("if (thing) {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], edge_ast::Stmt::IfElse(_, _)));
    if let edge_ast::Stmt::IfElse(branches, else_block) = &program.stmts[0] {
        assert_eq!(branches.len(), 1);
        assert_eq!(else_block, &Option::None);
    }
}

#[test]
fn parse_if() {
    let result = parse("if (thing) { let other_thing: u256; }");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], edge_ast::Stmt::IfElse(_, _)));
    if let edge_ast::Stmt::IfElse(branches, else_block) = &program.stmts[0] {
        assert_eq!(branches.len(), 1);
        assert_eq!(else_block, &Option::None);
    }
}

#[test]
fn parse_if_else() {
    let result = parse("if (thing) {} else {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], edge_ast::Stmt::IfElse(_, _)));
    if let edge_ast::Stmt::IfElse(branches, else_block) = &program.stmts[0] {
        assert_eq!(branches.len(), 1);
        assert!(matches!(else_block, Option::Some(_)));
    }
}

#[test]
fn parse_if_else_if() {
    let result = parse("if (thing) {} else if (other_thing) {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], edge_ast::Stmt::IfElse(_, _)));
    if let edge_ast::Stmt::IfElse(branches, else_block) = &program.stmts[0] {
        assert_eq!(branches.len(), 2);
        assert_eq!(else_block, &Option::None);
    }
}

#[test]
fn parse_if_else_if_else() {
    let result = parse("if (thing) {} else if (other_thing) {} else {}");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(&program.stmts[0], edge_ast::Stmt::IfElse(_, _)));
    if let edge_ast::Stmt::IfElse(branches, else_block) = &program.stmts[0] {
        assert_eq!(branches.len(), 2);
        assert!(matches!(else_block, Option::Some(_)));
    }
}
// ─── Return Stmts ───────────────────────────────────────────────────

#[test]
fn test_empty_return() {
    let result = parse("return;");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::Return(_, _)));
}

#[test]
fn test_return() {
    let result = parse("return thing;");
    assert!(result.is_ok(), "parse failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::Return(_, _)));

    if let edge_ast::Stmt::Return(expr, _) = &program.stmts[0] {
        assert!(expr.is_some());
    }
}

// ─── Error Cases ────────────────────────────────────────────────────

#[test]
fn parse_missing_semicolon_fails() {
    let result = parse("let x: u256");
    assert!(result.is_err(), "missing semicolon should fail");
}

#[test]
fn parse_incomplete_fn_fails() {
    let result = parse("fn foo(");
    assert!(result.is_err(), "incomplete function should fail");
}

// ─── New Top-Level Constructs ──────────────────────────────────────

#[test]
fn parse_event_declaration() {
    let source = "event Transfer(indexed from: addr, to: addr, amount: u256);";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse event declaration failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::EventDecl(..)));
    if let edge_ast::Stmt::EventDecl(ref event) = program.stmts[0] {
        assert_eq!(event.name.name, "Transfer");
        assert_eq!(event.fields.len(), 3);
        assert!(event.fields[0].indexed, "from should be indexed");
        assert!(!event.fields[1].indexed, "to should not be indexed");
    }
}

#[test]
fn parse_abi_declaration() {
    let source =
        "abi IERC20 { fn transfer(to: addr, amount: u256) -> (bool); fn approve(spender: addr, amount: u256) -> (bool); }";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse abi declaration failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::AbiDecl(..)));
    if let edge_ast::Stmt::AbiDecl(ref abi) = program.stmts[0] {
        assert_eq!(abi.name.name, "IERC20");
        assert_eq!(abi.functions.len(), 2);
        assert_eq!(abi.functions[0].name.name, "transfer");
    }
}

#[test]
fn parse_pub_fn_declaration() {
    let source = "pub fn foo() -> (u256) { return 42; }";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse pub fn declaration failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::FnAssign(..)));
    if let edge_ast::Stmt::FnAssign(ref decl, _) = program.stmts[0] {
        assert!(decl.is_pub, "function should be public");
        assert_eq!(decl.name.name, "foo");
    }
}

#[test]
fn parse_module_declaration() {
    let source = "mod tokens;";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse module declaration failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::ModuleDecl(..)));
    if let edge_ast::Stmt::ModuleDecl(ref module) = program.stmts[0] {
        assert_eq!(module.name.name, "tokens");
    }
}

#[test]
fn parse_use_import() {
    let source = "use lib::math;";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse use import failed: {:?}",
        result.err()
    );
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);
    assert!(matches!(program.stmts[0], edge_ast::Stmt::ModuleImport(..)));
    if let edge_ast::Stmt::ModuleImport(ref import) = program.stmts[0] {
        assert_eq!(import.root.name, "lib");
        assert!(import.path.is_some());
    }
}

// ─── Control Flow Statements ───────────────────────────────────────

#[test]
fn parse_break_statement() {
    // break inside a loop context - test at top level for simplicity
    // (parser doesn't enforce context, just syntax)
    let result = parse("break;");
    assert!(result.is_ok(), "parse break failed: {:?}", result.err());
    assert!(matches!(
        result.unwrap().stmts[0],
        edge_ast::Stmt::Break(..)
    ));
}

#[test]
fn parse_continue_statement() {
    let result = parse("continue;");
    assert!(result.is_ok(), "parse continue failed: {:?}", result.err());
    assert!(matches!(
        result.unwrap().stmts[0],
        edge_ast::Stmt::Continue(..)
    ));
}

// ─── Impl Block ───────────────────────────────────────────────────

#[test]
fn parse_impl_block() {
    let source = "impl Foo { fn bar() {} }";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse impl block failed: {:?}",
        result.err()
    );
    assert!(matches!(
        result.unwrap().stmts[0],
        edge_ast::Stmt::ImplBlock(..)
    ));
}

// ─── Contract with Functions ──────────────────────────────────────

#[test]
fn parse_contract_with_functions() {
    let source = "contract Counter { let count: &s u256; pub fn increment() { } }";
    let result = parse(source);
    assert!(
        result.is_ok(),
        "parse contract with functions failed: {:?}",
        result.err()
    );
    if let edge_ast::Stmt::ContractDecl(ref c) = result.unwrap().stmts[0] {
        assert_eq!(c.name.name, "Counter");
        assert_eq!(c.fields.len(), 1);
        assert_eq!(c.functions.len(), 1);
    } else {
        panic!("expected ContractDecl");
    }
}

// ─── Trait with Real Name ─────────────────────────────────────────

#[test]
fn parse_trait_with_real_name() {
    let source = "trait IOwned { fn owner() -> (addr); }";
    let result = parse(source);
    assert!(result.is_ok(), "parse trait failed: {:?}", result.err());
    if let edge_ast::Stmt::TraitDecl(ref t, _) = result.unwrap().stmts[0] {
        assert_eq!(t.name.name, "IOwned");
        assert_eq!(t.items.len(), 1);
    } else {
        panic!("expected TraitDecl");
    }
}

// ─── Match Statement ──────────────────────────────────────────────

#[test]
fn parse_match_statement() {
    let source = "match x { Foo::Bar => { } }";
    let result = parse(source);
    assert!(result.is_ok(), "parse match failed: {:?}", result.err());
    assert!(matches!(
        result.unwrap().stmts[0],
        edge_ast::Stmt::Match(..)
    ));
}
