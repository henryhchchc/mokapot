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
