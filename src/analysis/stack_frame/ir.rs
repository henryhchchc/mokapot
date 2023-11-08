use crate::elements::instruction::{Instruction, ProgramCounter};

use super::{Expression, Identifier, ValueRef};

pub enum MokaInstruction {
    Assignment {
        lhs: u16,
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
