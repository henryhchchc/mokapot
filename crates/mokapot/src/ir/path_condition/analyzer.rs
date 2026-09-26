use std::{cmp, collections::HashMap, convert::Infallible};

use super::{BlockId, BranchGuard, MokaIRMethod, PathCondition, Predicate, SolvingBudget};
use crate::{
    analysis::fixed_point::{DataflowProblem, JoinSemiLattice},
    ir::{
        ControlTransfer, Operation, ValueId,
        expression::{BooleanVariable, Expression, PathValue},
    },
    jvm::ConstantValue,
};

/// A forward dataflow analysis that propagates path conditions through a CFG.
#[derive(Debug)]
pub(super) struct PathConditionProblem<'method> {
    method: &'method MokaIRMethod,
    constants: HashMap<ValueId, &'method ConstantValue>,
    budget: SolvingBudget,
}

impl<'method> PathConditionProblem<'method> {
    /// Creates a path-condition analysis over the given method.
    #[must_use]
    pub(super) fn new(method: &'method MokaIRMethod, budget: SolvingBudget) -> Self {
        let constants = method
            .blocks
            .values()
            .flat_map(|block| &block.operations)
            .filter_map(|operation| match operation {
                Operation::Definition {
                    value,
                    expr: Expression::Const(constant),
                } => Some((*value, constant)),
                _ => None,
            })
            .collect();
        Self {
            method,
            constants,
            budget,
        }
    }

    fn normalize_guard(&self, guard: &BranchGuard<Predicate>) -> Option<BranchGuard<Predicate>> {
        let mut literals = Vec::new();
        for literal in guard.literals() {
            let (predicate, positive) = match literal {
                BooleanVariable::Positive(predicate) => (predicate, true),
                BooleanVariable::Negative(predicate) => (predicate, false),
            };
            let canonical: BooleanVariable<Predicate> = self.normalize_predicate(predicate).into();
            let canonical = if positive { canonical } else { !canonical };
            let (predicate, positive) = match &canonical {
                BooleanVariable::Positive(predicate) => (predicate, true),
                BooleanVariable::Negative(predicate) => (predicate, false),
            };
            if let Some(value) = evaluate_atom(predicate) {
                if value != positive {
                    return None;
                }
            } else {
                literals.push(canonical);
            }
        }
        Some(literals.into_iter().collect())
    }

    fn normalize_predicate(&self, predicate: &Predicate) -> Predicate {
        let value = |value: &PathValue| match value {
            PathValue::Variable(id) => self.constants.get(id).map_or_else(
                || value.clone(),
                |constant| PathValue::Constant((*constant).clone()),
            ),
            PathValue::Constant(_) => value.clone(),
        };
        match predicate.map_values(value) {
            Predicate::Equal(lhs, rhs) => normalize_equality(lhs, rhs, true),
            Predicate::NotEqual(lhs, rhs) => normalize_equality(lhs, rhs, false),
            mapped => mapped,
        }
    }
}

fn normalize_equality(lhs: PathValue, rhs: PathValue, equal: bool) -> Predicate {
    use Predicate::{Equal, IsNonZero, IsNotNull, IsNull, IsZero, NotEqual};
    let (lhs, rhs) = match (lhs, rhs) {
        (constant @ PathValue::Constant(_), variable @ PathValue::Variable(_)) => {
            (variable, constant)
        }
        pair => pair,
    };
    match (&lhs, &rhs, equal) {
        (PathValue::Variable(_), PathValue::Constant(ConstantValue::Integer(0)), true) => {
            IsZero(lhs)
        }
        (PathValue::Variable(_), PathValue::Constant(ConstantValue::Integer(0)), false) => {
            IsNonZero(lhs)
        }
        (PathValue::Variable(_), PathValue::Constant(ConstantValue::Null), true) => IsNull(lhs),
        (PathValue::Variable(_), PathValue::Constant(ConstantValue::Null), false) => IsNotNull(lhs),
        (_, _, true) => Equal(lhs, rhs),
        (_, _, false) => NotEqual(lhs, rhs),
    }
}

