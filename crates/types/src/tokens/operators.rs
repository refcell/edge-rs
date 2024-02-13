//! Operators

use std::fmt;

/// Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operator {
    /// Arithmetic Operator
    Arithmetic(ArithmeticOperator),
    /// Comparison Operator
    Comparison(ComparisonOperator),
    /// Logical Operator
    Logical(LogicalOperator),
    /// Bitwise Operator
    Bitwise(BitwiseOperator),
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Arithmetic(op) => write!(f, "{}", op),
            Operator::Comparison(op) => write!(f, "{}", op),
            Operator::Logical(op) => write!(f, "{}", op),
            Operator::Bitwise(op) => write!(f, "{}", op),
        }
    }
}

/// Arithmetic operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArithmeticOperator {
    /// Addition
    Add,
    /// Subtraction
    Sub,
    /// Multiplication
    Mul,
    /// Division
    Div,
    /// Modulus
    Mod,
}

impl fmt::Display for ArithmeticOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArithmeticOperator::Add => write!(f, "+"),
            ArithmeticOperator::Sub => write!(f, "-"),
            ArithmeticOperator::Mul => write!(f, "*"),
            ArithmeticOperator::Div => write!(f, "/"),
            ArithmeticOperator::Mod => write!(f, "%"),
        }
    }
}

/// Comparison Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComparisonOperator {
    /// Less than
    LessThan,
    /// Less than or equal to
    LessThanOrEqual,
    /// Greater than
    GreaterThan,
    /// Greater than or equal to
    GreaterThanOrEqual,
    /// Equal to
    Equal,
    /// Not equal to
    NotEqual,
}

impl fmt::Display for ComparisonOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonOperator::LessThan => write!(f, "<"),
            ComparisonOperator::LessThanOrEqual => write!(f, "<="),
            ComparisonOperator::GreaterThan => write!(f, ">"),
            ComparisonOperator::GreaterThanOrEqual => write!(f, ">="),
            ComparisonOperator::Equal => write!(f, "=="),
            ComparisonOperator::NotEqual => write!(f, "!="),
        }
    }
}

/// Logical Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogicalOperator {
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical NOT
    Not,
}

impl fmt::Display for LogicalOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOperator::And => write!(f, "&&"),
            LogicalOperator::Or => write!(f, "||"),
            LogicalOperator::Not => write!(f, "!"),
        }
    }
}

/// Bitwise Operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BitwiseOperator {
    /// Bitwise AND
    And,
    /// Bitwise OR
    Or,
    /// Bitwise XOR
    Xor,
    /// Bitwise NOT
    Not,
    /// Bitwise Left Shift
    LeftShift,
    /// Bitwise Right Shift
    RightShift,
}

impl fmt::Display for BitwiseOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitwiseOperator::And => write!(f, "&"),
            BitwiseOperator::Or => write!(f, "|"),
            BitwiseOperator::Xor => write!(f, "^"),
            BitwiseOperator::Not => write!(f, "~"),
            BitwiseOperator::LeftShift => write!(f, "<<"),
            BitwiseOperator::RightShift => write!(f, ">>"),
        }
    }
}
