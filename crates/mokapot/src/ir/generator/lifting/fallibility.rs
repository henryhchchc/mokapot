//! JVM instruction fallibility classification.

use crate::jvm::{ConstantValue, Method, code::Instruction, method};

/// Method-level context needed to classify instruction fallibility.
#[derive(Debug, Clone, Copy)]
pub(in crate::ir::generator) struct FallibilityContext {
    return_can_throw: bool,
}

impl FallibilityContext {
    pub(in crate::ir::generator) fn for_method(method: &Method) -> Self {
        // Exact structured-locking analysis is path- and alias-sensitive, so
        // explicit monitor use conservatively makes every method exit fallible.
        let has_explicit_monitor_operation = method.body.as_ref().is_some_and(|body| {
            body.instructions.iter().any(|(_, instruction)| {
                matches!(
                    instruction,
                    Instruction::MonitorEnter | Instruction::MonitorExit
                )
            })
        });
        Self {
            return_can_throw: method
                .access_flags
                .contains(method::AccessFlags::SYNCHRONIZED)
                || has_explicit_monitor_operation,
        }
    }

    /// Returns whether executing `instruction` can synchronously transfer
    /// control to a JVM exception handler.
    ///
    /// This is deliberately an exhaustive opcode classification. It includes
    /// resolution, initialization, allocation, bootstrap, and method-exit
    /// failures in addition to the instruction's most obvious runtime exception.
    pub(crate) const fn is_synchronously_fallible(self, instruction: &Instruction) -> bool {
        use Instruction::{
            AALoad, AAStore, ANewArray, AReturn, AThrow, ArrayLength, BALoad, BAStore, CALoad,
            CAStore, CheckCast, DALoad, DAStore, DReturn, FALoad, FAStore, FReturn, GetField,
            GetStatic, IALoad, IAStore, IDiv, IRem, IReturn, InstanceOf, InvokeDynamic,
            InvokeInterface, InvokeSpecial, InvokeStatic, InvokeVirtual, LALoad, LAStore, LDiv,
            LRem, LReturn, Ldc, Ldc2W, LdcW, MonitorEnter, MonitorExit, MultiANewArray, New,
            NewArray, PutField, PutStatic, Return, SALoad, SAStore,
        };

        match instruction {
            Ldc(value) | LdcW(value) | Ldc2W(value) => constant_resolution_is_fallible(value),
            IReturn | LReturn | FReturn | DReturn | AReturn | Return => self.return_can_throw,
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

    const ORDINARY: FallibilityContext = FallibilityContext {
        return_can_throw: false,
    };
    const FALLIBLE_RETURN: FallibilityContext = FallibilityContext {
        return_can_throw: true,
    };

    #[test]
    fn classifies_direct_runtime_failures() {
        assert!(ORDINARY.is_synchronously_fallible(&Instruction::IALoad));
        assert!(ORDINARY.is_synchronously_fallible(&Instruction::IDiv));
        assert!(ORDINARY.is_synchronously_fallible(&Instruction::ArrayLength));
        assert!(ORDINARY.is_synchronously_fallible(&Instruction::MonitorExit));
    }

    #[test]
    fn classifies_resolution_and_allocation_failures() {
        assert!(
            ORDINARY.is_synchronously_fallible(&Instruction::Ldc(ConstantValue::Class(
                "java/lang/String".parse().unwrap()
            )))
        );
        assert!(
            ORDINARY
                .is_synchronously_fallible(&Instruction::New("java/lang/Object".parse().unwrap()))
        );
    }

    #[test]
    fn classifies_returns_from_method_context() {
        let returns = [
            Instruction::IReturn,
            Instruction::LReturn,
            Instruction::FReturn,
            Instruction::DReturn,
            Instruction::AReturn,
            Instruction::Return,
        ];

        assert!(
            returns
                .iter()
                .all(|instruction| FALLIBLE_RETURN.is_synchronously_fallible(instruction))
        );
        assert!(
            returns
                .iter()
                .all(|instruction| !ORDINARY.is_synchronously_fallible(instruction))
        );
    }

    #[test]
    fn excludes_non_throwing_operations_and_primitive_constants() {
        assert!(!ORDINARY.is_synchronously_fallible(&Instruction::IAdd));
        assert!(!ORDINARY.is_synchronously_fallible(&Instruction::FDiv));
        assert!(!ORDINARY.is_synchronously_fallible(&Instruction::ILoad0));
        assert!(!ORDINARY.is_synchronously_fallible(&Instruction::Ldc(ConstantValue::Integer(1))));
    }
}
