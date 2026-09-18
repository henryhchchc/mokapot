//! Classifies the fallibility of JVM instructions.

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{ConstantValue, code::Instruction},
};

/// Returns whether executing `instruction` can synchronously transfer control
/// to a JVM exception handler.
///
/// This is deliberately an exhaustive opcode classification. It includes
/// resolution, initialization, allocation, bootstrap, and method-exit failures
/// in addition to the instruction's most obvious runtime exception. Returns are
/// conservatively fallible because a JVM may enforce structured locking.
pub(super) const fn can_throw(instruction: &Instruction) -> bool {
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
/// fail.
#[doc = see_jvm_spec!(6, 5)]
///
/// Every other entry must be resolved before use, and
/// resolution can fail: class, method handle, and method type references may
/// throw any `LinkageError`, while a dynamically-computed constant additionally
/// invokes its bootstrap method, which may throw any `Throwable`. String
/// constants need no resolution, but the virtual machine must still materialize
/// the interned instance, so they are conservatively treated as fallible too.
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
