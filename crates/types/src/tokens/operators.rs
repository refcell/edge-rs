//! Operators

use derive_more::Display;

/// Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum Operator {
    /// An Assignment Operator
    #[display("=")]
    Assignment,
    /// Compound Assignment Operator
    #[display("{_0}")]
    CompoundAssignment(CompoundAssignmentOperator),
    /// Arithmetic Operator
    #[display("{_0}")]
    Arithmetic(ArithmeticOperator),
    /// Comparison Operator
    #[display("{_0}")]
    Comparison(ComparisonOperator),
    /// Logical Operator
    #[display("{_0}")]
    Logical(LogicalOperator),
    /// Bitwise Operator
    #[display("{_0}")]
    Bitwise(BitwiseOperator),
}

/// Compound assignment operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum CompoundAssignmentOperator {
    /// +=
    #[display("+=")]
    AddAssign,
    /// -=
    #[display("-=")]
    SubAssign,
    /// *=
    #[display("*=")]
    MulAssign,
    /// /=
    #[display("/=")]
    DivAssign,
    /// %=
    #[display("%=")]
    ModAssign,
    /// **=
    #[display("**=")]
    ExpAssign,
    /// &=
    #[display("&=")]
    AndAssign,
    /// |=
    #[display("|=")]
    OrAssign,
    /// ^=
    #[display("^=")]
    XorAssign,
    /// >>=
    #[display(">>=")]
    ShrAssign,
    /// <<=
    #[display("<<=")]
    ShlAssign,
}

/// Arithmetic operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum ArithmeticOperator {
    /// Addition
    #[display("+")]
    Add,
    /// Subtraction
    #[display("-")]
    Sub,
    /// Multiplication
    #[display("*")]
    Mul,
    /// Division
    #[display("/")]
    Div,
    /// Modulus
    #[display("%")]
    Mod,
    /// Exponentiation
    #[display("**")]
    Exp,
}

/// Comparison Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum ComparisonOperator {
    /// Less than
    #[display("<")]
    LessThan,
    /// Less than or equal to
    #[display("<=")]
    LessThanOrEqual,
    /// Greater than
    #[display(">")]
    GreaterThan,
    /// Greater than or equal to
    #[display(">=")]
    GreaterThanOrEqual,
    /// Equal to
    #[display("==")]
    Equal,
    /// Not equal to
    #[display("!=")]
    NotEqual,
}

/// Logical Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum LogicalOperator {
    /// Logical AND
    #[display("&&")]
    And,
    /// Logical OR
    #[display("||")]
    Or,
    /// Logical NOT
    #[display("!")]
    Not,
}

/// Bitwise Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub enum BitwiseOperator {
    /// Bitwise AND
    #[display("&")]
    And,
    /// Bitwise OR
    #[display("|")]
    Or,
    /// Bitwise XOR
    #[display("^")]
    Xor,
    /// Bitwise NOT
    #[display("~")]
    Not,
    /// Bitwise Left Shift
    #[display("<<")]
    LeftShift,
    /// Bitwise Right Shift
    #[display(">>")]
    RightShift,
}
