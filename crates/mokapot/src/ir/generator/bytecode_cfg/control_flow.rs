//! Structural control flow of one decoded JVM instruction.

use std::collections::BTreeMap;

use itertools::Itertools;

use crate::{
    intrinsics::see_jvm_spec,
    ir::generator::error::{Error, MalformedBytecode, UnsupportedBytecode},
    jvm::{
        ConstantValue,
        code::{Instruction, MethodBody, ProgramCounter, WideInstruction},
        references::ClassRef,
    },
};

/// The structural control transfer of one decoded instruction.
///
/// Targets are generic so bytecode locations resolve to block identities
/// without changing the shape of the transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlFlow<T> {
    /// Execution continues at `next`; `handlers` are the exception outcomes,
    /// empty when the instruction cannot throw.
    Fallthrough {
        /// The next instruction.
        next: T,
        /// The exception outcomes, empty when the instruction cannot throw.
        handlers: Vec<Handler<T>>,
    },
    /// Execution continues at `target`.
    Goto {
        /// The destination.
        target: T,
    },
    /// Execution continues at `taken` or at `otherwise`.
    Branch {
        /// The taken destination.
        taken: T,
        /// The not-taken destination.
        otherwise: T,
    },
    /// Execution continues at the indexed destination or at `default`.
    Switch {
        /// The destination of each case, keyed by its matched value.
        cases: BTreeMap<i32, T>,
        /// The destination when no case matches.
        default: T,
    },
    /// Execution leaves the method normally.
    Return {
        /// The exception outcomes.
        handlers: Vec<Handler<T>>,
    },
    /// Execution leaves the method abruptly.
    Throw {
        /// The exception outcomes.
        handlers: Vec<Handler<T>>,
    },
}

impl<T> ControlFlow<T> {
    /// Whether the transfer may reach a JVM exception handler.
    pub(crate) const fn may_throw(&self) -> bool {
        match self {
            Self::Fallthrough { handlers, .. }
            | Self::Return { handlers }
            | Self::Throw { handlers } => !handlers.is_empty(),
            Self::Goto { .. } | Self::Branch { .. } | Self::Switch { .. } => false,
        }
    }

    /// Whether this transfer is the source of a terminator.
    ///
    /// A fallthrough without handlers continues into the next instruction, so
    /// the block has no terminator of its own.
    pub(crate) const fn has_terminator_source(&self) -> bool {
        match self {
            Self::Fallthrough { handlers, .. } => !handlers.is_empty(),
            _ => true,
        }
    }
}

/// One exception outcome of a throwing instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Handler<T> {
    /// Where the exception transfers.
    pub target: Target<T>,
    /// The caught type, or `None` for catch-all.
    pub catch: Option<ClassRef>,
}

/// A successor destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target<T> {
    /// An internal block.
    Block(T),
    /// The synthetic exit for an exception that escapes the method.
    Unwind,
}

/// The structural identity of one outgoing arm within its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ArmId {
    /// The ordinary continuation of a fallthrough.
    Fallthrough,
    /// The destination of an unconditional jump.
    Goto,
    /// The taken destination of a conditional branch.
    Taken,
    /// The not-taken destination of a conditional branch.
    Otherwise,
    /// The destination of one switch case, keyed by its matched value.
    Case(i32),
    /// The destination of a switch when no case matches.
    Default,
    /// The handler at the given position in its source's handler list.
    Handler(usize),
}

impl ControlFlow<ProgramCounter> {
    /// Classifies the control transfer of `instruction` at `pc` within `body`.
    pub(crate) fn of(
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
                .ok_or_else(|| Error::malformed(Some(pc), MalformedBytecode::MissingFallthrough))
        };
        Ok(match instruction {
            IReturn | LReturn | FReturn | DReturn | AReturn | Return => Self::Return {
                handlers: handlers(body, pc),
            },
            AThrow => Self::Throw {
                handlers: handlers(body, pc),
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
                jump_targets,
                default,
                range,
            } => Self::Switch {
                cases: range.clone().zip_eq(jump_targets.clone()).collect(),
                default: *default,
            },
            Instruction::LookupSwitch {
                default,
                match_targets,
            } => Self::Switch {
                cases: match_targets.clone(),
                default: *default,
            },
            _ => Self::Fallthrough {
                next: next()?,
                handlers: if can_throw(instruction) {
                    handlers(body, pc)
                } else {
                    Vec::new()
                },
            },
        })
    }
}

