import{u as a,j as e}from"./index-B0DkJJZv.js";const l={title:"Branching",description:"undefined"};function i(s){const n={a:"a",code:"code",div:"div",h1:"h1",h2:"h2",h3:"h3",header:"header",li:"li",p:"p",pre:"pre",span:"span",ul:"ul",...a(),...s.components};return e.jsxs(e.Fragment,{children:[e.jsx(n.header,{children:e.jsxs(n.h1,{id:"branching",children:["Branching",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#branching",children:e.jsx(n.div,{"data-autolink-icon":!0})})]})}),`
`,e.jsx(n.p,{children:"Branching refers to blocks of code that may be executed based on a defined condition."}),`
`,e.jsxs(n.h2,{id:"if-else-if-branch",children:["If Else If Branch",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#if-else-if-branch",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<if_else_if_branch> ::= "if" "(" <expr> ")" <code_block>'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    ("else" "if" "(" <expr> ")" <code_block>)*'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    ["else" <code_block>] ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<code_block>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<branch>"}),` contains an "if" keyword followed by a parenthesis delimited expression
and a code block. It may be followed by zero or more conditions under "else" "if"
keywords followed by a parenthesis delimited expression and a code block, and finally
it may optionally be suffixed with an "else" keyword followed by a code block.`]}),`
`,e.jsxs(n.h2,{id:"if-match",children:["If Match",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#if-match",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<if_match_branch> ::= "if" <pattern_match> <code_block> ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<pattern_match>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<code_block>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<if_match_branch>"}),` contains a pattern match expression followed by an optionally
typed identifier followed by a code block.`]}),`
`,e.jsxs(n.h2,{id:"match",children:["Match",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#match",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<match_arm> ::= (<union_pattern> | <ident> | "_") "=>" <code_block> ;'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:"<match> ::="})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "match" <expr> "{"'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    [<match_arm> ("," <match_arm>)* [","]]'})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'    "}" ;'})})]})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<union_pattern>"})}),`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<code_block>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<match_arm>"}),` is a single arm of a match statement. It may optionally be
prefixed with a union pattern and contains a lambda.`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<match>"}),` statement is a group of match arms that may pattern match against
an expression.`]}),`
`,e.jsx(n.p,{children:"Semantics of the match statement are defined in the control flow semantics."}),`
`,e.jsxs(n.h2,{id:"ternary",children:["Ternary",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#ternary",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsx(n.code,{children:e.jsx(n.span,{className:"line",children:e.jsx(n.span,{children:'<ternary> ::= <expr> "?" <expr> ":" <expr> ;'})})})})}),`
`,e.jsx(n.p,{children:"Dependencies:"}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsx(n.li,{children:e.jsx(n.code,{children:"<expr>"})}),`
`]}),`
`,e.jsxs(n.p,{children:["The ",e.jsx(n.code,{children:"<ternary>"}),` is a branching statement that takes an expression, followed
by a question mark, or ternary operator, followed by two colon separated
expressions.`]}),`
`,e.jsxs(n.h2,{id:"semantics",children:["Semantics",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#semantics",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsxs(n.h3,{id:"if-else-if-branch-1",children:["If Else If Branch",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#if-else-if-branch-1",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The expression of the "if" statement is evaluated. The type of the expression
must either be a boolean or it must be a value that can be cast to a boolean.
If the result is true, the subsequent block of code is executed. Otherwise the
next branch is checked. If the optional "else if" follows, the above process
is repeated until either there are no more branches or the optional "else"
follows. If no branches have resolved to true, the "else" block is executed.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn main() {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let n = 3;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    if (n == 1) {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        // .."})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    } else if (n == 2) {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        // .."})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    } else {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        // .."})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsxs(n.h3,{id:"if-match-1",children:["If Match",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#if-match-1",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The "if match" statement executes as the "if" statement does, however, the
expression to evaluate is a pattern match. While the pattern match semantics
are specified elsewhere, the "if match" branch brings into scope the
identifier(s) of the inner type(s) of the matched pattern.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Union = A(u8) | B;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn main() {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let u = Union::A(1);"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    if u matches Union::A(n) {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        assert(n == 1);"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsxs(n.h3,{id:"match-1",children:["Match",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#match-1",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`Matching requires all possible patterns for a given expression's type to be
evaluated. If any pattern is not matched in a match block, a compiler error
is thrown. The semantics for match arms are the same as those for the "if match"
statement.`}),`
`,e.jsx(n.p,{children:`The remaining branches for a pattern match may be grouped together either with
an identifier or if the identifier is unnecessary, an underscore. Using an
identifier assigns a subset of the associated type into scope. The subset of
the type contains one of the unmatched members. This does not create a new
distinct data type, rather it infers the non-existence of the pre-matched
branches.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Ua = A | B;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"type Ub = A | B;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn main() {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let u_a = Ua::B;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let u_b = Ub::B;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    match u_a {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        Ua::A => {},"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        Ub::B => {},"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    match u_b {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        Ua::A => {},"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        n => {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"            // `n` inferred to have type `Ub::B`"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsxs(n.h3,{id:"ternary-1",children:["Ternary",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#ternary-1",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`The ternary operator evaluates the expression, the first expression's result must
be of type boolean, and if the expression evaluates to true, the second expression
is evaluated, otherwise the third expression is evaluated.`}),`
`,e.jsx(e.Fragment,{children:e.jsx(n.pre,{className:"shiki shiki-themes github-light github-dark-dimmed",style:{backgroundColor:"#fff","--shiki-dark-bg":"#22272e",color:"#24292e","--shiki-dark":"#adbac7"},tabIndex:"0",children:e.jsxs(n.code,{children:[e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"fn main() {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let condition = true;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let mut a = 0;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    if (condition) {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        a = 1;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    } else {"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"        a = 2;"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    }"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    let b = condition ? 1 : 2;"})}),`
`,e.jsx(n.span,{className:"line","data-empty-line":!0,children:" "}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"    assert(a == b);"})}),`
`,e.jsx(n.span,{className:"line",children:e.jsx(n.span,{style:{color:"#24292E","--shiki-dark":"#ADBAC7"},children:"}"})})]})})}),`
`,e.jsxs(n.h3,{id:"short-circuiting",children:["Short Circuiting",e.jsx(n.a,{"aria-hidden":"true",tabIndex:"-1",href:"#short-circuiting",children:e.jsx(n.div,{"data-autolink-icon":!0})})]}),`
`,e.jsx(n.p,{children:`For all branch statements that evaluate a boolean expression to determine which
branches to take, the following statements hold if the expression is composed of
multiple inner boolean expressions separated by logical operators.`}),`
`,e.jsxs(n.ul,{children:[`
`,e.jsxs(n.li,{children:["if ",e.jsx(n.code,{children:"<expr0> && <expr1>"})," and ",e.jsx(n.code,{children:"<expr0>"})," is ",e.jsx(n.code,{children:"false"}),", short circuit to ",e.jsx(n.code,{children:"false"})]}),`
`,e.jsxs(n.li,{children:["if ",e.jsx(n.code,{children:"<expr0> || <expr1>"})," and ",e.jsx(n.code,{children:"<expr0>"})," is ",e.jsx(n.code,{children:"true"}),", short circuit to ",e.jsx(n.code,{children:"true"})]}),`
`]}),`
`,e.jsx(n.p,{children:`Also, for all chains of "if else if" statements, if the first evaluates to true,
do not evaluate the remaining chained statements.`})]})}function c(s={}){const{wrapper:n}={...a(),...s.components};return n?e.jsx(n,{...s,children:e.jsx(i,{...s})}):i(s)}export{c as default,l as frontmatter};
