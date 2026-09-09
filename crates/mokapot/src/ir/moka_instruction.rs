use std::{collections::HashSet, fmt};

use super::{
    control_flow::ControlTransfer,
    expression::{Expression, Predicate},
};

/// The identity of a basic block within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("b{_0}")]
pub struct BlockId(u32);

impl BlockId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
    pub(crate) const fn index(self) -> u32 {
        self.0
    }
}

/// The identity of an instruction, phi, or terminator within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("i{_0}")]
pub struct InstructionId(u32);

impl InstructionId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The identity of a control-flow edge within one Moka IR method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[repr(transparent)]
#[display("e{_0}")]
pub struct EdgeId(u32);

impl EdgeId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// The identity of a scalar value within one Moka IR method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
pub struct ValueId(u32);

impl ValueId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> u32 {
        self.0
    }
}

/// Describes where a scalar value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// The receiver of an instance method.
    This,
    /// A method parameter at the given parameter index.
    Parameter(u16),
    /// The exception introduced at a handler block.
    CaughtException(BlockId),
    /// A value produced by an ordinary instruction or phi.
    Instruction(InstructionId),
}

/// One incoming value of a phi node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhiInput {
    predecessor: BlockId,
    value: ValueId,
}

impl PhiInput {
    pub(crate) const fn new(predecessor: BlockId, value: ValueId) -> Self {
        Self { predecessor, value }
    }

    /// Returns the predecessor selecting this input.
    #[must_use]
    pub const fn predecessor(&self) -> BlockId {
        self.predecessor
    }

    /// Returns the value supplied by the predecessor.
    #[must_use]
    pub const fn value(&self) -> ValueId {
        self.value
    }
}

/// A value merge at basic-block entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phi {
    id: InstructionId,
    value: ValueId,
    inputs: Vec<PhiInput>,
}

impl Phi {
    pub(crate) const fn new(id: InstructionId, value: ValueId, inputs: Vec<PhiInput>) -> Self {
        Self { id, value, inputs }
    }

    /// Returns this phi's method-local instruction identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }

    /// Returns the value defined by this phi.
    #[must_use]
    pub const fn value(&self) -> ValueId {
        self.value
    }

    /// Returns the predecessor-indexed inputs.
    #[must_use]
    pub fn inputs(&self) -> &[PhiInput] {
        &self.inputs
    }

    /// Returns the values selected by this phi.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.inputs.iter().map(PhiInput::value).collect()
    }
}

/// The ordinary operation performed by a Moka IR instruction.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum InstructionKind {
    /// Evaluates an expression and defines its result.
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: ValueId,
        /// The expression producing the value.
        expr: Expression,
    },
    /// Evaluates an expression solely for its effects.
    #[display("{expr}")]
    Effect {
        /// The effectful expression.
        expr: Expression,
    },
}

impl InstructionKind {
    /// Returns the value defined by this operation, if any.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            Self::Effect { .. } => None,
        }
    }

    /// Returns the values used by this operation.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        match self {
            Self::Definition { expr, .. } | Self::Effect { expr } => expr.uses(),
        }
    }
}

/// An identified ordinary instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MokaInstruction {
    id: InstructionId,
    kind: InstructionKind,
}

impl MokaInstruction {
    pub(crate) const fn new(id: InstructionId, kind: InstructionKind) -> Self {
        Self { id, kind }
    }
    /// Returns this instruction's method-local identity.
    #[must_use]
    pub const fn id(&self) -> InstructionId {
        self.id
    }
    /// Returns the operation performed by this instruction.
    #[must_use]
    pub const fn kind(&self) -> &InstructionKind {
        &self.kind
    }
    /// Returns the value defined by this instruction, if any.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        self.kind.def()
    }
    /// Returns the values used by this instruction.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        self.kind.uses()
    }
}

impl fmt::Display for MokaInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

/// One ordered outgoing arm of a terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Successor {
    id: EdgeId,
    target: BlockId,
    transfer: ControlTransfer,
}

impl Successor {
    pub(crate) const fn new(id: EdgeId, target: BlockId, transfer: ControlTransfer) -> Self {
        Self {
            id,
            target,
            transfer,
        }
    }
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
    /// Returns from the method.
    #[display("return{}", _0.as_ref().map(|value| format!(" {value}")).unwrap_or_default())]
    Return(Option<ValueId>),
    /// Throws an exception.
    #[display("throw {_0}")]
    Throw(ValueId),
    /// Continues normally or enters an exception handler after a fallible operation.
    #[display("fallible")]
    Fallible,
    /// Returns from a legacy JVM subroutine.
    #[display("subroutine_ret {_0}")]
    SubroutineReturn(ValueId),
}

/// An identified terminator and its ordered successor arms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Terminator {
    id: InstructionId,
    kind: TerminatorKind,
    successors: Vec<Successor>,
}

impl Terminator {
    pub(crate) const fn new(
        id: InstructionId,
        kind: TerminatorKind,
        successors: Vec<Successor>,
    ) -> Self {
        Self {
            id,
            kind,
            successors,
        }
    }
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
    #[must_use]
    pub fn successors(&self) -> &[Successor] {
        &self.successors
    }
    /// Returns the values used by this terminator and its successor guards.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        let mut uses = match &self.kind {
            TerminatorKind::Switch { match_value: value }
            | TerminatorKind::Throw(value)
            | TerminatorKind::SubroutineReturn(value)
            | TerminatorKind::Return(Some(value)) => HashSet::from([*value]),
            TerminatorKind::Goto
            | TerminatorKind::Branch
            | TerminatorKind::Return(None)
            | TerminatorKind::Fallible => HashSet::new(),
        };
        for successor in &self.successors {
            if let ControlTransfer::Conditional(guard) = successor.transfer() {
                uses.extend(guard.predicates().flat_map(Predicate::uses));
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

/// A maximal basic block ending in exactly one terminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    id: BlockId,
    phis: Vec<Phi>,
    instructions: Vec<MokaInstruction>,
    terminator: Terminator,
}

impl BasicBlock {
    pub(crate) const fn new(
        id: BlockId,
        phis: Vec<Phi>,
        instructions: Vec<MokaInstruction>,
        terminator: Terminator,
    ) -> Self {
        Self {
            id,
            phis,
            instructions,
            terminator,
        }
    }
    /// Returns this block's method-local identity.
    #[must_use]
    pub const fn id(&self) -> BlockId {
        self.id
    }
    /// Returns the phi nodes evaluated at block entry.
    #[must_use]
    pub fn phis(&self) -> &[Phi] {
        &self.phis
    }
    /// Returns the ordinary instructions in execution order.
    #[must_use]
    pub fn instructions(&self) -> &[MokaInstruction] {
        &self.instructions
    }
    /// Returns the block terminator.
    #[must_use]
    pub const fn terminator(&self) -> &Terminator {
        &self.terminator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_exposes_predecessor_inputs() {
        let input = PhiInput::new(BlockId::new(1), ValueId::new(2));
        let phi = Phi::new(InstructionId::new(3), ValueId::new(4), vec![input]);
        assert_eq!(phi.id(), InstructionId::new(3));
        assert_eq!(phi.value(), ValueId::new(4));
        assert_eq!(phi.inputs(), &[input]);
    }
}
