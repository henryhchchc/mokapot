use std::{collections::HashSet, fmt};

use super::{
    BlockId, EdgeId, InstructionId, TryMapValues, ValueId, control_flow::ControlTransfer,
    expression::Predicate,
};
/// One ordered outgoing arm of a terminator.
///
/// Arms have independent identities, so parallel transfers between the same
/// two blocks remain distinguishable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Successor {
    pub(super) id: EdgeId,
    pub(super) target: BlockId,
    pub(super) transfer: ControlTransfer,
}

impl Successor {
    /// Returns this arm's identity.
    #[must_use]
    pub const fn id(&self) -> EdgeId {
        self.id
    }
    /// Returns the target block.
    #[must_use]
    pub const fn target(&self) -> BlockId {
        self.target
    }
    /// Returns the state transfer associated with this arm.
    #[must_use]
    pub const fn transfer(&self) -> &ControlTransfer {
        &self.transfer
    }
}

/// The control-flow operation ending a basic block.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum TerminatorKind<OP = ValueId> {
    /// Transfers control to one successor.
    #[display("goto")]
    Goto,
    /// Enters a legacy bytecode subroutine.
    #[display("subroutine call")]
    SubroutineCall,
    /// Returns from a legacy bytecode subroutine through an address token.
    #[display("subroutine return {address}")]
    SubroutineReturn {
        /// The return-address value consumed by `ret`.
        address: OP,
    },
    /// Selects one of two guarded successors.
    #[display("branch")]
    Branch,
    /// Selects a successor by matching a value.
    #[display("switch {match_value}")]
    Switch {
        /// The value matched by the switch arms.
        match_value: OP,
    },
    /// Completes the method normally, with exceptional successors when method
    /// exit itself can fail.
    #[display("return{}", _0.as_ref().map(|value| format!(" {value}")).unwrap_or_default())]
    Return(Option<OP>),
    /// Throws an exception.
    #[display("throw {_0}")]
    Throw(OP),
    /// Selects the normal or an exceptional outcome of a fallible operation.
    #[display("fallible")]
    Fallible,
    /// Propagates an exception out of the method.
    #[display("unwind")]
    Unwind,
}

impl<OP, OUT> TryMapValues<OUT> for TerminatorKind<OP> {
    type Value = OP;
    type Mapped = TerminatorKind<OUT>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(OP) -> Result<OUT, E>,
    ) -> Result<TerminatorKind<OUT>, E> {
        Ok(match self {
            Self::Goto => TerminatorKind::Goto,
            Self::SubroutineCall => TerminatorKind::SubroutineCall,
            Self::SubroutineReturn { address } => TerminatorKind::SubroutineReturn {
                address: remap(address)?,
            },
            Self::Branch => TerminatorKind::Branch,
            Self::Switch { match_value } => TerminatorKind::Switch {
                match_value: remap(match_value)?,
            },
            Self::Return(value) => TerminatorKind::Return(value.map(remap).transpose()?),
            Self::Throw(value) => TerminatorKind::Throw(remap(value)?),
            Self::Fallible => TerminatorKind::Fallible,
            Self::Unwind => TerminatorKind::Unwind,
        })
    }
}

/// An identified terminator and its ordered successor arms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator {
    pub(super) id: InstructionId,
    pub(super) kind: TerminatorKind,
    pub(super) successors: Vec<Successor>,
}

impl Terminator {
    /// Returns this terminator's method-local identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }
    /// Returns the control-flow operation.
    #[must_use]
    pub const fn kind(&self) -> &TerminatorKind {
        &self.kind
    }
    /// Returns the ordered outgoing arms.
    ///
    /// Exception arms retain JVM handler-table precedence.
    #[must_use]
    pub fn successors(&self) -> &[Successor] {
        &self.successors
    }
    /// Returns the values used by this terminator and its successor guards.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        let mut uses = match &self.kind {
            TerminatorKind::Switch { match_value: value }
            | TerminatorKind::SubroutineReturn { address: value }
            | TerminatorKind::Throw(value)
            | TerminatorKind::Return(Some(value)) => HashSet::from([*value]),
            TerminatorKind::Goto
            | TerminatorKind::SubroutineCall
            | TerminatorKind::Branch
            | TerminatorKind::Return(None)
            | TerminatorKind::Fallible
            | TerminatorKind::Unwind => HashSet::new(),
        };
        for successor in &self.successors {
            match successor.transfer() {
                ControlTransfer::Conditional(guard)
                | ControlTransfer::SubroutineReturn { guard, .. } => {
                    uses.extend(guard.predicates().flat_map(Predicate::uses));
                }
                ControlTransfer::Unconditional
                | ControlTransfer::SubroutineCall { .. }
                | ControlTransfer::Exception(_)
                | ControlTransfer::Unwind => {}
            }
        }
        uses
    }
}

impl fmt::Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ControlTransfer, EdgeId, InstructionId, Successor, Terminator, TerminatorKind,
        TryMapValues, ValueId,
    };
    use crate::{
        ir::{
            control_flow::path_condition::{BooleanVariable, BranchGuard, PathValue},
            expression::Condition,
        },
        jvm::code::ProgramCounter,
    };

    #[test]
    fn maps_value_bearing_terminators() {
        assert_eq!(
            TerminatorKind::Switch { match_value: 1_u8 }
                .try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(TerminatorKind::Switch {
                match_value: 11_u16
            })
        );
        assert_eq!(
            TerminatorKind::Return(Some(1_u8))
                .try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(TerminatorKind::Return(Some(11_u16)))
        );
        assert_eq!(
            TerminatorKind::Throw(1_u8).try_map_values(|_| Err::<u16, _>("unmapped")),
            Err("unmapped")
        );
        assert_eq!(
            TerminatorKind::SubroutineReturn { address: 2_u8 }
                .try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(TerminatorKind::SubroutineReturn { address: 12_u16 })
        );

        assert_eq!(
            TerminatorKind::<u8>::SubroutineCall.to_string(),
            "subroutine call"
        );
        assert_eq!(
            TerminatorKind::SubroutineReturn { address: 3_u8 }.to_string(),
            "subroutine return 3"
        );
    }

    #[test]
    fn uses_include_subroutine_return_address_and_guard_values() {
        let continuation = ProgramCounter::from(0x12);
        let terminator = Terminator {
            id: InstructionId::new(0),
            kind: TerminatorKind::SubroutineReturn {
                address: ValueId::new(1),
            },
            successors: vec![Successor {
                id: EdgeId::new(0),
                target: super::BlockId::new(1),
                transfer: ControlTransfer::SubroutineReturn {
                    continuation,
                    guard: BranchGuard::of(BooleanVariable::Positive(Condition::Equal(
                        PathValue::Variable(ValueId::new(2)),
                        PathValue::ReturnAddress(continuation),
                    ))),
                },
            }],
        };

        assert_eq!(
            terminator.uses(),
            [ValueId::new(1), ValueId::new(2)].into_iter().collect()
        );
    }
}
