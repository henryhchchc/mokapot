use std::{
    collections::{BTreeSet, HashSet, btree_set},
    fmt,
    hash::Hash,
};

use super::{control_flow::ControlTransfer, expression::Expression};
use crate::analysis::fixed_point::JoinSemiLattice;
use itertools::Itertools;

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

/// The identity of an instruction or terminator within one Moka IR method.
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

/// The identity of a value within one Moka IR method.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
pub struct ValueId(u32);

impl ValueId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

impl From<ValueId> for Operand {
    fn from(value: ValueId) -> Self {
        Self::just(Identifier::Local(value))
    }
}

impl From<ValueId> for Identifier {
    fn from(value: ValueId) -> Self {
        Self::Local(value)
    }
}

/// The ordinary operation performed by a Moka IR instruction.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum InstructionKind {
    /// A no-op instruction.
    #[display("nop")]
    Nop,
    /// Creates a definition by evaluating an [`Expression`].
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: ValueId,
        /// The expression that defines the value.
        expr: Expression,
    },
}

impl InstructionKind {
    /// Returns the value defined by the instruction if it is a definition.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            Self::Nop => None,
        }
    }

    /// Returns the set of [`Identifier`]s used by the instruction.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
        match self {
            Self::Nop => HashSet::new(),
            Self::Definition { expr, .. } => expr.uses(),
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

    /// Returns the identifiers used by this instruction.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
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
        match_value: Operand,
    },
    /// Returns from the method.
    #[display("return{}", _0.as_ref().map(|value| format!(" {value}")).unwrap_or_default())]
    Return(Option<Operand>),
    /// Throws an exception.
    #[display("throw {_0}")]
    Throw(Operand),
    /// Continues normally or enters an exception handler after a fallible operation.
    #[display("fallible")]
    Fallible,
    /// Returns from a legacy JVM subroutine.
    #[display("subroutine_ret {_0}")]
    SubroutineReturn(Operand),
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

    /// Returns the identifiers used by this terminator.
    #[must_use]
    pub fn uses(&self) -> HashSet<Identifier> {
        let mut uses = match &self.kind {
            TerminatorKind::Switch { match_value }
            | TerminatorKind::Throw(match_value)
            | TerminatorKind::SubroutineReturn(match_value) => {
                match_value.iter().copied().collect()
            }
            TerminatorKind::Return(Some(value)) => value.iter().copied().collect(),
            TerminatorKind::Goto
            | TerminatorKind::Branch
            | TerminatorKind::Return(None)
            | TerminatorKind::Fallible => HashSet::new(),
        };
        for successor in &self.successors {
            if let ControlTransfer::Conditional(guard) = successor.transfer() {
                uses.extend(
                    guard.predicates().flat_map(|condition| {
                        super::expression::Condition::<
                        super::control_flow::path_condition::Value,
                    >::uses(condition)
                    }),
                );
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
    instructions: Vec<MokaInstruction>,
    terminator: Terminator,
}

impl BasicBlock {
    pub(crate) const fn new(
        id: BlockId,
        instructions: Vec<MokaInstruction>,
        terminator: Terminator,
    ) -> Self {
        Self {
            id,
            instructions,
            terminator,
        }
    }

    /// Returns this block's method-local identity.
    #[must_use]
    pub const fn id(&self) -> BlockId {
        self.id
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

/// Represents a reference to a value in the Moka IR.
/// It can contain more than one possible values for a value combined from multiple branches.
/// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Operand(
    #[cfg_attr(test, proptest(strategy = "prop_test_phi_inner()"))] BTreeSet<Identifier>,
);

/// An error returned when constructing an [`Operand`] from an empty iterator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an operand must contain at least one identifier")]
pub struct EmptyOperandError;

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() > 1 {
            write!(f, "Phi({})", self.0.iter().format(", "))
        } else {
            self.0.first().expect("Operand is always non-empty").fmt(f)
        }
    }
}

#[cfg(test)]
fn prop_test_phi_inner() -> impl proptest::strategy::Strategy<Value = BTreeSet<Identifier>> {
    use proptest::prelude::*;
    proptest::collection::hash_set(any::<Identifier>(), 1..10).prop_map(BTreeSet::from_iter)
}

impl PartialOrd for Operand {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::{Equal, Greater, Less};

        if self == other {
            Some(Equal)
        } else if self.0.is_subset(&other.0) {
            Some(Less)
        } else if other.0.is_subset(&self.0) {
            Some(Greater)
        } else {
            None
        }
    }
}

impl From<Identifier> for Operand {
    fn from(value: Identifier) -> Self {
        Self::just(value)
    }
}