/// The exception outcomes of the instruction at `pc`.
///
/// Handler selection walks the table in order, so the first catch-all shadows
/// later entries. An escaping exception unwinds unless a catch-all applies.
fn handlers(body: &MethodBody, pc: ProgramCounter) -> Vec<Handler<ProgramCounter>> {
    let effective: Vec<_> = body
        .exception_table
        .iter()
        .filter(|entry| entry.covers(pc))
        .take_while_inclusive(|entry| !entry.catches_all())
        .collect();
    let unwind = effective.last().is_none_or(|entry| !entry.catches_all());
    let mut handlers: Vec<_> = effective
        .into_iter()
        .map(|entry| Handler {
            target: Target::Block(entry.handler_pc),
            catch: entry.catch_type.clone(),
        })
        .collect();
    if unwind {
        handlers.push(Handler {
            target: Target::Unwind,
            catch: None,
        });
    }
    handlers
}

/// Returns whether executing `instruction` can synchronously transfer control
/// to a JVM exception handler.
///
/// This is deliberately an exhaustive opcode classification. It includes
/// resolution, initialization, allocation, bootstrap, and method-exit failures
/// in addition to the instruction's most obvious runtime exception. Returns are
/// conservatively fallible because a JVM may enforce structured locking.
///
/// Returns and `athrow` are classified before this is consulted, so their
/// entries below merely restate that every method exit is fallible.
const fn can_throw(instruction: &Instruction) -> bool {
    use Instruction::{
        AALoad, AAStore, ANewArray, AReturn, AThrow, ArrayLength, BALoad, BAStore, CALoad, CAStore,
        CheckCast, DALoad, DAStore, DReturn, FALoad, FAStore, FReturn, GetField, GetStatic, IALoad,
        IAStore, IDiv, IRem, IReturn, InstanceOf, InvokeDynamic, InvokeInterface, InvokeSpecial,
        InvokeStatic, InvokeVirtual, LALoad, LAStore, LDiv, LRem, LReturn, Ldc, Ldc2W, LdcW,
        MonitorEnter, MonitorExit, MultiANewArray, New, NewArray, PutField, PutStatic, Return,
        SALoad, SAStore,
    };

    #[expect(clippy::match_same_arms, reason = "group by category")]
    match instruction {
        IReturn | LReturn | FReturn | DReturn | AReturn | Return => true,
        IALoad | LALoad | FALoad | DALoad | AALoad | BALoad | CALoad | SALoad => true,
        IAStore | LAStore | FAStore | DAStore | AAStore | BAStore | CAStore | SAStore => true,
        IDiv | LDiv | IRem | LRem => true,
        GetStatic(_) | PutStatic(_) | GetField(_) | PutField(_) => true,
        New(_) | NewArray(_) | ANewArray(_) | ArrayLength | MultiANewArray(_, _) => true,
        AThrow => true,
        CheckCast(_) | InstanceOf(_) => true,
        MonitorEnter | MonitorExit => true,
        Ldc(val) | LdcW(val) | Ldc2W(val) => can_const_resolution_fall(val),
        InvokeVirtual(_)
        | InvokeSpecial(_)
        | InvokeStatic(_)
        | InvokeInterface(_, _)
        | InvokeDynamic { .. } => true,
        _ => false,
    }
}

/// Whether loading `value` with `ldc`, `ldc_w`, or `ldc2_w` can fail.
///
/// Numeric constants are read straight out of the run-time constant pool, and
/// `Null` never reaches it (`aconst_null` pushes it instead), so neither can
/// fail. Every other entry may require resolution or materialization.
#[doc = see_jvm_spec!(6, 5)]
#[doc = see_jvm_spec!(5, 4, 3)]
const fn can_const_resolution_fall(value: &ConstantValue) -> bool {
    use ConstantValue::{Double, Float, Integer, Long, Null};
    !matches!(value, Null | Integer(_) | Float(_) | Long(_) | Double(_))
}