fn evaluate_atom(predicate: &Predicate) -> Option<bool> {
    use Predicate::{Equal, IsNegative, IsNull, IsPositive, IsZero, LessThan};
    let integer = |value: &PathValue| match value {
        PathValue::Constant(ConstantValue::Integer(value)) => Some(*value),
        _ => None,
    };
    let integers = |lhs: &PathValue, rhs: &PathValue| Some((integer(lhs)?, integer(rhs)?));
    match predicate {
        Equal(lhs, rhs) => {
            if matches!(lhs, PathValue::Constant(ConstantValue::Null))
                && matches!(rhs, PathValue::Constant(ConstantValue::Null))
            {
                Some(true)
            } else {
                integers(lhs, rhs).map(|(lhs, rhs)| lhs == rhs)
            }
        }
        LessThan(lhs, rhs) => integers(lhs, rhs).map(|(lhs, rhs)| lhs < rhs),
        IsNull(value) => matches!(value, PathValue::Constant(ConstantValue::Null)).then_some(true),
        IsZero(value) => integer(value).map(|value| value == 0),
        IsPositive(value) => integer(value).map(|value| value > 0),
        IsNegative(value) => integer(value).map(|value| value < 0),
        _ => None,
    }
}

impl DataflowProblem for PathConditionProblem<'_> {
    type Location = BlockId;

    type Fact = PathConditionFact;

    type Err = Infallible;

    fn seeds(&self) -> impl IntoIterator<Item = (Self::Location, Self::Fact)> {
        [(self.method.entry.block, PathConditionFact::one(self.budget))]
    }

    fn flow(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<impl IntoIterator<Item = (Self::Location, Self::Fact)>, Self::Err> {
        let block = self
            .method
            .block(*location)
            .expect("a dataflow location is a block of the method");
        Ok(block.terminator.successors().filter_map(|successor| {
            let target = successor.block_target()?;
            let propagated = match successor.transfer() {
                Some(ControlTransfer::Conditional(guard)) => {
                    fact.conjoin_branch_guard(self.normalize_guard(guard)?)
                }
                Some(ControlTransfer::Unconditional | ControlTransfer::Exception(_)) | None => {
                    fact.clone()
                }
            };
            (!propagated.is_contradiction()).then_some((target, propagated))
        }))
    }
}

/// Internal lattice wrapper used by the generic fixed-point solver.
///
/// Facts are reduced without generalization; the returned ones are generalized
/// by [`PathCondition::analyze_with_budget`].
#[derive(Debug, Clone)]
pub(super) struct PathConditionFact {
    inner: PathCondition<Predicate>,
    budget: SolvingBudget,
}

impl PathConditionFact {
    pub(super) fn one(budget: SolvingBudget) -> Self {
        Self::new(PathCondition::one(), budget)
    }

    pub(super) fn new(inner: PathCondition<Predicate>, budget: SolvingBudget) -> Self {
        let inner = inner.reduce_without_generalization(budget);
        Self { inner, budget }
    }

    pub(super) fn conjoin_branch_guard(&self, branch_guard: BranchGuard<Predicate>) -> Self {
        if branch_guard.is_tautology() {
            return self.clone();
        }
        Self::new(self.inner.conjoin_branch_guard(branch_guard), self.budget)
    }

    pub(super) fn is_contradiction(&self) -> bool {
        self.inner.is_contradiction()
    }

    pub(super) fn into_inner(self) -> PathCondition<Predicate> {
        self.inner
    }
}

impl PartialEq for PathConditionFact {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other) == Some(cmp::Ordering::Equal)
    }
}

impl Eq for PathConditionFact {}

impl PartialOrd for PathConditionFact {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        debug_assert_eq!(self.budget, other.budget);
        self.inner.cover.partial_cmp(&other.inner.cover)
    }
}

impl JoinSemiLattice for PathConditionFact {
    fn join_assign(&mut self, other: Self) -> bool {
        debug_assert_eq!(self.budget, other.budget);
        if other.inner.cover == self.inner.cover || other.inner.cover.implies(&self.inner.cover) {
            return false;
        }
        let inner = std::mem::replace(&mut self.inner, PathCondition::zero());
        *self = Self::new(inner | other.inner, self.budget);
        true
    }
}
