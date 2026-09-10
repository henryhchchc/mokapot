use std::collections::BTreeMap;

use super::{Location, OperandState, SsaValueId};
use crate::ir::expression::{Condition, Expression};
use crate::jvm::code::ProgramCounter;

#[derive(Debug)]
pub(super) enum Instruction<OP = OperandState> {
    HandlerEntry,
    Unwind,
    Erased,
    Definition {
        value: SsaValueId,
        expr: Expression<OP>,
    },
    Effect(Expression<OP>),
    Jump {
        condition: Option<Condition<OP>>,
        target: ProgramCounter,
    },
    Switch {
        match_value: OP,
        branches: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return(Option<OP>),
    Throw(OP),
    Subroutine {
        target: Location,
    },
    SubroutineReturn(OP),
}

impl<OP> Instruction<OP> {
    pub(super) const fn is_explicit_transfer(&self) -> bool {
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
