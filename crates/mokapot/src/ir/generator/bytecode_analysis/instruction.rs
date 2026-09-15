//! Register-form instructions used by the instruction graph.

use std::collections::BTreeMap;

use super::{NodeAddress, Value};
use crate::{
    ir::{
        expression::{Condition, Expression},
        generator::identity::SsaValueId,
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
        target: NodeAddress,
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
