use std::{collections::HashSet, fmt, slice};

use super::{
    BlockId, EdgeId, Operation, ValueId,
    control_flow::{ControlTransfer, path_condition::BranchGuard},
    expression::Predicate,
};

/// One outgoing arm of a terminator.
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

/// A structurally valid control-flow operation ending a basic block.
///
/// The shape is parameterized by its outgoing arm type so lifted and completed
/// IR stages share one definition. [`Successor`] is the default, so
/// `Terminator` names the completed public form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator<Arm = Successor> {
    /// Transfers control to one successor.
    Goto {
        /// The sole continuation.
        target: Arm,
    },
    /// Selects one of two guarded successors.
    Branch {
        /// The arm selected when the condition holds.
        taken: Arm,
        /// The arm selected when the condition does not hold.
        otherwise: Arm,
    },
    /// Selects an arm by matching a value, or takes the default arm.
    ///
    /// Each case arm carries a conditional guard matching the selected value
    /// against its case key, and the default arm carries the negation of every
    /// case. Those guards are the complete switch semantics.
    Switch {
        /// Ordered case arms.
        cases: Vec<Arm>,
        /// The arm selected when no key matches.
        default: Arm,
    },
    /// Selects the normal or an exceptional outcome of a fallible operation.
    Try {
        /// The operation attempted by this terminator.
        operation: Operation,
        /// The continuation taken after successful completion.
        normal: Arm,
        /// Ordered exceptional outcomes.
        exceptional: Vec<Arm>,
    },
    /// Completes the method normally.
    Return {
        /// The returned value, or `None` for a void return.
        value: Option<ValueId>,
    },
    /// Attempts to complete the method normally, but may fail while exiting.
    TryReturn {
        /// The returned value, or `None` for a void return.
        value: Option<ValueId>,
        /// Ordered failures possible while completing the return.
        exceptional: Vec<Arm>,
    },
    /// Throws an exception to an ordered handler or out of the method.
    Throw {
        /// The thrown value.
        value: ValueId,
        /// Ordered handlers followed by an optional unwind exit.
        exceptional: Vec<Arm>,
    },
}

impl<Arm> Terminator<Arm> {
    /// Maps every outgoing arm while preserving the terminator's structure.
    pub(crate) fn map_arms<MappedArm>(
        self,
        mut map: impl FnMut(Arm) -> MappedArm,
    ) -> Terminator<MappedArm> {
        match self {
            Self::Goto { target } => Terminator::Goto {
                target: map(target),
            },
            Self::Branch { taken, otherwise } => Terminator::Branch {
                taken: map(taken),
                otherwise: map(otherwise),
            },
            Self::Switch { cases, default } => Terminator::Switch {
                cases: cases.into_iter().map(&mut map).collect(),
                default: map(default),
            },
            Self::Try {
                operation,
                normal,
                exceptional,
            } => Terminator::Try {
                operation,
                normal: map(normal),
                exceptional: exceptional.into_iter().map(map).collect(),
            },
            Self::Return { value } => Terminator::Return { value },
            Self::TryReturn { value, exceptional } => Terminator::TryReturn {
                value,
                exceptional: exceptional.into_iter().map(map).collect(),
            },
            Self::Throw { value, exceptional } => Terminator::Throw {
                value,
                exceptional: exceptional.into_iter().map(map).collect(),
            },
        }
    }

    /// Iterates over outgoing arms in semantic order.
    pub(crate) fn arms(&self) -> impl Iterator<Item = &Arm> {
        let (head, tail): (&[_], &[_]) = match self {
            Self::Goto { target } => (slice::from_ref(target), &[]),
            Self::Branch { taken, otherwise } => {
                (slice::from_ref(taken), slice::from_ref(otherwise))
            }
            Self::Switch { cases, default } => (cases, slice::from_ref(default)),
            Self::Try {
                normal,
                exceptional,
                ..
            } => (slice::from_ref(normal), exceptional),
            Self::TryReturn { exceptional, .. } | Self::Throw { exceptional, .. } => {
                (&[], exceptional)
            }
            Self::Return { .. } => (&[], &[]),
        };
        head.iter().chain(tail)
    }

    /// Iterates mutably over outgoing arms in semantic order.
    pub(crate) fn arms_mut(&mut self) -> impl Iterator<Item = &mut Arm> {
        let (head, tail): (&mut [_], &mut [_]) = match self {
            Self::Goto { target } => (slice::from_mut(target), &mut []),
            Self::Branch { taken, otherwise } => {
                (slice::from_mut(taken), slice::from_mut(otherwise))
            }
            Self::Switch { cases, default } => (cases, slice::from_mut(default)),
            Self::Try {
                normal,
                exceptional,
                ..
            } => (slice::from_mut(normal), exceptional),
            Self::TryReturn { exceptional, .. } | Self::Throw { exceptional, .. } => {
                (&mut [], exceptional)
            }
            Self::Return { .. } => (&mut [], &mut []),
        };
        head.iter_mut().chain(tail)
    }
}

