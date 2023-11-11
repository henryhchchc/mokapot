use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use itertools::Itertools;

use crate::elements::instruction::ProgramCounter;

use super::{Condition, Expression};

#[derive(Debug)]
pub enum MokaInstruction {
    Nop,
    Assignment {
        lhs: Identifier,
        rhs: Expression,
    },
    SideEffect {
        rhs: Expression,
    },
    Jump {
        condition: Option<Condition>,
        target: ProgramCounter,
    },
    Switch {
        match_value: ValueRef,
        default: ProgramCounter,
        branches: Vec<(i32, ProgramCounter)>,
    },
    Return {
        value: Option<ValueRef>,
    },
    SubRoutineRet {
        target: ValueRef,
    },
}

impl Display for MokaInstruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nop => write!(f, "nop"),
            Self::Assignment { lhs, rhs } => write!(f, "{} = {}", lhs, rhs),
            Self::SideEffect { rhs: op } => write!(f, "{}", op),
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
            Self::Return { value } => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            Self::SubRoutineRet { target } => write!(f, "ret {}", target),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueRef {
    Def(Identifier),
    Phi(HashSet<Identifier>),
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Def(id) => write!(f, "{}", id),
            Self::Phi(ids) => {
                write!(
                    f,
                    "Phi({})",
                    ids.iter().map(|id| format!("{}", id)).join(", ")
                )
            }
        }
    }
}

impl From<Identifier> for ValueRef {
    fn from(value: Identifier) -> Self {
        Self::Def(value)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Identifier {
    Val(u16),
    This,
    Arg(u16),
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
