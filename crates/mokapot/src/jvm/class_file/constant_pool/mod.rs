//! Indexed constant-pool entries and binary parsing.

mod decode;
mod resolve;
pub(super) use resolve::{ConstantPoolBuilder, resolve};

use crate::jvm::constant_pool::storage::{PoolStorage, Slot};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::class_file::{Decode, ParseError},
};
use zerocopy::byteorder::big_endian::{F32, F64, I32, I64, U16};

/// The indexed, unresolved representation of a JVM constant pool.
/// UTF-8 entries borrow the input buffer and cannot outlive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawConstantPool<'a> {
    inner: PoolStorage<RawEntry<'a>>,
}

impl<'a> RawConstantPool<'a> {
    /// Parses the specified number of constant-pool slots from a byte slice.
    /// Bytes following the pool are ignored.
    /// # Errors
    /// Returns a [`ParseError`] for malformed or truncated entries.
    #[cfg(test)]
    pub fn from_bytes(mut bytes: &'a [u8], constant_pool_count: u16) -> Result<Self, ParseError> {
        Self::parse_prefix(&mut bytes, constant_pool_count)
    }

    pub(super) fn parse_prefix(
        bytes: &mut &'a [u8],
        constant_pool_count: u16,
    ) -> Result<Self, ParseError> {
        if constant_pool_count == 0 {
            return Err(ParseError::malform("Zero constant pool count"));
        }
        let count = usize::from(constant_pool_count);
        let mut slots = Vec::with_capacity(count);
        slots.push(Slot::Padding);
        while slots.len() < count {
            let entry = RawEntry::decode(bytes)?;
            let padding = entry.slot_width() == 2;
            slots.push(Slot::Entry(entry));
            if padding {
                if slots.len() == count {
                    return Err(ParseError::malform("Missing constant pool padding slot"));
                }
                slots.push(Slot::Padding);
            }
        }
        Ok(Self {
            inner: PoolStorage::from_slots(slots.into_boxed_slice()),
        })
    }

    /// Gets the constant pool entry at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds or the index does not point to a valid slot.
    #[must_use]
    #[cfg(test)]
    pub fn get_entry(&self, index: u16) -> Option<&RawEntry<'a>> {
        self.inner.get_entry(index)
    }

    /// Returns the indexed slot count, including slot zero and reserved slots.
    #[must_use]
    pub const fn count(&self) -> u16 {
        self.inner.count()
    }

    pub(super) fn into_slots(self) -> Box<[Slot<RawEntry<'a>>]> {
        self.inner.into_slots()
    }
}

#[cfg(test)]
impl<'a> RawConstantPool<'a> {
    pub fn from_entries(entries: impl IntoIterator<Item = RawEntry<'a>>) -> Self {
        let slots: Box<[_]> = std::iter::once(Slot::Padding)
            .chain(entries.into_iter().flat_map(|entry| {
                std::iter::once(Slot::Entry(entry))
                    .chain((entry.slot_width() == 2).then_some(Slot::Padding))
            }))
            .collect();
        assert!(u16::try_from(slots.len()).is_ok());
        Self {
            inner: PoolStorage::from_slots(slots),
        }
    }
}

/// An entry in the [`RawConstantPool`].
///
/// Equality compares wire bytes, including floating-point sign and NaN payload bits.
/// Discriminants match the JVM constant-pool tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RawEntry<'a> {
    /// Modified UTF-8 bytes borrowed from the input.
    #[doc = see_jvm_spec!(4, 4, 7)]
    Utf8(&'a [u8]) = 1,
    /// An integer.
    #[doc = see_jvm_spec!(4, 4, 4)]
    Integer(I32) = 3,
    /// A float.
    #[doc = see_jvm_spec!(4, 4, 4)]
    Float(F32) = 4,
    /// A long.
    #[doc = see_jvm_spec!(4, 4, 5)]
    Long(I64) = 5,
    /// A double.
    #[doc = see_jvm_spec!(4, 4, 5)]
    Double(F64) = 6,
    /// A class.
    #[doc = see_jvm_spec!(4, 4, 1)]
    Class {
        /// The index of its internal name or array descriptor.
        name_index: U16,
    } = 7,
    /// A string.
    #[doc = see_jvm_spec!(4, 4, 3)]
    String {
        /// The index in the constant pool of its UTF-8 value.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        string_index: U16,
    } = 8,
    /// A field reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    FieldRef {
        /// The index in the constant pool of the class containing the field.
        /// The entry at that index must be a [`RawEntry::Class`].
        class_index: U16,
        /// The index in the constant pool of the name and type of the field.
        /// The entry at that index must be a [`RawEntry::NameAndType`].
        name_and_type_index: U16,
    } = 9,
    /// A method reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    MethodRef {
        /// The index in the constant pool of the class containing the method.
        /// The entry at that index must be a [`RawEntry::Class`].
        class_index: U16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`RawEntry::NameAndType`].
        name_and_type_index: U16,
    } = 10,
    /// An interface method reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    InterfaceMethodRef {
        /// The index in the constant pool of the interface containing the method.
        /// The entry at that index must be a [`RawEntry::Class`].
        class_index: U16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`RawEntry::NameAndType`].
        name_and_type_index: U16,
    } = 11,
    /// A name and type.
    #[doc = see_jvm_spec!(4, 4, 6)]
    NameAndType {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        name_index: U16,
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        descriptor_index: U16,
    } = 12,
    /// A method handle.
    #[doc = see_jvm_spec!(4, 4, 8)]
    MethodHandle {
        /// The kind of method handle.
        reference_kind: u8,
        /// The index in the constant pool of the method handle.
        /// The entry at that index must be a [`RawEntry::MethodRef`], [`RawEntry::InterfaceMethodRef`] or [`RawEntry::FieldRef`].
        reference_index: U16,
    } = 15,
    /// A method type.
    #[doc = see_jvm_spec!(4, 4, 9)]
    MethodType {
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        descriptor_index: U16,
    } = 16,
    /// A dynamically computed constant.
    #[doc = see_jvm_spec!(4, 4, 10)]
    Dynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: U16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`RawEntry::NameAndType`].
        name_and_type_index: U16,
    } = 17,
    /// A dynamic call site linked through the bootstrap methods table.
    #[doc = see_jvm_spec!(4, 4, 10)]
    InvokeDynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: U16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`RawEntry::NameAndType`].
        name_and_type_index: U16,
    } = 18,
    /// A module.
    #[doc = see_jvm_spec!(4, 4, 11)]
    Module {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        name_index: U16,
    } = 19,
    /// A package.
    #[doc = see_jvm_spec!(4, 4, 12)]
    Package {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`RawEntry::Utf8`].
        name_index: U16,
    } = 20,
}

