use std::fmt::Display;

use crate::elements::instruction::{Instruction, ProgramCounter};

use super::{Expression, Identifier, ValueRef};

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MokaInstruction::Nop => write!(f, "nop"),
            MokaInstruction::Assignment { lhs, rhs } => write!(f, "{} = {}", lhs, rhs),
            MokaInstruction::SideEffect { rhs: op } => write!(f, "{}", op),
            MokaInstruction::Jump { target } => write!(f, "goto {}", target),
            MokaInstruction::UnitaryConditionalJump {
                condition,
                target,
                instruction,
            } => write!(f, "{}({}) goto {}", instruction.name(), condition, target),
            MokaInstruction::BinaryConditionalJump {
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
            MokaInstruction::Switch {
                condition,
                instruction,
            } => write!(f, "{}({})", instruction.name(), condition),
            MokaInstruction::Return { value } => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            MokaInstruction::SubRoutineRet { target } => write!(f, "ret {}", target),
        }
    }
}
