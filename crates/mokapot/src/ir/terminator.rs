use std::{collections::HashSet, fmt};

use super::{BlockId, EdgeId, ValueId, control_flow::ControlTransfer, expression::Predicate};
/// One ordered outgoing arm of a terminator.
///
/// Arms have independent identities, so parallel transfers between the same
/// two blocks remain distinguishable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Successor {
    pub(super) id: EdgeId,
    pub(super) target: SuccessorTarget,
    pub(super) arguments: Vec<ValueId>,
    pub(super) transfer: ControlTransfer,
}

impl Successor {
    /// Returns this arm's identity.
    #[must_use]
    pub const fn id(&self) -> EdgeId {
        self.id
    }
    /// Returns the control-flow destination.
    #[must_use]
    pub const fn target(&self) -> SuccessorTarget {
        self.target
    }
    /// Returns the target block, or `None` when this arm exits by unwinding.
    #[must_use]
    pub const fn block_target(&self) -> Option<BlockId> {
        match self.target {
            SuccessorTarget::Block(block) => Some(block),
            SuccessorTarget::Unwind => None,
        }
    }
    /// Returns the values supplied to the target block's parameters.
    #[must_use]
    pub fn arguments(&self) -> &[ValueId] {
        &self.arguments
    }
    /// Returns the state transfer associated with this arm.
    #[must_use]
    pub const fn transfer(&self) -> &ControlTransfer {
        &self.transfer
    }
}

/// The destination of a successor arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuccessorTarget {
    /// Continues execution in a basic block.
    Block(BlockId),
    /// Propagates an exception out of the method.
    Unwind,
}

/// The control-flow operation ending a basic block.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum TerminatorKind {
    /// Transfers control to one successor.
    #[display("goto")]
    Goto,
    /// Selects one of two guarded successors.
    #[display("branch")]
    Branch,
    /// Selects a successor by matching a value.
    #[display("switch {match_value}")]
    Switch {
        /// The value matched by the switch arms.
        match_value: ValueId,
    },
    /// Completes the method normally.
    #[display("return{}", _0.as_ref().map(|value| format!(" {value}")).unwrap_or_default())]
    Return(Option<ValueId>),
    /// Throws an exception.
    #[display("throw {_0}")]
    Throw(ValueId),
    /// Selects the normal or an exceptional outcome of a fallible operation.
    #[display("fallible")]
    Fallible,
}

/// A terminator and its ordered successor arms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator {
    pub(super) kind: TerminatorKind,
    pub(super) successors: Vec<Successor>,
}

impl Terminator {
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
        let mut uses = self.local_uses();
        for successor in &self.successors {
            uses.extend(successor.arguments.iter().copied());
        }
        uses
    }

    pub(crate) fn local_uses(&self) -> HashSet<ValueId> {
        let mut uses = match &self.kind {
            TerminatorKind::Switch { match_value: value }
            | TerminatorKind::Throw(value)
            | TerminatorKind::Return(Some(value)) => HashSet::from([*value]),
            TerminatorKind::Goto
            | TerminatorKind::Branch
            | TerminatorKind::Return(None)
            | TerminatorKind::Fallible => HashSet::new(),
        };
        for successor in &self.successors {
            match successor.transfer() {
                ControlTransfer::Conditional(guard) => {
                    uses.extend(guard.predicates().flat_map(Predicate::uses));
                }
                ControlTransfer::Unconditional
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