impl JoinSemiLattice for Operand {
    fn join(mut self, other: Self) -> Self {
        self.0.extend(other.0);
        self
    }
}

impl IntoIterator for Operand {
    type Item = Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = btree_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Operand {
    type Item = &'a Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = btree_set::Iter<'a, Identifier>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Operand {
    /// Creates an operand that can only refer to `identifier`.
    #[must_use]
    pub fn just(identifier: Identifier) -> Self {
        Self(BTreeSet::from([identifier]))
    }

    /// Creates an operand from all identifiers yielded by `identifiers`.
    ///
    /// # Errors
    ///
    /// Returns [`EmptyOperandError`] when `identifiers` yields no identifiers.
    pub fn try_from_iter<I>(identifiers: I) -> Result<Self, EmptyOperandError>
    where
        I: IntoIterator<Item = Identifier>,
    {
        let values = BTreeSet::from_iter(identifiers);
        if values.is_empty() {
            return Err(EmptyOperandError);
        }
        Ok(Self(values))
    }

    /// Returns an iterator over the possible [`Identifier`]s.
    pub fn iter(&self) -> impl Iterator<Item = &Identifier> {
        self.into_iter()
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Identifier {
    /// The `this` value in an instance method.
    #[display("%this")]
    This,
    /// An argument of the current method.
    #[display("%arg{_0}")]
    Arg(u16),
    /// A locally defined value.
    Local(ValueId),
    /// The exception caught by a `catch` block.
    #[display("%caught_exception{_0}")]
    CaughtException(ValueId),
}

#[cfg(test)]
pub(crate) mod test {
    use proptest::prelude::*;

    use super::*;

    fn operand(identifiers: impl IntoIterator<Item = Identifier>) -> Operand {
        Operand::try_from_iter(identifiers).expect("test operands must not be empty")
    }

    proptest! {
        #[test]
        fn value_identity_display(id in any::<u32>()) {
            let value = ValueId::new(id);
            prop_assert_eq!(value.to_string(), format!("%{id}"));
        }
    }

    #[test]
    fn operand_construction() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(Operand::try_from_iter([]), Err(EmptyOperandError));
        assert_eq!(
            operand([This, This, Arg(0)])
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([This, Arg(0)])
        );
        assert_eq!(operand([This, Arg(0)]).to_string(), "Phi(%this, %arg0)");
    }

    #[test]
    fn operand_merge() {
        use super::Identifier::*;

        assert_eq!(
            Operand::just(This).join(Operand::just(This)),
            Operand::just(This)
        );
        assert_eq!(
            Operand::just(This).join(Operand::just(Arg(0))),
            operand([This, Arg(0)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(Operand::just(This)),
            operand([This, Arg(0)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(Operand::just(Arg(1))),
            operand([Arg(0), Arg(1)])
        );
        assert_eq!(
            Operand::just(Arg(0)).join(operand([Arg(1), Arg(2)])),
            operand([Arg(0), Arg(1), Arg(2)])
        );
        assert_eq!(
            operand([Arg(1), Arg(2)]).join(Operand::just(Arg(0))),
            operand([Arg(0), Arg(1), Arg(2)])
        );
        assert_eq!(
            operand([Arg(1), Arg(2)]).join(operand([Arg(0), Arg(1), Arg(3)])),
            operand([Arg(0), Arg(1), Arg(2), Arg(3)])
        );
    }

    #[test]
    fn operand_iter() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(
            Operand::just(This).into_iter().collect::<HashSet<_>>(),
            HashSet::from([This])
        );
        assert_eq!(
            Operand::just(Arg(0)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([Arg(0)])
        );
        assert_eq!(
            operand([Arg(0), Arg(1)])
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([Arg(0), Arg(1)])
        );
    }

    #[test]
    fn operand_iter_over_refs() {
        use std::collections::HashSet;

        use super::Identifier::*;

        assert_eq!(
            (&Operand::just(This)).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&This])
        );
        assert_eq!(
            (&Operand::just(Arg(0))).into_iter().collect::<HashSet<_>>(),
            HashSet::from([&Arg(0)])
        );
        assert_eq!(
            (&operand([Arg(0), Arg(1)]))
                .into_iter()
                .collect::<HashSet<_>>(),
            HashSet::from([&Arg(0), &Arg(1)])
        );
    }

    proptest! {
       #[test]
       fn operand_join_ordering(
           lhs in any::<Operand>(),
           rhs in any::<Operand>(),
       ) {
           let joined = lhs.clone().join(rhs.clone());
           prop_assert!(joined >= lhs);
           prop_assert!(joined >= rhs);
       }
    }
}
