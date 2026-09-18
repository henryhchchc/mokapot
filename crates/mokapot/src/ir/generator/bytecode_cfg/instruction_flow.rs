use std::collections::BTreeMap;

use super::model::{BranchPredicate, ReturnOperand, StructuralBlockId, StructuralTerminator};
use crate::{
    ir::generator::error::{Error, MalformedBytecode},
    jvm::code::{Instruction, MethodBody, ProgramCounter},
};

/// The validated PC-level control flow of one decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PcFlow {
    Fallthrough {
        target: ProgramCounter,
    },
    Goto {
        target: ProgramCounter,
    },
    Branch {
        predicate: BranchPredicate,
        taken: ProgramCounter,
        fallthrough: ProgramCounter,
    },
    Switch {
        cases: BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Return {
        operand: ReturnOperand,
    },
    Throw,
}

impl PcFlow {
    /// Classifies one instruction without losing its concrete PCs.
    ///
    /// Instructions without structural control-flow behavior use the final
    /// fallthrough case.
    pub(super) fn classify(
        body: &MethodBody,
        pc: ProgramCounter,
        instruction: &Instruction,
    ) -> Result<Self, Error> {
        let fallthrough = || {
            body.instructions
                .next_pc_of(&pc)
                .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingFallthrough))
        };
        let validate_target = |target| {
            body.instruction_at(target)
                .is_some()
                .then_some(())
                .ok_or_else(|| {
                    Error::malformed(Some(target), MalformedBytecode::MissingInstruction)
                })
        };
        let branch = |predicate, taken| {
            validate_target(taken)?;
            Ok::<_, Error>(Self::Branch {
                predicate,
                taken,
                fallthrough: fallthrough()?,
            })
        };

        let flow = match instruction {
            Instruction::IfEq(target) => branch(BranchPredicate::IsZero, *target)?,
            Instruction::IfNe(target) => branch(BranchPredicate::IsNonZero, *target)?,
            Instruction::IfLt(target) => branch(BranchPredicate::IsNegative, *target)?,
            Instruction::IfGe(target) => branch(BranchPredicate::IsNonNegative, *target)?,
            Instruction::IfGt(target) => branch(BranchPredicate::IsPositive, *target)?,
            Instruction::IfLe(target) => branch(BranchPredicate::IsNonPositive, *target)?,
            Instruction::IfICmpEq(target) | Instruction::IfACmpEq(target) => {
                branch(BranchPredicate::Equal, *target)?
            }
            Instruction::IfICmpNe(target) | Instruction::IfACmpNe(target) => {
                branch(BranchPredicate::NotEqual, *target)?
            }
            Instruction::IfICmpLt(target) => branch(BranchPredicate::LessThan, *target)?,
            Instruction::IfICmpGe(target) => branch(BranchPredicate::GreaterThanOrEqual, *target)?,
            Instruction::IfICmpGt(target) => branch(BranchPredicate::GreaterThan, *target)?,
            Instruction::IfICmpLe(target) => branch(BranchPredicate::LessThanOrEqual, *target)?,
            Instruction::IfNull(target) => branch(BranchPredicate::IsNull, *target)?,
            Instruction::IfNonNull(target) => branch(BranchPredicate::IsNotNull, *target)?,
            Instruction::Goto(target) | Instruction::GotoW(target) => {
                validate_target(*target)?;
                Self::Goto { target: *target }
            }
            Instruction::TableSwitch {
                range,
                jump_targets,
                default,
            } => {
                for target in jump_targets.iter().copied().chain([*default]) {
                    validate_target(target)?;
                }
                Self::Switch {
                    cases: range.clone().zip(jump_targets.iter().copied()).collect(),
                    default: *default,
                }
            }
            Instruction::LookupSwitch {
                default,
                match_targets,
            } => {
                for target in match_targets.values().copied().chain([*default]) {
                    validate_target(target)?;
                }
                Self::Switch {
                    cases: match_targets.clone(),
                    default: *default,
                }
            }
            Instruction::IReturn | Instruction::FReturn | Instruction::AReturn => Self::Return {
                operand: ReturnOperand::Category1,
            },
            Instruction::LReturn | Instruction::DReturn => Self::Return {
                operand: ReturnOperand::Category2,
            },
            Instruction::Return => Self::Return {
                operand: ReturnOperand::Void,
            },
            Instruction::AThrow => Self::Throw,
            _ => Self::Fallthrough {
                target: fallthrough()?,
            },
        };
        Ok(flow)
    }

    pub(super) fn resolve(
        &self,
        block_at: impl Fn(ProgramCounter) -> Result<StructuralBlockId, Error>,
    ) -> Result<StructuralTerminator, Error> {
        let terminator = match self {
            Self::Fallthrough { target } => StructuralTerminator::Fallthrough {
                target: block_at(*target)?,
            },
            Self::Goto { target } => StructuralTerminator::Goto {
                target: block_at(*target)?,
            },
            Self::Branch {
                predicate,
                taken,
                fallthrough,
            } => StructuralTerminator::Branch {
                predicate: *predicate,
                taken: block_at(*taken)?,
                fallthrough: block_at(*fallthrough)?,
            },
            Self::Switch { cases, default } => StructuralTerminator::Switch {
                cases: cases
                    .iter()
                    .map(|(&case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Self::Return { operand } => StructuralTerminator::Return { operand: *operand },
            Self::Throw => StructuralTerminator::Throw,
        };
        Ok(terminator)
    }
}
