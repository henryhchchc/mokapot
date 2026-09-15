//! Stack effects and analyzed successors derived from structural terminators.

use std::collections::BTreeMap;

use super::super::{
    analyzer::Analyzer,
    jvm::ValueCategory::{Category1, Category2},
    model::{AnalyzedEdge, Frame, Location},
};
use crate::ir::{
    OperationKind, TerminatorKind,
    control_flow::{
        ControlTransfer,
        path_condition::{BooleanVariable, BranchGuard, PathValue},
    },
    expression::Predicate,
    generator::{
        bytecode_cfg::{
            self, BranchPredicate, ReturnOperand, StructuralBlockId, StructuralTerminator,
        },
        error::Error,
    },
};

type LoweredTerminator = (TerminatorKind, Vec<AnalyzedEdge>, Option<OperationKind>);

impl Analyzer<'_, '_> {
    /// Lowers the structural terminator of `block`, appending its successors.
    ///
    /// The returned successors' target set is a pure function of `block`:
    /// `frame` only supplies operand values, so a mismatching frame can turn
    /// the lowering into an error but never change the targets (see
    /// `Analyzer::execute`).
    pub(super) fn lower_terminator(
        block: &bytecode_cfg::Block,
        frame: &mut Frame,
    ) -> Result<LoweredTerminator, Error> {
        let result = match &block.terminator {
            StructuralTerminator::Fallthrough { target } => {
                let terminator_kind = if block.exceptional_successors.is_empty() {
                    TerminatorKind::Goto
                } else {
                    TerminatorKind::Fallible
                };
                (terminator_kind, vec![unconditional(*target)], None)
            }
            StructuralTerminator::Goto { target } => {
                (TerminatorKind::Goto, vec![unconditional(*target)], None)
            }
            StructuralTerminator::Branch {
                predicate,
                taken,
                fallthrough,
            } => lower_branch(*predicate, *taken, *fallthrough, frame)?,
            StructuralTerminator::Switch { cases, default } => {
                lower_switch(cases, *default, frame)?
            }
            StructuralTerminator::Return { operand } => {
                let value = match operand {
                    ReturnOperand::Void => None,
                    ReturnOperand::Category1 => Some(frame.stack.pop(Category1)?),
                    ReturnOperand::Category2 => Some(frame.stack.pop(Category2)?),
                };
                (TerminatorKind::Return(value), Vec::new(), None)
            }
            StructuralTerminator::Throw => {
                let throw = TerminatorKind::Throw(frame.stack.pop(Category1)?);
                (throw, Vec::new(), None)
            }
        };
        Ok(result)
    }
}

fn lower_branch(
    predicate: BranchPredicate,
    taken: StructuralBlockId,
    fallthrough: StructuralBlockId,
    frame: &mut Frame,
) -> Result<LoweredTerminator, Error> {
    let condition: BooleanVariable<_> = pop_condition(frame, predicate)?.into();
    Ok((
        TerminatorKind::Branch,
        vec![
            AnalyzedEdge {
                target: Location::Bytecode(taken),
                transfer: ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
            },
            AnalyzedEdge {
                target: Location::Bytecode(fallthrough),
                transfer: ControlTransfer::Conditional(BranchGuard::of(!condition)),
            },
        ],
        None,
    ))
}

fn lower_switch(
    cases: &BTreeMap<i32, StructuralBlockId>,
    default: StructuralBlockId,
    frame: &mut Frame,
) -> Result<LoweredTerminator, Error> {
    let match_value = frame.stack.pop(Category1)?;
    let mut successors = cases
        .iter()
        .map(|(&case, &target)| AnalyzedEdge {
            target: Location::Bytecode(target),
            transfer: ControlTransfer::Conditional(BranchGuard::of(BooleanVariable::Positive(
                Predicate::Equal(
                    match_value.into(),
                    PathValue::Constant(crate::jvm::ConstantValue::Integer(case)),
                ),
            ))),
        })
        .collect::<Vec<_>>();
    let default_guard = cases
        .keys()
        .map(|case| {
            BooleanVariable::Negative(Predicate::Equal(
                match_value.into(),
                PathValue::Constant(crate::jvm::ConstantValue::Integer(*case)),
            ))
        })
        .collect();
    successors.push(AnalyzedEdge {
        target: Location::Bytecode(default),
        transfer: ControlTransfer::Conditional(default_guard),
    });
    Ok((TerminatorKind::Switch { match_value }, successors, None))
}

fn pop_condition(frame: &mut Frame, predicate: BranchPredicate) -> Result<Predicate, Error> {
    if predicate.operand_count() == 1 {
        let operand = frame.stack.pop(Category1)?.into();
        let condition = match predicate {
            BranchPredicate::IsZero => Predicate::IsZero(operand),
            BranchPredicate::IsNonZero => Predicate::IsNonZero(operand),
            BranchPredicate::IsNegative => Predicate::IsNegative(operand),
            BranchPredicate::IsNonNegative => Predicate::IsNonNegative(operand),
            BranchPredicate::IsPositive => Predicate::IsPositive(operand),
            BranchPredicate::IsNonPositive => Predicate::IsNonPositive(operand),
            BranchPredicate::IsNull => Predicate::IsNull(operand),
            BranchPredicate::IsNotNull => Predicate::IsNotNull(operand),
            _ => unreachable!("operand count classifies every branch predicate"),
        };
        return Ok(condition);
    }

    let rhs = frame.stack.pop(Category1)?.into();
    let lhs = frame.stack.pop(Category1)?.into();
    Ok(match predicate {
        BranchPredicate::Equal => Predicate::Equal(lhs, rhs),
        BranchPredicate::NotEqual => Predicate::NotEqual(lhs, rhs),
        BranchPredicate::LessThan => Predicate::LessThan(lhs, rhs),
        BranchPredicate::GreaterThanOrEqual => Predicate::GreaterThanOrEqual(lhs, rhs),
        BranchPredicate::GreaterThan => Predicate::GreaterThan(lhs, rhs),
        BranchPredicate::LessThanOrEqual => Predicate::LessThanOrEqual(lhs, rhs),
        _ => unreachable!("operand count classifies every branch predicate"),
    })
}

const fn unconditional(target: StructuralBlockId) -> AnalyzedEdge {
    AnalyzedEdge {
        target: Location::Bytecode(target),
        transfer: ControlTransfer::Unconditional,
    }
}
