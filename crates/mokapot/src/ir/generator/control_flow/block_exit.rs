//! Structural control flow of one decoded JVM instruction.

use std::collections::BTreeMap;

use super::{Error, fallibility::fallthrough_may_throw};
use crate::{
    ir::generator::error::{MalformedControlFlow, UnsupportedBytecode},
    jvm::{
        code::{Instruction, MethodBody, ProgramCounter, WideInstruction},
        references::ClassRef,
    },
    types::{field_type::FieldType, reference_type::ReferenceType},
};

/// The structural exit of one decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockExit<T> {
    Continue {
        next: T,
        exception_arms: Vec<ExceptionArm<T>>,
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
        exception_arms: Vec<ExceptionArm<T>>,
    },
    Throw {
        exception_arms: Vec<ExceptionArm<T>>,
    },
}

impl<T> BlockExit<T> {
    /// Whether this exit forces the following instruction into a new block.
    ///
    /// A continuation without exception arms proceeds into the next instruction
    /// without its own terminator.
    pub(crate) const fn forces_block_boundary(&self) -> bool {
        !matches!(self, Self::Continue { exception_arms, .. } if exception_arms.is_empty())
    }
}

/// One ordered exception arm of a throwing instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExceptionArm<T> {
    /// Where the exception transfers.
    pub target: ExceptionTarget<T>,
    /// The caught type, or `None` for catch-all.
    pub catch_type: Option<ClassRef>,
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

impl BlockExit<ProgramCounter> {
    /// Classifies the control transfer of `instruction` at `pc` within `body`.
    pub(super) fn of(
        body: &MethodBody,
        pc: ProgramCounter,
        instruction: &Instruction,
    ) -> Result<Self, Error> {
        use Instruction::{
            AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, IfACmpEq, IfACmpNe, IfEq,
            IfGe, IfGt, IfICmpEq, IfICmpGe, IfICmpGt, IfICmpLe, IfICmpLt, IfICmpNe, IfLe, IfLt,
            IfNe, IfNonNull, IfNull, Jsr, JsrW, LReturn, Ret, Return, Wide,
        };
        let next = || {
            body.instructions
                .next_pc_of(&pc)
                .ok_or(MalformedControlFlow::MissingFallthrough(pc))
        };
        if let Instruction::MultiANewArray(array_type, dimensions) = instruction
            && !valid_multi_array_dimensions(array_type, *dimensions)
        {
            return Err(MalformedControlFlow::InvalidMultiArrayDimensions(pc).into());
        }
        Ok(match instruction {
            IReturn | LReturn | FReturn | DReturn | AReturn | Return => Self::Return {
                exception_arms: exception_arms(body, pc),
            },
            AThrow => Self::Throw {
                exception_arms: exception_arms(body, pc),
            },
            Goto(target) | GotoW(target) => Self::Goto { target: *target },
            IfEq(pc) | IfNe(pc) | IfLt(pc) | IfGe(pc) | IfGt(pc) | IfLe(pc) | IfICmpEq(pc)
            | IfICmpNe(pc) | IfICmpLt(pc) | IfICmpGe(pc) | IfICmpGt(pc) | IfICmpLe(pc)
            | IfACmpEq(pc) | IfACmpNe(pc) | IfNull(pc) | IfNonNull(pc) => Self::Branch {
                taken: *pc,
                otherwise: next()?,
            },
            Jsr(_) | JsrW(_) | Ret(_) | Wide(WideInstruction::Ret(_)) => {
                let kind = UnsupportedBytecode::LegacySubroutine;
                return Err(Error::UnsupportedBytecode { pc, kind });
            }
            Instruction::TableSwitch {
                low,
                jump_targets: targets,
                default,
            } => {
                let high = Instruction::tableswitch_high(*low, targets.len())
                    .ok_or(MalformedControlFlow::InvalidTableSwitchRange(pc))?;
                let cases = (*low..=high).zip(targets.iter().copied()).collect();
                Self::Switch {
                    cases,
                    default: *default,
                }
            }
            Instruction::LookupSwitch {
                match_targets,
                default,
            } => Self::Switch {
                cases: match_targets.clone(),
                default: *default,
            },
            _ => Self::Continue {
                next: next()?,
                exception_arms: if fallthrough_may_throw(instruction) {
                    exception_arms(body, pc)
                } else {
                    Vec::new()
                },
            },
        })
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

/// The ordered exception arms of the instruction at `pc`.
///
/// Selection walks the table in order, so the first catch-all shadows later
/// entries and an escaping exception unwinds.
fn exception_arms(body: &MethodBody, pc: ProgramCounter) -> Vec<ExceptionArm<ProgramCounter>> {
    let mut exception_arms = Vec::new();
    for entry in body.exception_table.iter().filter(|entry| entry.covers(pc)) {
        exception_arms.push(ExceptionArm {
            target: ExceptionTarget::Handler(entry.handler_pc),
            catch_type: entry.catch_type.clone(),
        });
        if entry.catches_all() {
            return exception_arms;
        }
    }
    exception_arms.push(ExceptionArm {
        target: ExceptionTarget::Unwind,
        catch_type: None,
    });
    exception_arms
}
