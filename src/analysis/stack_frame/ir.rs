use crate::elements::instruction::{Instruction, ProgramCounter};

use super::{Expression, Identifier, ValueRef};

#[derive(Debug)]
pub enum MokaInstruction {
    Assignment {
        lhs: Identifier,
        rhs: Expression,
    },
    Jump {
        target: ProgramCounter,
    },
    ConditionalJump {
        condition: ValueRef,
        target: ProgramCounter,
    },
    Switch {
        condition: ValueRef,
        instruction: Instruction,
    },
    Return {
        value: Option<ValueRef>,
    },
}
