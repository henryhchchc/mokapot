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
    pub exports: Box<[&'a ExportsInfo]>,
    pub opens: Box<[&'a OpensInfo]>,
    pub uses: &'a [U16],
    pub provides: Box<[&'a ProvidesInfo]>,
}

impl<'a> Decode<'a> for ModuleInfo<'a> {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let module_name_index = decode_value(bytes)?;
        let module_flags = decode_value(bytes)?;
        let module_version_index = decode_value(bytes)?;
        let requires_count: u16 = decode_value(bytes)?;
        let requires = borrow_slice(bytes, usize::from(requires_count))?;
        let exports_count: u16 = decode_value(bytes)?;
        let exports = parse_repeated(bytes, exports_count, <&ExportsInfo>::decode)?;
        let opens_count: u16 = decode_value(bytes)?;
        let opens = parse_repeated(bytes, opens_count, <&OpensInfo>::decode)?;
        let uses_count: u16 = decode_value(bytes)?;
        let uses = borrow_slice(bytes, usize::from(uses_count))?;
        let provides_count: u16 = decode_value(bytes)?;
        let provides = parse_repeated(bytes, provides_count, <&ProvidesInfo>::decode)?;
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
pub struct RequiresInfo {
    pub requires_index: U16,
    pub requires_flags: U16,
    pub requires_version_index: U16,
}

impl FixedLayout for RequiresInfo {}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ExportsInfo {
    pub exports_index: U16,
    pub exports_flags: U16,
    pub exports_to_count: U16,
    pub exports_to: [U16],
}

impl<'a> Decode<'a> for &'a ExportsInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: u16 = decode_value(&mut header)?;
        let _: u16 = decode_value(&mut header)?;
        let count: u16 = decode_value(&mut header)?;
        let (entry, remaining) = ExportsInfo::ref_from_prefix_with_elems(bytes, usize::from(count))
            .map_err(|_| ParseError::malform("Truncated class-file exports entry"))?;
        *bytes = remaining;
        Ok(entry)
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct OpensInfo {
    pub opens_index: U16,
    pub opens_flags: U16,
    pub opens_to_count: U16,
    pub opens_to: [U16],
}

impl<'a> Decode<'a> for &'a OpensInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: u16 = decode_value(&mut header)?;
        let _: u16 = decode_value(&mut header)?;
        let count: u16 = decode_value(&mut header)?;
        let (entry, remaining) =
            OpensInfo::ref_from_prefix_with_elems(bytes, usize::from(count))
                .map_err(|_| ParseError::malform("Truncated class-file opens entry"))?;
        *bytes = remaining;
        Ok(entry)
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct ProvidesInfo {
    pub provides_index: U16,
    pub provides_with_count: U16,
    pub provides_with: [U16],
}

impl<'a> Decode<'a> for &'a ProvidesInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: u16 = decode_value(&mut header)?;
        let count: u16 = decode_value(&mut header)?;
        let (entry, remaining) =
            ProvidesInfo::ref_from_prefix_with_elems(bytes, usize::from(count))
                .map_err(|_| ParseError::malform("Truncated class-file provides entry"))?;
        *bytes = remaining;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_tables_follow_entry_counts() {
        let words: &[u16] = &[
            1, 0, 0, // module name, flags, version
            0, // requires
            2, 3, 0, 0, 4, 0, 2, 5, 6, // exports: empty, then two targets
            2, 7, 0, 0, 8, 0, 2, 9, 10, // opens: empty, then two targets
            0,  // uses
            2, 11, 0, 12, 2, 13, 14, // provides: empty, then two implementations
        ];
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_be_bytes()).collect();
        let mut remaining = bytes.as_slice();
        let module = ModuleInfo::decode(&mut remaining).unwrap();

        let empty: &[U16] = &[];
        assert_eq!(&module.exports[0].exports_to, empty);
        assert_eq!(module.exports[1].exports_index.get(), 4);
        assert_eq!(&module.exports[1].exports_to, [U16::new(5), U16::new(6)]);
        assert_eq!(&module.opens[0].opens_to, empty);
        assert_eq!(module.opens[1].opens_index.get(), 8);
        assert_eq!(&module.opens[1].opens_to, [U16::new(9), U16::new(10)]);
        assert_eq!(&module.provides[0].provides_with, empty);
        assert_eq!(module.provides[1].provides_index.get(), 12);
        assert_eq!(
            &module.provides[1].provides_with,
            [U16::new(13), U16::new(14)]
        );
        assert_eq!(remaining, b"");
    }
}
