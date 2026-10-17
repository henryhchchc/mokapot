//! Raw attribute payloads.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError, class::raw as class_raw, code::raw as code_raw, decoder::decode_value,
    fixed_layout::FixedLayout,
};
use crate::intrinsics::see_jvm_spec;

macro_rules! impl_decode {
    ($name:ident, $count:ty) => {
        impl<'a> Decode<'a> for &'a $name {
            fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
                let mut header = *bytes;
                let count: $count = decode_value(&mut header)?;
                let (attribute, remaining) =
                    $name::ref_from_prefix_with_elems(bytes, usize::from(count))
                        .map_err(|_| ParseError::malform("Truncated counted attribute"))?;
                *bytes = remaining;
                Ok(attribute)
            }
        }
    };
}

#[doc = see_jvm_spec!(4, 7, 2)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ConstantValue {
    pub constantvalue_index: U16,
}

impl FixedLayout for ConstantValue {}

#[doc = see_jvm_spec!(4, 7, 5)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct Exceptions {
    pub number_of_exceptions: U16,
    pub exception_index_table: [U16],
}

impl_decode!(Exceptions, u16);

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct InnerClasses {
    pub number_of_classes: U16,
    pub classes: [class_raw::InnerClass],
}

impl_decode!(InnerClasses, u16);

#[doc = see_jvm_spec!(4, 7, 9)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct Signature {
    pub signature_index: U16,
}

impl FixedLayout for Signature {}

#[doc = see_jvm_spec!(4, 7, 10)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct SourceFile {
    pub sourcefile_index: U16,
}

impl FixedLayout for SourceFile {}

#[doc = see_jvm_spec!(4, 7, 12)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LineNumberTable {
    pub line_number_table_length: U16,
    pub line_number_table: [code_raw::LineNumberTableEntry],
}

impl_decode!(LineNumberTable, u16);

#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableTable {
    pub local_variable_table_length: U16,
    pub local_variable_table: [code_raw::LocalVariableInfo],
}

impl_decode!(LocalVariableTable, u16);

#[doc = see_jvm_spec!(4, 7, 14)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableTypeTable {
    pub local_variable_type_table_length: U16,
    pub local_variable_type_table: [code_raw::LocalVariableTypeInfo],
}

impl_decode!(LocalVariableTypeTable, u16);

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct MethodParameterInfo {
    pub name_index: U16,
    pub access_flags: U16,
}

impl FixedLayout for MethodParameterInfo {}

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct MethodParameters {
    pub parameters_count: u8,
    pub parameters: [MethodParameterInfo],
}

impl_decode!(MethodParameters, u8);

#[doc = see_jvm_spec!(4, 7, 26)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ModulePackages {
    pub package_count: U16,
    pub package_index: [U16],
}

impl_decode!(ModulePackages, u16);

#[doc = see_jvm_spec!(4, 7, 27)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ModuleMainClass {
    pub main_class_index: U16,
}

impl FixedLayout for ModuleMainClass {}

#[doc = see_jvm_spec!(4, 7, 28)]
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct NestHost {
    pub host_class_index: U16,
}

impl FixedLayout for NestHost {}

#[doc = see_jvm_spec!(4, 7, 29)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct NestMembers {
    pub number_of_classes: U16,
    pub classes: [U16],
}

impl_decode!(NestMembers, u16);

#[doc = see_jvm_spec!(4, 7, 31)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct PermittedSubclasses {
    pub number_of_classes: U16,
    pub classes: [U16],
}

impl_decode!(PermittedSubclasses, u16);