impl RawEntry<'_> {
    const fn slot_width(&self) -> u16 {
        if matches!(self, Self::Long(_) | Self::Double(_)) {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::jvm::errors::ParseErrorKind;

    use super::*;

    fn assert_entry_decoding(entry: &RawEntry<'_>, bytes: &[u8]) {
        let mut reader = bytes;
        assert_eq!(&RawEntry::decode(&mut reader).unwrap(), entry);
        assert_eq!(reader, b"");
        for end in 0..bytes.len() {
            assert_eq!(
                RawEntry::decode(&mut &bytes[..end]).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
    }

    #[test]
    fn entry_tags_decode_payloads_and_reject_truncation() {
        let cases: &[(RawEntry, &[u8])] = &[
            (
                RawEntry::Integer((-0x123_4567).into()),
                &[3, 0xfe, 0xdc, 0xba, 0x99],
            ),
            (RawEntry::Float(1.0.into()), &[4, 0x3f, 0x80, 0, 0]),
            (
                RawEntry::Long(0x0102_0304_0506_0708.into()),
                &[5, 1, 2, 3, 4, 5, 6, 7, 8],
            ),
            (
                RawEntry::Double((-2.0).into()),
                &[6, 0xc0, 0, 0, 0, 0, 0, 0, 0],
            ),
            (RawEntry::Utf8(&[0xc0, 0x80]), &[1, 0, 2, 0xc0, 0x80]),
            (
                RawEntry::Class {
                    name_index: 0x1234.into(),
                },
                &[7, 0x12, 0x34],
            ),
            (
                RawEntry::FieldRef {
                    class_index: 0x1234.into(),
                    name_and_type_index: 0x5678.into(),
                },
                &[9, 0x12, 0x34, 0x56, 0x78],
            ),
            (
                RawEntry::MethodHandle {
                    reference_kind: 6,
                    reference_index: 0x1234.into(),
                },
                &[15, 6, 0x12, 0x34],
            ),
        ];
        for (entry, bytes) in cases {
            assert_entry_decoding(entry, bytes);
        }
    }

    #[test]
    fn parsing_preserves_reserved_slots_and_suffix() {
        let bytes = [
            5, 0, 0, 0, 0, 0, 0, 0, 42, // Long #1, reserved #2
            3, 0, 0, 0, 7, // Integer #3
            6, 0x40, 0x0c, 0, 0, 0, 0, 0, 0,    // Double #4, reserved #5
            0xff, // suffix
        ];
        let mut remaining = bytes.as_slice();
        let pool = RawConstantPool::parse_prefix(&mut remaining, 6).unwrap();
        assert_eq!(pool.get_entry(0), None);
        assert_eq!(pool.get_entry(1), Some(&RawEntry::Long(42.into())));
        assert_eq!(pool.get_entry(2), None);
        assert_eq!(pool.get_entry(3), Some(&RawEntry::Integer(7.into())));
        assert_eq!(pool.get_entry(4), Some(&RawEntry::Double(3.5.into())));
        assert_eq!(pool.get_entry(5), None);
        assert_eq!(pool.get_entry(6), None);
        assert_eq!(pool.count(), 6);
        assert_eq!(remaining, [0xff]);
    }

    #[test]
    fn invalid_slot_counts() {
        for (count, bytes) in [
            (0, vec![]),
            (2, vec![]),
            (3, vec![3, 0, 0, 0, 7]),
            (2, vec![5, 0, 0, 0, 0, 0, 0, 0, 0]),
            (2, vec![6, 0, 0, 0, 0, 0, 0, 0, 0]),
        ] {
            let error = RawConstantPool::from_bytes(&bytes, count).unwrap_err();
            assert_eq!(error.kind(), ParseErrorKind::Malformed);
        }
    }

    #[test]
    fn utf8_entries_borrow_input() {
        let bytes = vec![1, 0, 3, b'a', b'b', b'c'];
        let pool = RawConstantPool::from_bytes(&bytes, 2).unwrap();
        let Some(RawEntry::Utf8(value)) = pool.get_entry(1) else {
            panic!("expected a UTF-8 entry");
        };
        assert_eq!(*value, b"abc");
        assert!(std::ptr::eq(value.as_ptr(), bytes[3..].as_ptr()));
    }
}
