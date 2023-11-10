use std::{
    collections::HashSet,
    fmt::{Display, Formatter},
};

use itertools::Itertools;

use crate::elements::instruction::{Instruction, ProgramCounter};

use super::Expression;

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
        target: ProgramCounter,
    },
    UnitaryConditionalJump {
        condition: ValueRef,
        target: ProgramCounter,
        instruction: Instruction,
    },
    BinaryConditionalJump {
        condition: [ValueRef; 2],
        target: ProgramCounter,
        instruction: Instruction,
    },
    Switch {
        condition: ValueRef,
        instruction: Instruction,
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
            Self::Jump { target } => write!(f, "goto {}", target),
            Self::UnitaryConditionalJump {
                condition,
                target,
                instruction,
            } => write!(f, "{}({}) goto {}", instruction.name(), condition, target),
            Self::BinaryConditionalJump {
                condition,
                target,
                instruction,
            } => {
                write!(
                    f,
                    "{}({}, {}) goto {}",
                    instruction.name(),
                    condition[0],
                    condition[1],
                    target
                )
            }
            Self::Switch {
                condition,
                instruction,
            } => write!(f, "{}({})", instruction.name(), condition),
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
