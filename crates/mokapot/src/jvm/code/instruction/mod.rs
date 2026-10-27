//! Resolved JVM instruction model and presentation.

use derive_more::Display;

mod display;
mod opcodes;

use crate::types::class_name::ClassName;

use super::ProgramCounter;
use crate::{
    intrinsics::{enum_discriminant, see_jvm_spec},
    jvm::{
        ConstantPool, ConstantVal, JavaStr,
        constant_pool::{
            ClassId, ConstantValueId, FieldRefId, InvokeDynamicId, MethodRefId, ReferenceTypeId,
        },
        references::{FieldRef, MethodRef},
    },
    types::{
        field_type::PrimitiveType, method_descriptor::MethodDescriptor,
        reference_type::ReferenceType,
    },
};

macro_rules! define_instructions {
    (
        $class:lifetime, $pool:ident;
        unit { $($unit:ident = $unit_opcode:literal, $unit_name:literal;)* }
        copy {
            $($copy:ident($($copy_arg:ident: $copy_type:ty),+) = $copy_opcode:literal, $copy_name:literal;)*
        }
        pool {
            $($pooled:ident($($pool_arg:ident: $stored_type:ty => $view_type:ty = $view:expr),+)
                = $pool_opcode:literal, $pool_name:literal;)*
        }
        structured {
            $($structured:ident {
                $($(#[$field_doc:meta])* $field:ident: $stored_field_type:ty => $view_field_type:ty = $field_view:expr,)*
            } = $structured_opcode:literal, $structured_name:literal;)*
        }
        custom {
            $($custom:ident($($custom_arg:ident: $custom_type:ty),+) => {
                $($custom_field:ident: $custom_field_type:ty,)*
            } = $custom_opcode:literal, $custom_name:literal $custom_view:block)*
        }
    ) => {
        #[derive(Debug, Clone, PartialEq)]
        #[repr(u8)]
        pub(crate) enum InstructionData {
            $($unit = $unit_opcode,)*
            $($copy($($copy_type),+) = $copy_opcode,)*
            $($pooled($($stored_type),+) = $pool_opcode,)*
            $($structured { $($field: $stored_field_type,)* } = $structured_opcode,)*
            $($custom($($custom_type),+) = $custom_opcode,)*
        }

        /// A JVM instruction borrowing its class.
        #[doc = see_jvm_spec!(6, 5)]
        #[derive(Debug, PartialEq, Clone, Copy)]
        #[allow(missing_docs, reason = "Variants map one-to-one to JVM instructions")]
        #[repr(u8)]
        pub enum Instruction<$class> {
            $($unit = $unit_opcode,)*
            $($copy($($copy_type),+) = $copy_opcode,)*
            $($pooled($($view_type),+) = $pool_opcode,)*
            $($structured {
                $($(#[$field_doc])* $field: $view_field_type,)*
            } = $structured_opcode,)*
            $($custom { $($custom_field: $custom_field_type,)* } = $custom_opcode,)*
        }

        impl InstructionData {
            pub(crate) fn as_borrowed<$class>(&$class self, $pool: &$class ConstantPool) -> Instruction<$class> {
                match self {
                    $(Self::$unit => Instruction::$unit,)*
                    $(Self::$copy($($copy_arg),+) => Instruction::$copy($(*$copy_arg),+),)*
                    $(Self::$pooled($($pool_arg),+) => Instruction::$pooled($($view),+),)*
                    $(Self::$structured { $($field,)* } => Instruction::$structured { $($field: $field_view,)* },)*
                    $(Self::$custom($($custom_arg),+) => $custom_view,)*
                }
            }
        }

        impl Instruction<'_> {
            /// Gets the name of the instruction.
            #[must_use]
            pub const fn name<'a>(&self) -> &'a str {
                match self {
                    $(Self::$unit => $unit_name,)*
                    $(Self::$copy(..) => $copy_name,)*
                    $(Self::$pooled(..) => $pool_name,)*
                    $(Self::$structured { .. } => $structured_name,)*
                    $(Self::$custom { .. } => $custom_name,)*
                }
            }
        }
    };
}

opcodes::for_each_instruction!(define_instructions, 'class, pool);

/// A wide instruction.
#[doc = see_jvm_spec!(6, 5)]
#[allow(
    missing_docs,
    reason = "This maps one-to-one to those in the JVM spec."
)]
#[allow(
    clippy::module_name_repetitions,
    reason = "For consistent type names with the JVM spec."
)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Display)]
pub enum WideInstruction {
    #[display("iload {_0}")]
    ILoad(u16),
    #[display("lload {_0}")]
    LLoad(u16),
    #[display("fload {_0}")]
    FLoad(u16),
    #[display("dload {_0}")]
    DLoad(u16),
    #[display("aload {_0}")]
    ALoad(u16),
    #[display("istore {_0}")]
    IStore(u16),
    #[display("lstore {_0}")]
    LStore(u16),
    #[display("fstore {_0}")]
    FStore(u16),
    #[display("dstore {_0}")]
    DStore(u16),
    #[display("astore {_0}")]
    AStore(u16),
    #[display("iinc {_0} {_1}")]
    IInc(u16, i32),
    #[display("ret {_0}")]
    Ret(u16),
}

impl Instruction<'_> {
    pub(crate) fn tableswitch_high(low: i32, target_count: usize) -> Option<i32> {
        let offset = i128::try_from(target_count.checked_sub(1)?).ok()?;
        i32::try_from(i128::from(low) + offset).ok()
    }

    /// Gets the opcode.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        // SAFETY: Self is repr(u8) so it should be fine
        unsafe { enum_discriminant(self) }
    }
}

#[cfg(test)]
mod tests;
