use std::collections::BTreeMap;

use super::model::{BranchPredicate, ReturnOperand, StructuralBlockId, StructuralTerminator};
use crate::{
    ir::generator::error::{Error, MalformedBytecode},
    jvm::code::{Instruction, MethodBody, ProgramCounter, WideInstruction},
};

/// The PC-level control flow of one decoded instruction.
pub(super) enum InstructionFlow<'instruction> {
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
    TableSwitch {
        range: std::ops::RangeInclusive<i32>,
        jump_targets: &'instruction [ProgramCounter],
        default: ProgramCounter,
    },
    LookupSwitch {
        match_targets: &'instruction BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    },
    Jsr {
        target: ProgramCounter,
        continuation: ProgramCounter,
    },
    Ret {
        local: u16,
    },
    Return {
        operand: ReturnOperand,
    },
    Throw,
}

impl<'instruction> InstructionFlow<'instruction> {
    /// Classifies one instruction without losing its concrete PCs.
    ///
    /// This match is deliberately exhaustive so adding a JVM instruction
    /// requires deciding its structural control-flow behavior here.
    pub(super) fn classify(
        body: &MethodBody,
        pc: ProgramCounter,
        instruction: &'instruction Instruction,
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
                Self::TableSwitch {
                    range: range.clone(),
                    jump_targets,
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
                Self::LookupSwitch {
                    match_targets,
                    default: *default,
                }
            }
            Instruction::Jsr(target) | Instruction::JsrW(target) => {
                validate_target(*target)?;
                Self::Jsr {
                    target: *target,
                    continuation: fallthrough()?,
                }
            }
            Instruction::Ret(local) => Self::Ret {
                local: u16::from(*local),
            },
            Instruction::Wide(WideInstruction::Ret(local)) => Self::Ret { local: *local },
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
            Self::TableSwitch {
                range,
                jump_targets,
                default,
            } => StructuralTerminator::Switch {
                cases: range
                    .clone()
                    .zip(*jump_targets)
                    .map(|(case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Self::LookupSwitch {
                match_targets,
                default,
            } => StructuralTerminator::Switch {
                cases: match_targets
                    .iter()
                    .map(|(&case, &target)| block_at(target).map(|block| (case, block)))
                    .collect::<Result<_, _>>()?,
                default: block_at(*default)?,
            },
            Self::Jsr {
                target,
                continuation,
            } => StructuralTerminator::Jsr {
                target: block_at(*target)?,
                continuation: block_at(*continuation)?,
            },
            Self::Ret { local } => StructuralTerminator::Ret {
                local: *local,
                continuations: BTreeMap::new(),
            },
            Self::Return { operand } => StructuralTerminator::Return { operand: *operand },
            Self::Throw => StructuralTerminator::Throw,
        };
        Ok(terminator)
    }
}
