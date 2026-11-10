//! Structural control flow of one decoded JVM instruction.

use crate::types::class_name::ClassName;

use std::{collections::BTreeMap, sync::Arc};

use super::fallibility::fallthrough_may_throw;
use crate::{
    ir::generator::error::ErrorKind,
    jvm::code::{Instruction, ProgramCounter, WideInstruction},
    types::{field_type::FieldType, reference_type::ReferenceType},
};

/// The structural exit of one decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockExit<'class, T> {
    Continue {
        next: T,
        exception_arms: Arc<[ExceptionArm<'class, T>]>,
    },
    Goto {
        target: T,
    },
    Branch {
        taken: T,
        otherwise: T,
    },
    Switch {
        cases: BTreeMap<i32, T>,
        default: T,
    },
    Return {
        exception_arms: Arc<[ExceptionArm<'class, T>]>,
    },
    Throw {
        exception_arms: Arc<[ExceptionArm<'class, T>]>,
    },
}

impl<T> BlockExit<'_, T> {
    /// Whether this exit forces the following instruction into a new block.
    ///
    /// A continuation without exception arms proceeds into the next instruction
    /// without its own terminator.
    pub(crate) fn forces_block_boundary(&self) -> bool {
        !matches!(self, Self::Continue { exception_arms, .. } if exception_arms.is_empty())
    }
}

/// One ordered exception arm of a throwing instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExceptionArm<'class, T> {
    /// Where the exception transfers.
    pub target: ExceptionTarget<T>,
    /// The caught type, or `None` for catch-all.
    pub catch_type: Option<&'class ClassName>,
}

/// An exception successor destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExceptionTarget<T> {
    /// The entry block of an exception handler.
    Handler(T),
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// The structural identity of one outgoing arm within its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArmKey {
    /// The ordinary continuation after the final instruction.
    Continue,
    /// The destination of an unconditional transfer.
    Unconditional,
    /// The taken destination of a conditional branch.
    Taken,
    /// The not-taken destination of a conditional branch.
    Otherwise,
    /// The destination of one switch case, keyed by its matched value.
    Case(i32),
    /// The destination of a switch when no case matches.
    Default,
    /// The exception arm at the given position in its source's exception-arm list.
    Exception(usize),
}