impl Terminator<Successor> {
    #[cfg(test)]
    pub(crate) fn successor_mut(
        &mut self,
        mut predicate: impl FnMut(&Successor) -> bool,
    ) -> Option<&mut Successor> {
        self.arms_mut().find(|arm| predicate(arm))
    }

    /// Iterates over outgoing arms in semantic order.
    pub fn successors(&self) -> impl Iterator<Item = &Successor> {
        self.arms()
    }

    /// Returns the operation attempted by this terminator, if any.
    #[must_use]
    pub const fn operation(&self) -> Option<&Operation> {
        match self {
            Self::Try { operation, .. } => Some(operation),
            _ => None,
        }
    }

    /// Returns the values used by this terminator and its successor guards.
    #[must_use]
    pub fn uses(&self) -> HashSet<ValueId> {
        let successor_values = self
            .successors()
            .flat_map(|successor| successor.arguments.iter().copied());
        self.local_uses()
            .into_iter()
            .chain(successor_values)
            .collect()
    }

    /// Returns the values this terminator uses, excluding successor arguments.
    pub(crate) fn local_uses(&self) -> HashSet<ValueId> {
        let value = match self {
            Self::Throw { value, .. }
            | Self::Return { value: Some(value) }
            | Self::TryReturn {
                value: Some(value), ..
            } => Some(*value),
            _ => None,
        };
        let guard_uses = self
            .successors()
            .filter_map(|it| match it.transfer() {
                ControlTransfer::Conditional(guard) => Some(guard),
                _ => None,
            })
            .flat_map(BranchGuard::predicates)
            .flat_map(Predicate::uses);
        let operation_uses = match self {
            Self::Try { operation, .. } => operation.uses(),
            _ => HashSet::new(),
        };
        value
            .into_iter()
            .chain(operation_uses)
            .chain(guard_uses)
            .collect()
    }

    /// Returns the value defined by a successful attempted operation.
    #[must_use]
    pub const fn def(&self) -> Option<ValueId> {
        match self {
            Self::Try { operation, .. } => operation.def(),
            _ => None,
        }
    }
}

impl<Arm> fmt::Display for Terminator<Arm> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Goto { .. } => f.write_str("goto"),
            Self::Branch { .. } => f.write_str("branch"),
            Self::Switch { .. } => f.write_str("switch"),
            Self::Try { operation, .. } => write!(f, "try {operation}"),
            Self::Return { value: Some(value) } => write!(f, "return {value}"),
            Self::Return { value: None } => f.write_str("return"),
            Self::TryReturn {
                value: Some(value), ..
            } => write!(f, "try return {value}"),
            Self::TryReturn { value: None, .. } => f.write_str("try return"),
            Self::Throw { value, .. } => write!(f, "throw {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(id: u32) -> Successor {
        Successor {
            id: EdgeId::new(id),
            target: SuccessorTarget::Block(BlockId::new(0)),
            arguments: vec![],
            transfer: ControlTransfer::Unconditional,
        }
    }

    fn ids(terminator: &Terminator) -> Vec<EdgeId> {
        terminator.successors().map(Successor::id).collect()
    }

    #[test]
    fn structural_terminators_iterate_arms_in_semantic_order() {
        assert_eq!(ids(&Terminator::Goto { target: arm(0) }), [EdgeId::new(0)]);
        assert_eq!(
            ids(&Terminator::Branch {
                taken: arm(1),
                otherwise: arm(2),
            }),
            [EdgeId::new(1), EdgeId::new(2)]
        );
        assert_eq!(
            ids(&Terminator::Switch {
                cases: vec![arm(3), arm(4)],
                default: arm(5),
            }),
            [EdgeId::new(3), EdgeId::new(4), EdgeId::new(5)]
        );
        assert_eq!(
            ids(&Terminator::Throw {
                value: ValueId::new(0),
                exceptional: vec![arm(6), arm(7)],
            }),
            [EdgeId::new(6), EdgeId::new(7)]
        );
    }

    #[test]
    fn try_terminator_orders_normal_before_exceptional_outcomes() {
        let terminator = Terminator::Try {
            operation: crate::ir::Operation::Effect {
                expr: crate::ir::expression::Expression::Const(crate::jvm::ConstantValue::Null),
            },
            normal: arm(2),
            exceptional: vec![arm(0), arm(1)],
        };
        assert_eq!(
            ids(&terminator),
            [EdgeId::new(2), EdgeId::new(0), EdgeId::new(1)]
        );
    }
}
