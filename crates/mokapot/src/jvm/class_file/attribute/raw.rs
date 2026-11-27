//! Raw attribute payloads.

use zerocopy::{
    FromBytes, Immutable, KnownLayout, TryFromBytes, Unaligned, byteorder::big_endian::U16,
};

use super::super::{
    Decode, ParseError, class::raw as class_raw, code::raw as code_raw, decoder::try_borrow_view,
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{ConstantPoolIndex, constant_pool::OptionalConstantPoolIndex},
};

macro_rules! impl_decode {
    ($name:ident, $count:ty) => {
        impl<'a> Decode<'a> for &'a $name {
            fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
                let mut header = *bytes;
                let count = <$count>::decode(&mut header)?;
                try_borrow_view(bytes, usize::from(count))
            }
        }
    };
}

/// Shared payload of `Exceptions`, `ModulePackages`, `NestMembers` and `PermittedSubclasses`.
#[doc = see_jvm_spec!(4, 7, 5)]
#[doc = see_jvm_spec!(4, 7, 26)]
#[doc = see_jvm_spec!(4, 7, 29)]
#[doc = see_jvm_spec!(4, 7, 31)]
#[derive(TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ConstantPoolIndices {
    pub count: U16,
    pub indices: [ConstantPoolIndex],
}

impl_decode!(ConstantPoolIndices, u16);

#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct InnerClasses {
    pub number_of_classes: U16,
    pub classes: [class_raw::InnerClass],
}

impl_decode!(InnerClasses, u16);

#[doc = see_jvm_spec!(4, 7, 12)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LineNumberTable {
    pub line_number_table_length: U16,
    pub line_number_table: [code_raw::LineNumberTableEntry],
}

impl_decode!(LineNumberTable, u16);

#[doc = see_jvm_spec!(4, 7, 13)]
#[derive(TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct LocalVariableTable {
    pub local_variable_table_length: U16,
    pub local_variable_table: [code_raw::LocalVariableInfo],
}

impl_decode!(LocalVariableTable, u16);

#[doc = see_jvm_spec!(4, 7, 14)]
#[derive(TryFromBytes, KnownLayout, Immutable, Unaligned)]
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
    pub name: OptionalConstantPoolIndex,
    pub access_flags: U16,
}

#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct MethodParameters {
    pub parameters_count: u8,
    pub parameters: [MethodParameterInfo],
}

impl_decode!(MethodParameters, u8);
