//! Raw attribute types for module-related attributes.

use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::U16};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    Decode, ParseError,
    decoder::{borrow_slice, decode_value, parse_repeated},
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ModuleInfo<'a> {
    pub module_name_index: u16,
    pub module_flags: u16,
    pub module_version_index: u16,
    pub requires: &'a [RequiresInfo],
    pub exports: Box<[ExportsInfo<'a>]>,
    pub opens: Box<[OpensInfo<'a>]>,
    pub uses: &'a [U16],
    pub provides: Box<[ProvidesInfo<'a>]>,
}

impl<'a> Decode<'a> for ModuleInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let module_name_index = decode_value(bytes)?;
        let module_flags = decode_value(bytes)?;
        let module_version_index = decode_value(bytes)?;
        let requires_count: u16 = decode_value(bytes)?;
        let requires = borrow_slice(bytes, usize::from(requires_count))?;
        let exports_count: u16 = decode_value(bytes)?;
        let exports = parse_repeated(bytes, usize::from(exports_count), ExportsInfo::decode)?;
        let opens_count: u16 = decode_value(bytes)?;
        let opens = parse_repeated(bytes, usize::from(opens_count), OpensInfo::decode)?;
        let uses_count: u16 = decode_value(bytes)?;
        let uses = borrow_slice(bytes, usize::from(uses_count))?;
        let provides_count: u16 = decode_value(bytes)?;
        let provides = parse_repeated(bytes, usize::from(provides_count), ProvidesInfo::decode)?;
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
#[derive(Clone, Copy, FromBytes, KnownLayout, Immutable, Unaligned)]
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
pub struct ExportsInfo<'a> {
    pub exports_index: u16,
    pub exports_flags: u16,
    pub exports_to: &'a [U16],
}

impl<'a> Decode<'a> for ExportsInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let exports_index = decode_value(bytes)?;
        let exports_flags = decode_value(bytes)?;
        let exports_to_count: u16 = decode_value(bytes)?;
        let exports_to = borrow_slice(bytes, usize::from(exports_to_count))?;
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
pub struct OpensInfo<'a> {
    pub opens_index: u16,
    pub opens_flags: u16,
    pub opens_to: &'a [U16],
}

impl<'a> Decode<'a> for OpensInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let opens_index = decode_value(bytes)?;
        let opens_flags = decode_value(bytes)?;
        let opens_to_count: u16 = decode_value(bytes)?;
        let opens_to = borrow_slice(bytes, usize::from(opens_to_count))?;
        Ok(Self {
            opens_index,
            opens_flags,
            opens_to,
        })
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ProvidesInfo<'a> {
    pub provides_index: u16,
    pub provides_with: &'a [U16],
}

impl<'a> Decode<'a> for ProvidesInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let provides_index = decode_value(bytes)?;
        let provides_with_count: u16 = decode_value(bytes)?;
        let provides_with = borrow_slice(bytes, usize::from(provides_with_count))?;
        Ok(Self {
            provides_index,
            provides_with,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_tables_borrow_their_input() {
        let words: [u16; 23] = [
            1, 0, 0, // module name, flags, version
            1, 2, 0, 0, // requires
            1, 3, 0, 1, 2, // exports
            1, 4, 0, 1, 2, // opens
            1, 5, // uses
            1, 5, 1, 6, // provides
        ];
        let mut bytes = vec![0xff];
        bytes.extend(words.into_iter().flat_map(u16::to_be_bytes));
        let input = &bytes[1..];
        let mut remaining = input;
        let module = ModuleInfo::decode(&mut remaining).unwrap();
        assert_eq!(remaining, b"");
        for (pointer, word_offset) in [
            (module.requires.as_ptr().cast::<u8>(), 4),
            (module.exports[0].exports_to.as_ptr().cast::<u8>(), 11),
            (module.opens[0].opens_to.as_ptr().cast::<u8>(), 16),
            (module.uses.as_ptr().cast::<u8>(), 18),
            (module.provides[0].provides_with.as_ptr().cast::<u8>(), 22),
        ] {
            assert!(std::ptr::eq(pointer, input[word_offset * 2..].as_ptr()));
        }
        assert_eq!(module.requires[0].requires_index.get(), 2);
        assert_eq!(module.provides[0].provides_with[0].get(), 6);
    }
}
