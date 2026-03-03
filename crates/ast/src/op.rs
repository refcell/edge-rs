//! Operator AST nodes
//!
//! Defines binary and unary operators used in expressions.

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    /// Addition: +
    Add,
    /// Subtraction: -
    Sub,
    /// Multiplication: *
    Mul,
    /// Division: /
    Div,
    /// Modulus: %
    Mod,
    /// Exponentiation: **
    Exp,

    // Bitwise
    /// Bitwise AND: &
    BitwiseAnd,
    /// Bitwise OR: |
    BitwiseOr,
    /// Bitwise XOR: ^
    BitwiseXor,
    /// Bitwise left shift: <<
    Shl,
    /// Bitwise right shift: >>
    Shr,

    // Logical
    /// Logical AND: &&
    LogicalAnd,
    /// Logical OR: ||
    LogicalOr,

    // Comparison
    /// Equal: ==
    Eq,
    /// Not equal: !=
    Neq,
    /// Less than: <
    Lt,
    /// Less than or equal: <=
    Lte,
    /// Greater than: >
    Gt,
    /// Greater than or equal: >=
    Gte,

    // Compound assignment
    /// Add-assign: +=
    AddAssign,
    /// Sub-assign: -=
    SubAssign,
    /// Mul-assign: *=
    MulAssign,
    /// Div-assign: /=
    DivAssign,
    /// Mod-assign: %=
    ModAssign,
    /// Exp-assign: **=
    ExpAssign,
    /// Bitwise AND-assign: &=
    BitwiseAndAssign,
    /// Bitwise OR-assign: |=
    BitwiseOrAssign,
    /// Bitwise XOR-assign: ^=
    BitwiseXorAssign,
    /// Left shift-assign: <<=
    ShlAssign,
    /// Right shift-assign: >>=
    ShrAssign,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Negation: -
    Neg,
    /// Bitwise NOT: ~
    BitwiseNot,
    /// Logical NOT: !
    LogicalNot,
}
