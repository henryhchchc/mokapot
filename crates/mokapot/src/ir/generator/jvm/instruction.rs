use std::collections::BTreeMap;

use crate::{
    ir::{
        expression::{Condition, Expression},
        generator::{
            identity::SsaValueId,
            jvm::{normalization::Location, symbolic_execution::SymbolicValue},
        },
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug)]
pub(crate) enum RegisterInstruction {
    HandlerEntry,
    Unwind,
    Erased,
    Definition {
        value: SsaValueId,
        expr: Expression<SymbolicValue>,
    },
    Effect(Expression<SymbolicValue>),
    Jump {
        condition: Option<Condition<SymbolicValue>>,
        target: ProgramCounter,
    },
    Switch {
        match_value: SymbolicValue,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<SymbolicValue>),
    Throw(SymbolicValue),
    Subroutine {
        target: Location,
    },
    SubroutineReturn(SymbolicValue),
}

impl RegisterInstruction {
    pub const fn is_explicit_transfer(&self) -> bool {
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
