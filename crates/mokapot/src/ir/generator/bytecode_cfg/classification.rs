//! Structural classification of decoded JVM instructions.

use std::collections::BTreeMap;

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        ConstantValue,
        code::{Instruction, ProgramCounter, WideInstruction},
    },
};

/// All information about an instruction needed to construct bytecode blocks.
///
/// Opcode execution remains in `bytecode_analysis::lifting`; this description
/// contains only topology that can be known before frame propagation.
#[derive(Debug, Clone)]
pub(super) struct InstructionDescription {
    pub control_flow: ControlFlow,
    pub can_throw: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum ControlFlow {
    Fallthrough,
    Goto(ProgramCounter),
    Branch(ProgramCounter),
    Switch(BTreeMap<i32, ProgramCounter>, ProgramCounter),
    Terminal,
    Legacy,
}

pub(super) fn describe(instruction: &Instruction) -> InstructionDescription {
    InstructionDescription {
        control_flow: control_flow(instruction),
        can_throw: can_throw(instruction),
    }
}

pub(crate) fn control_flow(instruction: &Instruction) -> ControlFlow {
    use Instruction::{
        AReturn, AThrow, DReturn, FReturn, Goto, GotoW, IReturn, IfACmpEq, IfACmpNe, IfEq, IfGe,
        IfGt, IfICmpEq, IfICmpGe, IfICmpGt, IfICmpLe, IfICmpLt, IfICmpNe, IfLe, IfLt, IfNe,
        IfNonNull, IfNull, Jsr, JsrW, LReturn, Ret, Return, Wide,
    };
    match instruction {
        IReturn | LReturn | FReturn | DReturn | AReturn | Return | AThrow => ControlFlow::Terminal,
        Goto(target) | GotoW(target) => ControlFlow::Goto(*target),
        IfEq(pc) | IfNe(pc) | IfLt(pc) | IfGe(pc) | IfGt(pc) | IfLe(pc) | IfICmpEq(pc)
        | IfICmpNe(pc) | IfICmpLt(pc) | IfICmpGe(pc) | IfICmpGt(pc) | IfICmpLe(pc)
        | IfACmpEq(pc) | IfACmpNe(pc) | IfNull(pc) | IfNonNull(pc) => ControlFlow::Branch(*pc),
        Jsr(_) | JsrW(_) | Ret(_) | Wide(WideInstruction::Ret(_)) => ControlFlow::Legacy,
        Instruction::TableSwitch {
            jump_targets,
            default,
            range,
        } => ControlFlow::Switch(range.clone().zip(jump_targets.clone()).collect(), *default),
        Instruction::LookupSwitch {
            default,
            match_targets,
        } => ControlFlow::Switch(match_targets.clone(), *default),
        _ => ControlFlow::Fallthrough,
    }
}

/// Returns whether executing `instruction` can synchronously transfer control
/// to a JVM exception handler.
///
/// This is deliberately an exhaustive opcode classification. It includes
/// resolution, initialization, allocation, bootstrap, and method-exit failures
/// in addition to the instruction's most obvious runtime exception. Returns are
/// conservatively fallible because a JVM may enforce structured locking.
const fn can_throw(instruction: &Instruction) -> bool {
    use Instruction::{
        AALoad, AAStore, ANewArray, AReturn, AThrow, ArrayLength, BALoad, BAStore, CALoad, CAStore,
        CheckCast, DALoad, DAStore, DReturn, FALoad, FAStore, FReturn, GetField, GetStatic, IALoad,
        IAStore, IDiv, IRem, IReturn, InstanceOf, InvokeDynamic, InvokeInterface, InvokeSpecial,
        InvokeStatic, InvokeVirtual, LALoad, LAStore, LDiv, LRem, LReturn, Ldc, Ldc2W, LdcW,
        MonitorEnter, MonitorExit, MultiANewArray, New, NewArray, PutField, PutStatic, Return,
        SALoad, SAStore,
    };

    match instruction {
        Ldc(value) | LdcW(value) | Ldc2W(value) => constant_resolution_is_fallible(value),
        IReturn
        | LReturn
        | FReturn
        | DReturn
        | AReturn
        | Return
        | IALoad
        | LALoad
        | FALoad
        | DALoad
        | AALoad
        | BALoad
        | CALoad
        | SALoad
        | IAStore
        | LAStore
        | FAStore
        | DAStore
        | AAStore
        | BAStore
        | CAStore
        | SAStore
        | IDiv
        | LDiv
        | IRem
        | LRem
        | GetStatic(_)
        | PutStatic(_)
        | GetField(_)
        | PutField(_)
        | InvokeVirtual(_)
        | InvokeSpecial(_)
        | InvokeStatic(_)
        | InvokeInterface(_, _)
        | InvokeDynamic { .. }
        | New(_)
        | NewArray(_)
        | ANewArray(_)
        | ArrayLength
        | AThrow
        | CheckCast(_)
        | InstanceOf(_)
        | MonitorEnter
        | MonitorExit
        | MultiANewArray(_, _) => true,
        _ => false,
    }
}

/// Whether loading `value` with `ldc`, `ldc_w`, or `ldc2_w` can fail
/// synchronously.
///
/// Numeric constants are read straight out of the run-time constant pool, and
/// `Null` never reaches it (`aconst_null` pushes it instead), so neither can
/// fail. Every other entry may require resolution or materialization.
#[doc = see_jvm_spec!(6, 5)]
#[doc = see_jvm_spec!(5, 4, 3)]
const fn constant_resolution_is_fallible(value: &ConstantValue) -> bool {
    !matches!(
        value,
        ConstantValue::Null
            | ConstantValue::Integer(_)
            | ConstantValue::Float(_)
            | ConstantValue::Long(_)
            | ConstantValue::Double(_)
    )
}
