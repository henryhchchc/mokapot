use std::{
    collections::{BTreeMap, BTreeSet, btree_set},
    iter::{self, Once},
    ops::BitOr,
};

use itertools::{Either, Itertools};

use super::expression::{Condition, Expression};
use crate::jvm::code::ProgramCounter;

/// Represents a single instruction in the Moka IR.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
#[instability::unstable(feature = "moka-ir")]
pub enum MokaInstruction {
    /// A no-op instruction.
    #[display("nop")]
    Nop,
    /// Creates a definition by evaluating an [`Expression`].
    #[display("{value} = {expr}")]
    Definition {
        /// The value defined by the expression.
        value: LocalValue,
        /// The expression that defines the value.
        expr: Expression,
    },
    /// Jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) holds.
    /// Unconditionally jumps to [`target`](MokaInstruction::Jump::target) if [`condition`](MokaInstruction::Jump::condition) is [`None`].
    #[display("{}goto {target}", condition.as_ref().map(|cond| format!("if {cond} ")).unwrap_or_default())]
    Jump {
        /// The condition that must hold for the jump to occur.
        /// It denotes an Unconditional jump if it is [`None`].
        condition: Option<Condition>,
        /// The target of the jump.
        target: ProgramCounter,
    },
    /// Jump to the [`target`](MokaInstruction::Switch::default) corresponding to [`match_value`](MokaInstruction::Switch::match_value).
    /// If [`match_value`](MokaInstruction::Switch::match_value) does not match any [`target`](MokaInstruction::Switch::branches), jump to [`default`](MokaInstruction::Switch::default).
    #[display(
        "switch {} {{ {}, else => {} }}",
        match_value,
        branches.iter().map(|(key, target)| format!("{key} => {target}")).join(", "),
        default
    )]
    Switch {
        /// The value to match against the branches.
        match_value: Operand,
        /// The branches of the switch.
        branches: BTreeMap<i32, ProgramCounter>,
        /// The target of the switch if no branches match.
        default: ProgramCounter,
    },
    /// Returns from the current method with a value if it is [`Some`].
    /// Otherwise, returns from the current method with `void`.
    #[display("return{}", _0.as_ref().map(|it| format!(" {it}")).unwrap_or_default())]
    Return(Option<Operand>),
    /// Returns from a subroutine.
    #[display("subroutine_ret {_0}")]
    SubroutineRet(Operand),
}

impl MokaInstruction {
    /// Returns the value defined by the instruction if it is a definition.
    #[must_use]
    pub fn def(&self) -> Option<LocalValue> {
        match self {
            Self::Definition { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Returns the set of [`Identifier`]s used by the instruction.
    #[must_use]
    pub fn uses(&self) -> BTreeSet<Identifier> {
        match self {
            Self::Nop => BTreeSet::new(),
            Self::Definition { expr, .. } => expr.uses(),
            Self::Jump {
                condition: Some(condition),
                ..
            } => condition.uses(),
            Self::Return(Some(uses))
            | Self::SubroutineRet(uses)
            | Self::Switch {
                match_value: uses, ..
            } => uses.iter().copied().collect(),
            _ => BTreeSet::default(),
        }
    }
}

/// Represents a reference to a value in the Moka IR.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, derive_more::Display)]
#[instability::unstable(feature = "moka-ir")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Operand {
    /// A reference to a value defined in the current scope.
    #[display("{_0}")]
    Just(Identifier),
    /// A reference to a value combined from multiple branches.
    /// See the Phi function in [Static single-assignment form](https://en.wikipedia.org/wiki/Static_single-assignment_form) for more information.
    #[display("Phi({})", _0.iter().map(ToString::to_string).join(", "))]
    Phi(BTreeSet<Identifier>),
}

impl From<Identifier> for Operand {
    fn from(value: Identifier) -> Self {
        Self::Just(value)
    }
}

impl BitOr for Operand {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        use Operand::{Just, Phi};
        match (self, rhs) {
            (Just(lhs), Just(rhs)) if lhs == rhs => Just(lhs),
            (Just(lhs), Just(rhs)) => Phi(BTreeSet::from([lhs, rhs])),
            (Just(id), Phi(mut ids)) | (Phi(mut ids), Just(id)) => {
                ids.insert(id);
                Phi(ids)
            }
            (Phi(mut lhs), Phi(mut rhs)) => {
                lhs.append(&mut rhs);
                Phi(lhs)
            }
        }
    }
}

impl IntoIterator for Operand {
    type Item = Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, btree_set::IntoIter<Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        use Operand::{Just, Phi};
        match self {
            Just(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.into_iter()),
        }
    }
}