impl<'class> BlockExit<'class, ProgramCounter> {
    /// Classifies and validates instruction-local structure.
    pub(super) fn of(
        instruction: &Instruction<'_>,
        next: Option<ProgramCounter>,
        empty: &Arc<[ExceptionArm<'class, ProgramCounter>]>,
        unwind: &Arc<[ExceptionArm<'class, ProgramCounter>]>,
    ) -> Result<Self, ErrorKind> {
        use Instruction::{
            AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, IfACmpEq, IfACmpNe, IfEq,
            IfGe, IfGt, IfICmpEq, IfICmpGe, IfICmpGt, IfICmpLe, IfICmpLt, IfICmpNe, IfLe, IfLt,
            IfNe, IfNonNull, IfNull, Jsr, JsrW, LReturn, LookupSwitch, Ret, Return, TableSwitch,
            Wide,
        };
        if let Instruction::MultiANewArray(array_type, dimensions) = instruction
            && !valid_multi_array_dimensions(array_type, *dimensions)
        {
            return Err(ErrorKind::InvalidMultiArrayDimensions);
        }
        Ok(match instruction {
            Jsr(_) | JsrW(_) | Ret(_) | Wide(WideInstruction::Ret(_)) => {
                return Err(ErrorKind::UnsupportedLegacySubroutine);
            }
            IReturn | LReturn | FReturn | DReturn | AReturn | Return => Self::Return {
                exception_arms: Arc::clone(unwind),
            },
            AThrow => Self::Throw {
                exception_arms: Arc::clone(unwind),
            },
            Goto(target) | GotoW(target) => Self::Goto { target: *target },
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
            | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target)
            | IfACmpNe(target) | IfNull(target) | IfNonNull(target) => Self::Branch {
                taken: *target,
                otherwise: next.ok_or(ErrorKind::MissingFallthrough)?,
            },
            TableSwitch {
                low,
                jump_targets,
                default,
            } => {
                let high = Instruction::tableswitch_high(*low, jump_targets.len())
                    .ok_or(ErrorKind::InvalidTableSwitchRange)?;
                Self::Switch {
                    cases: (*low..=high).zip(jump_targets.iter().copied()).collect(),
                    default: *default,
                }
            }
            LookupSwitch {
                match_targets,
                default,
            } => Self::Switch {
                cases: match_targets.iter().copied().collect(),
                default: *default,
            },
            _ => Self::Continue {
                next: next.ok_or(ErrorKind::MissingFallthrough)?,
                exception_arms: Arc::clone(if fallthrough_may_throw(instruction) {
                    unwind
                } else {
                    empty
                }),
            },
        })
    }

    pub(super) fn collect_targets(&self, leaders: &mut Vec<ProgramCounter>) {
        match self {
            Self::Goto { target } | Self::Branch { taken: target, .. } => leaders.push(*target),
            Self::Switch { cases, default } => {
                leaders.extend(cases.values());
                leaders.push(*default);
            }
            Self::Continue { .. } | Self::Return { .. } | Self::Throw { .. } => {}
        }
    }
}

fn valid_multi_array_dimensions(array_type: &ReferenceType, dimensions: u8) -> bool {
    let ReferenceType::Array(element_type) = array_type else {
        return false;
    };
    let mut element_type = element_type.as_ref();
    if dimensions == 0 {
        return false;
    }
    for _ in 1..dimensions {
        let FieldType::Array(inner) = element_type else {
            return false;
        };
        element_type = inner.as_ref();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::test::prelude::*;

    fn classify(
        instruction: &Instruction<'_>,
        next: Option<ProgramCounter>,
    ) -> Result<BlockExit<'static, ProgramCounter>, ErrorKind> {
        BlockExit::of(
            instruction,
            next,
            &Arc::from([]),
            &Arc::from([ExceptionArm {
                target: ExceptionTarget::Unwind,
                catch_type: None,
            }]),
        )
    }

    #[test]
    fn switch_encodings_normalize_to_the_same_cases_and_default() {
        let targets = [10.into(), 20.into()];
        let pairs = [(3, targets[0]), (4, targets[1])];
        let table = Instruction::TableSwitch {
            low: 3,
            jump_targets: &targets,
            default: 30.into(),
        };
        let lookup = Instruction::LookupSwitch {
            match_targets: &pairs,
            default: 30.into(),
        };
        let expected = BlockExit::Switch {
            cases: BTreeMap::from(pairs),
            default: 30.into(),
        };
        assert_eq!(classify(&table, None).unwrap(), expected);
        assert_eq!(classify(&lookup, None).unwrap(), expected);
    }

    #[test]
    fn rejects_invalid_tableswitch_ranges() {
        for (low, jump_targets) in [(i32::MAX, vec![10.into(), 10.into()]), (0, vec![])] {
            let switch = InstructionData::TableSwitch {
                low,
                jump_targets: jump_targets.into_boxed_slice(),
                default: 10.into(),
            };
            let instructions = [(0, switch), (10, InstructionData::Return)];
            let method = method(instructions, "()V", vec![]);
            let body = method.as_method().body().unwrap();
            let instruction = body.instruction_at(0.into()).unwrap();
            assert_eq!(
                classify(&instruction, body.instructions().next_pc_of(&0.into())).map(|_| ()),
                Err(ErrorKind::InvalidTableSwitchRange)
            );
        }
    }

    #[test]
    fn rejects_invalid_multianewarray_dimensions() {
        for (array_type, dimensions) in [("[[I", 0), ("[I", 2), ("java/lang/Object", 1)] {
            let mut pool = ConstantPool::new();
            let array_type = pool.insert_reference_type(ref_t(array_type));
            let instructions = [
                (0, InstructionData::MultiANewArray(array_type, dimensions)),
                (1, InstructionData::Return),
            ];
            let method = method_with_pool(pool, instructions, "()V", vec![]);
            let body = method.as_method().body().unwrap();
            let instruction = body.instruction_at(0.into()).unwrap();
            assert_eq!(
                classify(&instruction, body.instructions().next_pc_of(&0.into())).map(|_| ()),
                Err(ErrorKind::InvalidMultiArrayDimensions)
            );
        }
    }

    #[test]
    fn rejects_legacy_subroutine_instructions() {
        let instructions = [
            InstructionData::Jsr(0.into()),
            InstructionData::JsrW(0.into()),
            InstructionData::Ret(0),
            InstructionData::Wide(WideInstruction::Ret(300)),
        ];
        for instruction in instructions {
            let method = method([(0, instruction)], "()V", vec![]);
            let body = method.as_method().body().unwrap();
            let view = body.instruction_at(0.into()).unwrap();
            assert_eq!(
                classify(&view, body.instructions().next_pc_of(&0.into())).map(|_| ()),
                Err(ErrorKind::UnsupportedLegacySubroutine)
            );
        }
    }

    #[test]
    fn rejects_missing_fallthrough() {
        assert_eq!(
            classify(&Instruction::Nop, None).map(|_| ()),
            Err(ErrorKind::MissingFallthrough)
        );
    }
}
