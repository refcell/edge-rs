# Branching

Branching refers to blocks of code that may be executed based on a defined condition.

## If Else If Branch

```
<if_else_if_branch> ::= "if" "(" <expr> ")" <code_block>
    ("else" "if" "(" <expr> ")" <code_block>)*
    ["else" <code_block>] ;
```

Dependencies:

* `<expr>`
* `<code_block>`

The `<branch>` contains an "if" keyword followed by a parenthesis delimited expression
and a code block. It may be followed by zero or more conditions under "else" "if"
keywords followed by a parenthesis delimited expression and a code block, and finally
it may optionally be suffixed with an "else" keyword followed by a code block.

## If Match

```
<if_match_branch> ::= "if" <pattern_match> <code_block> ;
```

Dependencies:

* `<pattern_match>`
* `<code_block>`

The `<if_match_branch>` contains a pattern match expression followed by an optionally
typed identifier followed by a code block.

## Match

```
<match_arm> ::= (<union_pattern> | <ident> | "_") "=>" <code_block> ;

<match> ::=
    "match" <expr> "{"
    [<match_arm> ("," <match_arm>)* [","]]
    "}" ;
```

Dependencies:

* `<expr>`
* `<union_pattern>`
* `<code_block>`

The `<match_arm>` is a single arm of a match statement. It may optionally be
prefixed with a union pattern and contains a lambda.

The `<match>` statement is a group of match arms that may pattern match against
an expression.

Semantics of the match statement are defined in the control flow semantics.

## Ternary

```
<ternary> ::= <expr> "?" <expr> ":" <expr> ;
```

Dependencies:

* `<expr>`

The `<ternary>` is a branching statement that takes an expression, followed
by a question mark, or ternary operator, followed by two colon separated
expressions.

## Semantics

### If Else If Branch

The expression of the "if" statement is evaluated. The type of the expression
must either be a boolean or it must be a value that can be cast to a boolean.
If the result is true, the subsequent block of code is executed. Otherwise the
next branch is checked. If the optional "else if" follows, the above process
is repeated until either there are no more branches or the optional "else"
follows. If no branches have resolved to true, the "else" block is executed.

```
fn main() {
    let n = 3;

    if (n == 1) {
        // ..
    } else if (n == 2) {
        // ..
    } else {
        // ..
    }
}
```

### If Match

The "if match" statement executes as the "if" statement does, however, the
expression to evaluate is a pattern match. While the pattern match semantics
are specified elsewhere, the "if match" branch brings into scope the
identifier(s) of the inner type(s) of the matched pattern.

```
type Union = A(u8) | B;

fn main() {
    let u = Union::A(1);

    if u matches Union::A(n) {
        assert(n == 1);
    }
}
```

### Match

Matching requires all possible patterns for a given expression's type to be
evaluated. If any pattern is not matched in a match block, a compiler error
is thrown. The semantics for match arms are the same as those for the "if match"
statement.

The remaining branches for a pattern match may be grouped together either with
an identifier or if the identifier is unnecessary, an underscore. Using an
identifier assigns a subset of the associated type into scope. The subset of
the type contains one of the unmatched members. This does not create a new
distinct data type, rather it infers the non-existence of the pre-matched
branches.

```
type Ua = A | B;
type Ub = A | B;

fn main() {
    let u_a = Ua::B;
    let u_b = Ub::B;

    match u_a {
        Ua::A => {},
        Ub::B => {},
    }

    match u_b {
        Ua::A => {},
        n => {
            // `n` inferred to have type `Ub::B`
        }
    }
}
```

### Ternary

The ternary operator evaluates the expression, the first expression's result must
be of type boolean, and if the expression evaluates to true, the second expression
is evaluated, otherwise the third expression is evaluated.

```
fn main() {
    let condition = true;

    let mut a = 0;

    if (condition) {
        a = 1;
    } else {
        a = 2;
    }

    let b = condition ? 1 : 2;

    assert(a == b);
}
```

### Short Circuiting

For all branch statements that evaluate a boolean expression to determine which
branches to take, the following statements hold if the expression is composed of
multiple inner boolean expressions separated by logical operators.

* if `<expr0> && <expr1>` and `<expr0>` is `false`, short circuit to `false`
* if `<expr0> || <expr1>` and `<expr0>` is `true`, short circuit to `true`

Also, for all chains of "if else if" statements, if the first evaluates to true,
do not evaluate the remaining chained statements.
