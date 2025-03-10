//! Constant pool in a JVM class file.

use std::io::{self, Read};

use crate::{
    jvm::{
        JavaString,
        class::ConstantPool,
        parsing::{ToWriter, ToWriterError},
    },
    macros::see_jvm_spec,
    utils::enum_discriminant,
};

#[derive(Debug, Clone)]
pub(super) enum Slot {
    Entry(Entry),
    Padding,
}

impl ConstantPool {
    /// Creates a new empty constant pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: vec![Slot::Padding],
        }
    }

    /// Creates a new constant pool with the given capacity.
    /// # Parameters
    /// - `count`: the maximum index of entries in the constant pool plus one.
    #[must_use]
    pub fn with_capacity(count: u16) -> Self {
        // The `constant_pool` table is indexed from `1` to `constant_pool_count - 1`.
        let mut inner = Vec::with_capacity((count + 1) as usize);
        inner.push(Slot::Padding);
        Self { inner }
    }

    /// Parses a constant pool from the given bytes.
    /// - `constant_pool_count` is the maximum index of entries in the constant pool plus one.
    #[doc = see_jvm_spec!(4, 1)]
    /// # Errors
    /// See [`Error`](super::super::parsing::Error) for more information.
    pub fn from_reader<R>(reader: &mut R, constant_pool_count: u16) -> io::Result<Self>
    where
        R: Read + ?Sized,
    {
        let mut constant_pool = Self::with_capacity(constant_pool_count);
        while constant_pool.count() < constant_pool_count {
            // NOTE: Do not use `put_entry` here since it will do deduplication.
            let entry = Entry::parse(reader)?;
            if let entry @ (Entry::Long(_) | Entry::Double(_)) = entry {
                constant_pool.inner.push(Slot::Entry(entry));
                constant_pool.inner.push(Slot::Padding);
            } else {
                constant_pool.inner.push(Slot::Entry(entry));
            }
        }
        Ok(constant_pool)
    }

    /// Gets the constant pool entry at the given index.
    /// # Errors
    /// - [`Error::BadIndex`] if `index` does not point to a valid entry.
    pub fn get_entry(&self, index: u16) -> Result<&Entry, Error> {
        match self.inner.get(usize::from(index)) {
            Some(Slot::Entry(entry)) => Ok(entry),
            _ => Err(Error::BadIndex(index)),
        }
    }

    /// Pushes a constant pool entry to the end of the constant pool.
    /// # Errors
    /// - [`Error::Overflow`] if the constant pool is full.
    pub fn put_entry(&mut self, entry: Entry) -> Result<u16, Error> {
        if let Some(index) = self.find_index(|it| it == &entry) {
            return Ok(index);
        }
        let new_index = self.count();
        if matches!(entry, Entry::Long(_) | Entry::Double(_)) {
            if self.inner.len() + 2 > u16::MAX as usize {
                return Err(Error::Overflow);
            }
            self.inner.push(Slot::Entry(entry));
            self.inner.push(Slot::Padding);
        } else {
            if self.inner.len() > u16::MAX as usize {
                return Err(Error::Overflow);
            }
            self.inner.push(Slot::Entry(entry));
        }
        Ok(new_index)
    }

    /// Finds the first constant pool entry that satisfies the given predicate.
    pub fn find<P>(&self, predicate: P) -> Option<(u16, &Entry)>
    where
        P: Fn(&Entry) -> bool,
    {
        self.inner
            .iter()
            .enumerate()
            .find_map(|(idx, slot)| match slot {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "When constructing the constant pool, \
                              we ensured that the index is within the bounds of u16. \
                              Therefore, it is safe to cast the length to u16."
                )]
                Slot::Entry(entry) if predicate(entry) => Some((idx as u16, entry)),
                _ => None,
            })
    }

    pub(crate) fn find_index<P>(&self, predicate: P) -> Option<u16>
    where
        P: Fn(&Entry) -> bool,
    {
        self.find(predicate).map(|(idx, _)| idx)
    }

    /// Gets the count of the constant pool. Note that this is NOT the number of entries.
    #[doc = see_jvm_spec!(4, 1)]
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "When constructing the constant pool, \
                  we ensured that the index is within the bounds of u16. \
                  Therefore, it is safe to cast the length to u16."
    )]
    pub fn count(&self) -> u16 {
        self.inner.len() as u16
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ToWriter for ConstantPool {
    fn write_to<W: io::Write>(&self, writer: &mut W) -> Result<(), ToWriterError> {
        writer.write_all(&self.count().to_be_bytes())?;
        for entry in &self.inner {
            entry.write_to(writer)?;
        }
        Ok(())
    }
}

impl ToWriter for Slot {
    fn write_to<W: io::Write>(&self, writer: &mut W) -> Result<(), ToWriterError> {
        if let Self::Entry(entry) = self {
            entry.write_to(writer)?;
        }
        Ok(())
    }
}

/// An error when getting an entry from the constant pool with an invalid index.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The index used to access the constant pool is invalid.
    #[error("Bad constant pool index: {0}")]
    BadIndex(u16),
    /// The constant pool is full.
    #[error("The constant pool is full")]
    Overflow,
}

/// An entry in the [`ConstantPool`].
#[derive(Debug, Clone, PartialEq)]
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

impl Eq for Entry {}

impl Entry {
    /// Returns the tag of this constant pool entry.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        // Safety: Self is marked as repr(u8)
        unsafe { enum_discriminant(self) }
    }

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
    use crate::jvm::parsing::constant_pool::tests::arb_constant_pool_bytes;
    use proptest::prelude::*;

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
