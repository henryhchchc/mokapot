use std::{collections::BTreeMap, fmt};

use super::{DiscoveryValue, Location, ProvisionalValueId};
use crate::ir::expression::{LiftedCondition, LiftedExpression};
use crate::jvm::code::ProgramCounter;

#[derive(Debug, Clone)]
pub(super) enum LiftedInstruction<OP: fmt::Display = DiscoveryValue> {
    HandlerEntry,
    Unwind,
    Erased,
    Definition {
        value: ProvisionalValueId,
        expr: LiftedExpression<OP>,
    },
    Effect(LiftedExpression<OP>),
    Jump {
        condition: Option<LiftedCondition<OP>>,
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

impl<OP: fmt::Display> LiftedInstruction<OP> {
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
