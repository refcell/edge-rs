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
    assert!(result.is_ok(), "parse(\"let x: u256;\") failed: {:?}", result.err());
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
    if let edge_ast::Stmt::VarDecl(ref ident, _, _) = program.stmts[0] {
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
    assert!(result.is_ok(), "parse(\"fn foo() {{}}\") failed: {:?}", result.err());
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
    // Expression statements go through parse_expr_stmt which wraps them in VarAssign
    let result = parse("1 + 2;");
    assert!(result.is_ok(), "parse(\"1 + 2;\") failed: {:?}", result.err());
    let program = result.unwrap();
    assert_eq!(program.stmts.len(), 1);

    // The expression statement is wrapped in VarAssign by the parser
    if let edge_ast::Stmt::VarAssign(ref lhs, _, _) = program.stmts[0] {
        // The lhs should be a Binary expression (1 + 2)
        assert!(
            matches!(lhs, edge_ast::Expr::Binary(..)),
            "expected Binary expression, got {lhs:?}"
        );
        if let edge_ast::Expr::Binary(_, ref op, _, _) = lhs {
            assert_eq!(*op, edge_ast::BinOp::Add);
        }
    } else {
        panic!("expected VarAssign wrapping expression, got {:?}", program.stmts[0]);
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
