use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use super::{Condition, Expression};
use crate::elements::instruction::ProgramCounter;
use itertools::Itertools;

/// Represents a single instruction in the Moka IR.
#[derive(Debug)]
pub enum MokaInstruction {
    /// A no-op instruction.
    Nop,
    /// Assigns [`expr`](MokaInstruction::Assignment::expr) to [`def_id`](MokaInstruction::Assignment::def_id).
    Assignment {
        def_id: Identifier,
        expr: Expression,
    },
    /// Evaluates an [`Expression`] for its side effects.
    SideEffect(Expression),
    /// Jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) holds.
    /// Unconditionally jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) is [`None`].
    Jump {
        condition: Option<Condition>,
        target: ProgramCounter,
    },
    /// Jump to the [`target`](MokaInstruction::Switch::default) corresponding to [`match_value`](MokaInstruction::Switch::match_value).
    /// If [`match_value`](MokaInstruction::Switch::match_value) does not match any [`target`](MokaInstruction::Switch::branches), jump to [`default`](MokaInstruction::Switch::default).
    Switch {
        match_value: ValueRef,
        default: ProgramCounter,
        branches: Vec<(i32, ProgramCounter)>,
    },
    /// Returns from the current method with [`value`](MokaInstruction::Return::value) if [`value`](MokaInstruction::Return::value) is [`Some`].
    /// Otherwise, returns from the current method with `void`.
    Return(Option<ValueRef>),
    /// Returns from a subroutine.
    SubroutineRet(ValueRef),
}

impl Display for MokaInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nop => write!(f, "nop"),
            Self::Assignment {
                def_id: lhs,
                expr: rhs,
            } => write!(f, "{} = {}", lhs, rhs),
            Self::SideEffect(op) => write!(f, "{}", op),
            Self::Jump {
                condition: Some(condition),
                target,
            } => {
                write!(f, "if {} goto {}", condition, target)
            }
            Self::Jump {
                condition: None,
                target,
            } => {
                write!(f, "goto {}", target)
            }
            Self::Switch {
                match_value,
                default,
                branches,
            } => {
                write!(
                    f,
                    "switch {} {{ {}, else => {} }}",
                    match_value,
                    default,
                    branches
                        .iter()
                        .map(|(key, target)| format!("{} => {}", key, target))
                        .join(", ")
                )
            }
            Self::Return(value) => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            Self::SubroutineRet(target) => write!(f, "ret {}", target),
        }
    }
}

/// Represents a reference to a value in the Moka IR.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueRef {
    /// A reference to a value defined in the current scope.
    Def(Identifier),
    /// A reference to a value combined from multiple branches.
    /// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
    Phi(HashSet<Identifier>),
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Def(id) => id.fmt(f),
            Self::Phi(ids) => write!(
                f,
                "Phi({})",
                ids.iter().map(|id| format!("{}", id)).join(", ")
            ),
        }
    }
}

impl From<Identifier> for ValueRef {
    fn from(value: Identifier) -> Self {
        Self::Def(value)
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Identifier {
    /// A locally defined value.
    Val(u16),
    /// The `this` value in an instance method.
    This,
    /// An argument of the current method.
    Arg(u16),
    /// The exception caught by a `catch` block.
    CaughtException,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Identifier::*;
        match self {
            Val(idx) => write!(f, "v{}", idx),
            This => write!(f, "this"),
            Arg(idx) => write!(f, "arg{}", idx),
            CaughtException => write!(f, "exception"),
        }
    }
}
