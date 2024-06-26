//! Constant pool in a JVM class file.

use std::io::{self, Read};

use crate::macros::see_jvm_spec;

use crate::jvm::JavaString;

use super::ConstantPool;

#[derive(Debug, Clone)]
pub(super) enum Slot {
    Entry(Entry),
    Padding,
}

impl ConstantPool {
    /// Parses a constant pool from the given bytes.
    /// - `constant_pool_count` is the maximum index of entries in the constant pool plus one.
    #[doc = see_jvm_spec!(4, 1)]
    /// # Errors
    /// See [`Error`](super::super::parsing::Error) for more information.
    pub fn from_reader<R>(reader: &mut R, constant_pool_count: u16) -> io::Result<Self>
    where
        R: Read + ?Sized,
    {
        // The `constant_pool` table is indexed from `1` to `constant_pool_count - 1`.
        let count: usize = constant_pool_count.into();
        let mut inner = Vec::with_capacity(count);
        inner.push(Slot::Padding);
        while inner.len() < count {
            let entry = Entry::parse(reader)?;
            if let entry @ (Entry::Long(_) | Entry::Double(_)) = entry {
                inner.push(Slot::Entry(entry));
                inner.push(Slot::Padding);
            } else {
                inner.push(Slot::Entry(entry));
            }
        }
        Ok(Self { inner })
    }

    /// Gets the constant pool entry at the given index.
    /// # Errors
    /// - [`BadConstantPoolIndex`] if `index` does not point to a valid entry.
    pub fn get_entry(&self, index: u16) -> Result<&Entry, BadConstantPoolIndex> {
        match self.inner.get(usize::from(index)) {
            Some(Slot::Entry(entry)) => Ok(entry),
            _ => Err(BadConstantPoolIndex(index)),
        }
    }
}

/// An error when getting an entry from the constant pool with an invalid index.
#[derive(Debug, thiserror::Error)]
#[error("Bad constant pool index: {0}")]
pub struct BadConstantPoolIndex(pub u16);

/// An entry in the [`ConstantPool`].
#[derive(Debug, Clone)]
#[repr(u8)]
#[non_exhaustive]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Entry {
    /// A UTF-8 string.
    #[doc = see_jvm_spec!(4, 4, 7)]
    Utf8(JavaString) = 1,
    /// An integer.
    #[doc = see_jvm_spec!(4, 4, 4)]
    Integer(i32) = 3,
    /// A float.
    #[doc = see_jvm_spec!(4, 4, 4)]
    Float(f32) = 4,
    /// A long.
    #[doc = see_jvm_spec!(4, 4, 5)]
    Long(i64) = 5,
    /// A double.
    #[doc = see_jvm_spec!(4, 4, 5)]
    Double(f64) = 6,
    /// A class.
    #[doc = see_jvm_spec!(4, 4, 1)]
    Class {
        /// The index in the constant pool of its binary name.
        name_index: u16,
    } = 7,
    /// A string.
    #[doc = see_jvm_spec!(4, 4, 3)]
    String {
        /// The index in the constant pool of its UTF-8 value.
        /// The entry at that index must be a [`Entry::Utf8`].
        string_index: u16,
    } = 8,
    /// A field reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    FieldRef {
        /// The index in the constant pool of the class containing the field.
        /// The entry at that index must be a [`Entry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the field.
        /// The entry at that index must be a [`Entry::NameAndType`].
        name_and_type_index: u16,
    } = 9,
    /// A method reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    MethodRef {
        /// The index in the constant pool of the class containing the method.
        /// The entry at that index must be a [`Entry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`Entry::NameAndType`].
        name_and_type_index: u16,
    } = 10,
    /// An interface method reference.
    #[doc = see_jvm_spec!(4, 4, 2)]
    InterfaceMethodRef {
        /// The index in the constant pool of the interface containing the method.
        /// The entry at that index must be a [`Entry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`Entry::NameAndType`].
        name_and_type_index: u16,
    } = 11,
    /// A name and type.
    #[doc = see_jvm_spec!(4, 4, 6)]
    NameAndType {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`Entry::Utf8`].
        name_index: u16,
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`Entry::Utf8`].
        descriptor_index: u16,
    } = 12,
    /// A method handle.
    #[doc = see_jvm_spec!(4, 4, 8)]
    MethodHandle {
        /// The kind of method handle.
        reference_kind: u8,
        /// The index in the constant pool of the method handle.
        /// The entry at that index must be a [`Entry::MethodRef`], [`Entry::InterfaceMethodRef`] or [`Entry::FieldRef`].
        reference_index: u16,
    } = 15,
    /// A method type.
    #[doc = see_jvm_spec!(4, 4, 9)]
    MethodType {
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`Entry::Utf8`].
        descriptor_index: u16,
    } = 16,
    /// A dynamically computed constant.
    #[doc = see_jvm_spec!(4, 4, 10)]
    Dynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`Entry::NameAndType`].
        name_and_type_index: u16,
    } = 17,
    /// An invokedynamic instruction.
    #[doc = see_jvm_spec!(4, 4, 10)]
    InvokeDynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`Entry::NameAndType`].
        name_and_type_index: u16,
    } = 18,
    /// A module.
    #[doc = see_jvm_spec!(4, 4, 11)]
    Module {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`Entry::Utf8`].
        name_index: u16,
    } = 19,
    /// A package.
    #[doc = see_jvm_spec!(4, 4, 12)]
    Package {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`Entry::Utf8`].
        name_index: u16,
    } = 20,
}

