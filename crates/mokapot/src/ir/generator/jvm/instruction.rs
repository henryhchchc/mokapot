use std::collections::BTreeMap;

use crate::{
    ir::{
        expression::{Condition, Expression},
        generator::{
            identity::SsaValueId,
            jvm::{subroutine_expansion::Location, symbolic_execution::Value},
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
        expr: Expression<Value>,
    },
    Effect(Expression<Value>),
    Jump {
        condition: Option<Condition<Value>>,
        target: ProgramCounter,
    },
    Switch {
        match_value: Value,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<Value>),
    Throw(Value),
    Subroutine {
        target: Location,
    },
    SubroutineReturn(Value),
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
