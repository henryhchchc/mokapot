//! Raw attribute types for module-related attributes.

use zerocopy::{Immutable, KnownLayout, TryFromBytes, Unaligned, byteorder::big_endian::U16};

use super::super::{
    Decode, ParseError,
    decoder::{parse_repeated, try_borrow_view},
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{ConstantPoolIndex, constant_pool::OptionalConstantPoolIndex},
};

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ModuleInfo<'a> {
    pub module_name: ConstantPoolIndex,
    pub module_flags: u16,
    pub module_version: Option<ConstantPoolIndex>,
    pub requires: &'a [RequiresInfo],
    pub exports: Box<[&'a ExportsInfo]>,
    pub opens: Box<[&'a OpensInfo]>,
    pub uses: &'a [ConstantPoolIndex],
    pub provides: Box<[&'a ProvidesInfo]>,
}

impl<'a> Decode<'a> for ModuleInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let module_name = Decode::decode(bytes)?;
        let module_flags = Decode::decode(bytes)?;
        let module_version = Decode::decode(bytes)?;
        let requires_count = u16::decode(bytes)?;
        let requires = try_borrow_view(bytes, usize::from(requires_count))?;
        let exports_count = u16::decode(bytes)?;
        let exports = parse_repeated(bytes, exports_count, <&ExportsInfo>::decode)?;
        let opens_count = u16::decode(bytes)?;
        let opens = parse_repeated(bytes, opens_count, <&OpensInfo>::decode)?;
        let uses_count = u16::decode(bytes)?;
        let uses = try_borrow_view(bytes, usize::from(uses_count))?;
        let provides_count = u16::decode(bytes)?;
        let provides = parse_repeated(bytes, provides_count, <&ProvidesInfo>::decode)?;
        Ok(Self {
            module_name,
            module_flags,
            module_version,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct RequiresInfo {
    pub module: ConstantPoolIndex,
    pub requires_flags: U16,
    pub requires_version: OptionalConstantPoolIndex,
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ExportsInfo {
    pub package: ConstantPoolIndex,
    pub exports_flags: U16,
    pub exports_to_count: U16,
    pub exports_to: [ConstantPoolIndex],
}

impl<'a> Decode<'a> for &'a ExportsInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        u16::decode(&mut header)?;
        u16::decode(&mut header)?;
        let count = u16::decode(&mut header)?;
        try_borrow_view(bytes, usize::from(count))
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct OpensInfo {
    pub package: ConstantPoolIndex,
    pub opens_flags: U16,
    pub opens_to_count: U16,
    pub opens_to: [ConstantPoolIndex],
}

impl<'a> Decode<'a> for &'a OpensInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        u16::decode(&mut header)?;
        u16::decode(&mut header)?;
        let count = u16::decode(&mut header)?;
        try_borrow_view(bytes, usize::from(count))
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, TryFromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ProvidesInfo {
    pub service: ConstantPoolIndex,
    pub provides_with_count: U16,
    pub provides_with: [ConstantPoolIndex],
}

impl<'a> Decode<'a> for &'a ProvidesInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        u16::decode(&mut header)?;
        let count = u16::decode(&mut header)?;
        try_borrow_view(bytes, usize::from(count))
    }
}