impl Entry {
    /// Gets the kind of this constant pool entry.
    #[must_use]
    pub const fn constant_kind<'a>(&self) -> &'a str {
        match self {
            Self::Utf8(_) => "CONSTANT_Utf8",
            Self::Integer(_) => "CONSTANT_Integer",
            Self::Float(_) => "CONSTANT_Float",
            Self::Long(_) => "CONSTANT_Long",
            Self::Double(_) => "CONSTANT_Double",
            Self::Class { .. } => "CONSTANT_Class",
            Self::String { .. } => "CONSTANT_String",
            Self::FieldRef { .. } => "CONSTANT_Fieldref",
            Self::MethodRef { .. } => "CONSTANT_Methodref",
            Self::InterfaceMethodRef { .. } => "CONSTANT_InterfaceMethodref",
            Self::NameAndType { .. } => "CONSTANT_NameAndType",
            Self::MethodHandle { .. } => "CONSTANT_MethodHandle",
            Self::MethodType { .. } => "CONSTANT_MethodType",
            Self::Dynamic { .. } => "CONSTANT_Dynamic",
            Self::InvokeDynamic { .. } => "CONSTANT_InvokeDynamic",
            Self::Module { .. } => "CONSTANT_Module",
            Self::Package { .. } => "CONSTANT_Package",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::parsing::constant_pool::tests::arb_constant_pool_info;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_constant_pool_bytes()(
            entries in prop::collection::vec(arb_constant_pool_info(), 1..=50)
        ) -> (u16, Vec<u8>) {
            let count = {
                let mut len = entries.len();
                len += entries.iter().filter(|&it| {
                    it.first().is_some_and(|&it| it == 5 || it == 6)
                }).count();
                len += 1;
                u16::try_from(len).unwrap()
            };
            let bytes = entries.into_iter().flatten().collect();
            (count, bytes)
        }
    }

    proptest! {

        #[test]
        fn from_reader((count, bytes) in arb_constant_pool_bytes()) {
            let mut reader = bytes.as_slice();
            let constant_pool = ConstantPool::from_reader(&mut reader, count);
            assert!(constant_pool.is_ok());
            assert!(reader.is_empty());
        }

        #[test]
        fn from_reader_err_on_wrong_count((count, bytes) in arb_constant_pool_bytes()) {
            let mut reader = bytes.as_slice();
            let constant_pool = ConstantPool::from_reader(&mut reader, count + 1);
            assert!(constant_pool.is_err());
        }

        #[test]
        fn constant_kind(entry in any::<Entry>()) {
            let kind = entry.constant_kind();
            assert!(kind.starts_with("CONSTANT_"));
        }

    }
}
