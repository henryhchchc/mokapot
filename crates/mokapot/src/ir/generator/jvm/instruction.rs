use std::collections::BTreeMap;

use crate::{
    ir::{
        expression::{Condition, Expression},
        generator::{
            identity::SsaValueId,
            jvm::{analysis::OperandState, normalization::Location},
        },
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug)]
pub(in crate::ir::generator) enum Instruction {
    HandlerEntry,
    Unwind,
    Erased,
    Definition {
        value: SsaValueId,
        expr: Expression<OperandState>,
    },
    Effect(Expression<OperandState>),
    Jump {
        condition: Option<Condition<OperandState>>,
        target: ProgramCounter,
    },
    Switch {
        match_value: OperandState,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<OperandState>),
    Throw(OperandState),
    Subroutine {
        target: Location,
    },
    SubroutineReturn(OperandState),
}

impl Instruction {
    pub(in crate::ir::generator) const fn is_explicit_transfer(&self) -> bool {
        matches!(
            self,
            Self::HandlerEntry
                | Self::Unwind
                | Self::Jump { .. }
                | Self::Switch { .. }
                | Self::Return(_)
                | Self::Throw(_)
                | Self::Subroutine { .. }
                | Self::SubroutineReturn(_)
        )
    }
}
