use crate::jvm::{ConstantValue, code::Instruction};

/// Returns whether executing `instruction` can synchronously transfer control
/// to a JVM exception handler.
///
/// This is deliberately an exhaustive opcode classification. It includes
/// resolution, initialization, allocation, and bootstrap failures in addition
/// to the instruction's most obvious runtime exception.
pub(super) const fn is_synchronously_fallible(instruction: &Instruction) -> bool {
    use Instruction::{
        AALoad, AAStore, ANewArray, AThrow, ArrayLength, BALoad, BAStore, CALoad, CAStore,
        CheckCast, DALoad, DAStore, FALoad, FAStore, GetField, GetStatic, IALoad, IAStore, IDiv,
        IRem, InstanceOf, InvokeDynamic, InvokeInterface, InvokeSpecial, InvokeStatic,
        InvokeVirtual, LALoad, LAStore, LDiv, LRem, Ldc, Ldc2W, LdcW, MonitorEnter, MonitorExit,
        MultiANewArray, New, NewArray, PutField, PutStatic, SALoad, SAStore,
    };

    match instruction {
        Ldc(value) | LdcW(value) | Ldc2W(value) => constant_resolution_is_fallible(value),
        IALoad
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_direct_runtime_failures() {
        assert!(is_synchronously_fallible(&Instruction::IALoad));
        assert!(is_synchronously_fallible(&Instruction::IDiv));
        assert!(is_synchronously_fallible(&Instruction::ArrayLength));
        assert!(is_synchronously_fallible(&Instruction::MonitorExit));
    }

    #[test]
    fn classifies_resolution_and_allocation_failures() {
        assert!(is_synchronously_fallible(&Instruction::Ldc(
            ConstantValue::Class("java/lang/String".parse().unwrap())
        )));
        assert!(is_synchronously_fallible(&Instruction::New(
            "java/lang/Object".parse().unwrap()
        )));
    }

    #[test]
    fn excludes_non_throwing_operations_and_primitive_constants() {
        assert!(!is_synchronously_fallible(&Instruction::IAdd));
        assert!(!is_synchronously_fallible(&Instruction::FDiv));
        assert!(!is_synchronously_fallible(&Instruction::ILoad0));
        assert!(!is_synchronously_fallible(&Instruction::Ldc(
            ConstantValue::Integer(1)
        )));
    }
}
