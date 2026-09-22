//! Whether executing one decoded JVM instruction can fail.

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{ConstantValue, code::Instruction},
};

/// Returns whether a continuing instruction can synchronously transfer
/// control to a JVM exception handler.
pub(super) const fn fallthrough_may_throw(instruction: &Instruction) -> bool {
    use Instruction::{
        AALoad, AAStore, ANewArray, ArrayLength, BALoad, BAStore, CALoad, CAStore, CheckCast,
        DALoad, DAStore, FALoad, FAStore, GetField, GetStatic, IALoad, IAStore, IDiv, IRem,
        InstanceOf, InvokeDynamic, InvokeInterface, InvokeSpecial, InvokeStatic, InvokeVirtual,
        LALoad, LAStore, LDiv, LRem, Ldc, Ldc2W, LdcW, MonitorEnter, MonitorExit, MultiANewArray,
        New, NewArray, PutField, PutStatic, SALoad, SAStore,
    };

    #[expect(clippy::match_same_arms, reason = "group by category")]
    match instruction {
        IALoad | LALoad | FALoad | DALoad | AALoad | BALoad | CALoad | SALoad => true,
        IAStore | LAStore | FAStore | DAStore | AAStore | BAStore | CAStore | SAStore => true,
        IDiv | LDiv | IRem | LRem => true,
        GetStatic(_) | PutStatic(_) | GetField(_) | PutField(_) => true,
        New(_) | NewArray(_) | ANewArray(_) | ArrayLength | MultiANewArray(_, _) => true,
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
///
/// The constants that cannot fail are named, so a newly added constant defaults
/// to fallible.
#[doc = see_jvm_spec!(6, 5)]
#[doc = see_jvm_spec!(5, 4, 3)]
const fn can_const_resolution_fall(value: &ConstantValue) -> bool {
    use ConstantValue::{Double, Float, Integer, Long, Null};
    !matches!(value, Null | Integer(_) | Float(_) | Long(_) | Double(_))
}
