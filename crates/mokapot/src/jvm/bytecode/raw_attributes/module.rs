//! Raw attribute types for module-related attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, ParseError,
    decoder::{BytecodeSlice, read_value},
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ModuleInfo {
    pub module_name_index: u16,
    pub module_flags: u16,
    pub module_version_index: u16,
    pub requires: Vec<RequiresInfo>,
    pub exports: Vec<ExportsInfo>,
    pub opens: Vec<OpensInfo>,
    pub uses: Vec<u16>,
    pub provides: Vec<ProvidesInfo>,
}

impl FromBytecode for ModuleInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let module_name_index = read_value::<U16>(bytes)?.get();
        let module_flags = read_value::<U16>(bytes)?.get();
        let module_version_index = read_value::<U16>(bytes)?.get();
        let requires_count: u16 = read_value::<U16>(bytes)?.get();
        let requires = (0..requires_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
        let exports_count: u16 = read_value::<U16>(bytes)?.get();
        let exports = (0..exports_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
        let opens_count: u16 = read_value::<U16>(bytes)?.get();
        let opens = (0..opens_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
        let uses_count: u16 = read_value::<U16>(bytes)?.get();
        let uses = (0..uses_count)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        let provides_count: u16 = read_value::<U16>(bytes)?.get();
        let provides = (0..provides_count)
            .map(|_| bytes.decode_value())
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            module_name_index,
            module_flags,
            module_version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct RequiresInfo {
    pub requires_index: U16,
    pub requires_flags: U16,
    pub requires_version_index: U16,
}

impl FixedLayout for RequiresInfo {}

#[doc = see_jvm_spec!(4, 7, 25)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct ExportsInfo {
    pub exports_index: u16,
    pub exports_flags: u16,
    pub exports_to: Box<[u16]>,
}

impl FromBytecode for ExportsInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let exports_index = read_value::<U16>(bytes)?.get();
        let exports_flags = read_value::<U16>(bytes)?.get();
        let exports_to_count: u16 = read_value::<U16>(bytes)?.get();
        let exports_to = (0..exports_to_count)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            exports_index,
            exports_flags,
            exports_to,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct OpensInfo {
    pub opens_index: u16,
    pub opens_flags: u16,
    pub opens_to: Vec<u16>,
}

impl FromBytecode for OpensInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let opens_index = read_value::<U16>(bytes)?.get();
        let opens_flags = read_value::<U16>(bytes)?.get();
        let opens_to_count: u16 = read_value::<U16>(bytes)?.get();
        let opens_to = (0..opens_to_count)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            opens_index,
            opens_flags,
            opens_to,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ProvidesInfo {
    pub provides_index: u16,
    pub provides_with: Vec<u16>,
}

impl FromBytecode for ProvidesInfo {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        let provides_index = read_value::<U16>(bytes)?.get();
        let provides_with_count: u16 = read_value::<U16>(bytes)?.get();
        let provides_with = (0..provides_with_count)
            .map(|_| read_value::<U16>(bytes).map(u16::from))
            .collect::<Result<_, ParseError>>()?;
        Ok(Self {
            provides_index,
            provides_with,
        })
    }
}