impl<'a> IntoIterator for &'a Operand {
    type Item = &'a Identifier;

    // TODO: Replace it with opaque type when it's stable.
    //       See https://github.com/rust-lang/rust/issues/63063.
    type IntoIter = Either<Once<Self::Item>, btree_set::Iter<'a, Identifier>>;

    fn into_iter(self) -> Self::IntoIter {
        use Operand::{Just, Phi};
        match self {
            Just(id) => Either::Left(iter::once(id)),
            Phi(ids) => Either::Right(ids.iter()),
        }
    }
}

impl Operand {
    /// Creates an iterator over the possible [`Identifier`].
    pub fn iter(&self) -> impl Iterator<Item = &Identifier> {
        self.into_iter()
    }
}

/// A unique identifier of a value defined in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[repr(transparent)]
#[display("%{_0}")]
#[instability::unstable(feature = "moka-ir")]
pub struct LocalValue(u16);

impl LocalValue {
    /// Creates a new [`LocalValue`] with the given ID.
    #[must_use]
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Create an [`Operand`] by referencing this [`LocalValue`].
    #[must_use]
    pub fn as_argument(&self) -> Operand {
        Operand::Just((*self).into())
    }
}

impl From<LocalValue> for u16 {
    fn from(value: LocalValue) -> Self {
        value.0
    }
}

/// Represents an identifier of a value in the current scope.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[instability::unstable(feature = "moka-ir")]
pub enum Identifier {
    /// The `this` value in an instance method.
    #[display("%this")]
    This,
    /// An argument of the current method.
    #[display("%arg{_0}")]
    Arg(u16),
    /// A locally defined value.
    Local(LocalValue),
    /// The exception caught by a `catch` block.
    #[display("%caught_exception@{_0}")]
    CaughtException(ProgramCounter),
}

impl From<LocalValue> for Identifier {
    fn from(value: LocalValue) -> Self {
        Self::Local(value)
    }
}

#[cfg(test)]
pub(super) mod test {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn local_value_inner_conversion(id in 0..u16::MAX) {
            let value = LocalValue::new(id);
            let id: u16 = value.into();
            assert_eq!(id, value.0);
        }
    }

    pub(crate) fn arb_argument() -> impl Strategy<Value = Operand> {
        prop_oneof![
            any::<Identifier>().prop_map(Operand::Just),
            prop::collection::btree_set(any::<Identifier>(), 1..10).prop_map(Operand::Phi)
        ]
    }

    #[test]
    fn value_ref_merge() {
        use std::collections::BTreeSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(Just(This) | Just(This), Just(This));
        assert_eq!(
            Just(This) | Just(Arg(0)),
            Phi(BTreeSet::from([This, Arg(0)]))
        );
        assert_eq!(
            Just(Arg(0)) | Just(This),
            Phi(BTreeSet::from([This, Arg(0)]))
        );
        assert_eq!(
            Just(Arg(0)) | Just(Arg(1)),
            Phi(BTreeSet::from([Arg(0), Arg(1)]))
        );
        assert_eq!(
            Just(Arg(0)) | Phi(BTreeSet::from([Arg(1), Arg(2)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Just(Arg(0)),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2)]))
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(1), Arg(2)])) | Phi(BTreeSet::from([Arg(0), Arg(1), Arg(3)])),
            Phi(BTreeSet::from([Arg(0), Arg(1), Arg(2), Arg(3)]))
        );
    }

    #[test]
    fn value_ref_iter() {
        use std::collections::BTreeSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(
            Just(This).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([This])
        );
        assert_eq!(
            Just(Arg(0)).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([Arg(0)])
        );
        assert_eq!(
            Phi(BTreeSet::from([Arg(0), Arg(1)]))
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([Arg(0), Arg(1)])
        );
    }

    #[test]
    fn value_ref_iter_over_refs() {
        use std::collections::BTreeSet;

        use super::{Identifier::*, Operand::*};

        assert_eq!(
            (&Just(This)).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([&This])
        );
        assert_eq!(
            (&Just(Arg(0))).into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from([&Arg(0)])
        );
        assert_eq!(
            (&Phi(BTreeSet::from([Arg(0), Arg(1)])))
                .into_iter()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([&Arg(0), &Arg(1)])
        );
    }
}
