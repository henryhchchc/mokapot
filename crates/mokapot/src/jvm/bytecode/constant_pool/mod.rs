//! Indexed constant-pool entries and binary parsing.

mod codec;

use crate::{
    intrinsics::{enum_discriminant, see_jvm_spec},
    jvm::{
        bytecode::{FromBytecode, ParseError},
        constant_pool_storage::{PoolStorage, Slot},
    },
};
use zerocopy::byteorder::big_endian::{F32, F64, I32, I64, U16};

/// The indexed, unresolved representation of a JVM constant pool.
/// UTF-8 entries borrow the input buffer and cannot outlive it.
///
/// ```compile_fail
/// use mokapot::jvm::bytecode::constant_pool::RawConstantPool;
///
/// let pool = {
///     let bytes = vec![1, 0, 1, b'A'];
///     RawConstantPool::from_bytes(&bytes, 2).unwrap()
/// };
/// assert!(pool.get_entry(1).is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawConstantPool<'a> {
    pub(crate) inner: PoolStorage<Vec<Slot<RawEntry<'a>>>>,
}

impl<'a> RawConstantPool<'a> {
    /// Creates a new empty constant pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: PoolStorage::with_capacity(1),
        }
    }

    /// Creates a new constant pool with the given capacity.
    /// # Parameters
    /// - `count`: the maximum index of entries in the constant pool plus one.
    #[must_use]
    pub fn with_capacity(count: u16) -> Self {
        Self {
            inner: PoolStorage::with_capacity(count),
        }
    }

    /// Parses the specified number of constant-pool slots from a byte slice.
    /// Bytes following the pool are ignored.
    /// # Errors
    /// Returns a [`ParseError`] for malformed or truncated entries.
    pub fn from_bytes(mut bytes: &'a [u8], constant_pool_count: u16) -> Result<Self, ParseError> {
        Self::parse(&mut bytes, constant_pool_count)
    }

    pub(super) fn parse(
        bytes: &mut &'a [u8],
        constant_pool_count: u16,
    ) -> Result<Self, ParseError> {
        if constant_pool_count == 0 {
            return Err(ParseError::malform("Zero constant pool count"));
        }
        let mut constant_pool = Self::with_capacity(constant_pool_count);
        while constant_pool.count() < constant_pool_count {
            let entry = RawEntry::from_bytes(bytes)?;
            if entry.slot_width() > constant_pool_count - constant_pool.count() {
                return Err(ParseError::malform("Missing constant pool padding slot"));
            }
            constant_pool
                .put_entry(entry)
                .map_err(ParseError::malform)?;
        }
        Ok(constant_pool)
    }

    /// Gets the constant pool entry at the given index.
    ///
    /// Returns [`None`] if the index is out of bounds or the index does not point to a valid slot.
    #[must_use]
    pub fn get_entry(&self, index: u16) -> Option<&RawEntry<'a>> {
        self.inner.get_entry(index)
    }

    /// Put a constant pool entry to the end of the constant pool and return the index of the inserted entry.
    ///
    /// # Errors
    /// Returns back the entry if the constant pool is full (i.e., contains more than 65535 slots).
    ///
    /// `long` and `double` constants occupy two slots:
    ///
    /// ```
    /// use mokapot::jvm::bytecode::constant_pool::{RawConstantPool, RawEntry};
    ///
    /// let mut pool = RawConstantPool::new();
    /// let index = pool.put_entry(RawEntry::Long(42.into())).unwrap();
    /// assert_eq!(index, 1);
    /// assert_eq!(pool.get_entry(index), Some(&RawEntry::Long(42.into())));
    /// assert_eq!(pool.get_entry(index + 1), None);
    /// assert_eq!(pool.put_entry(RawEntry::Integer(7.into())).unwrap(), index + 2);
    /// ```
    pub fn put_entry(&mut self, entry: RawEntry<'a>) -> Result<u16, Overflow<'a>> {
        let padding = entry.slot_width() == 2;
        self.inner.push(entry, padding).map_err(Overflow)
    }

    /// Pushes a constant pool entry to the end of the constant pool if it does not already exist.
    ///
    /// # Return Values
    /// [`Ok`] with a tuple indicating the index of the entry within the constant pool, and whether the entry is freshly
    /// inserted into the pool.
    ///
    /// # Errors
    /// Returns back the entry if the constant pool is full (i.e., contains more than 65535 slots).
    ///
    /// ```
    /// use mokapot::jvm::bytecode::constant_pool::{RawConstantPool, RawEntry};
    ///
    /// let mut pool = RawConstantPool::new();
    /// let (index, inserted) = pool.put_entry_deduplicated(RawEntry::Integer(42.into())).unwrap();
    /// assert!(inserted);
    /// assert_eq!(pool.put_entry_deduplicated(RawEntry::Integer(42.into())).unwrap(), (index, false));
    /// assert_eq!(pool.get_entry(index), Some(&RawEntry::Integer(42.into())));
    /// ```
    pub fn put_entry_deduplicated(
        &mut self,
        entry: RawEntry<'a>,
    ) -> Result<(u16, bool), Overflow<'a>> {
        if let Some((index, _)) = self.find(|it| it == &entry) {
            return Ok((index, false));
        }
        self.put_entry(entry).map(|index| (index, true))
    }

    /// Finds the first constant pool entry that satisfies the given predicate.
    pub fn find<P>(&self, predicate: P) -> Option<(u16, &RawEntry<'a>)>
    where
        P: Fn(&RawEntry<'a>) -> bool,
    {
        self.inner.find(predicate)
    }

    /// Returns the indexed slot count, including slot zero and reserved slots.
    #[must_use]
    pub const fn count(&self) -> u16 {
        self.inner.count()
    }
}

impl Default for RawConstantPool<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// An error when an insertion to the constant pool causes an overflow.
#[derive(Debug, thiserror::Error)]
#[error("The constant pool is full.")]
pub struct Overflow<'a>(pub RawEntry<'a>);

/// An entry in the [`RawConstantPool`].
///
/// Equality compares wire bytes, including floating-point sign and NaN payload bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
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
        /// The index in the constant pool of its binary name.
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
    /// An invokedynamic instruction.
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

    /// Returns the tag of this constant pool entry.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        // Safety: Self is marked as repr(u8)
        unsafe { enum_discriminant(self) }
    }

    /// Gets the kind of this constant pool entry.
    #[doc = see_jvm_spec!(4, 4)]
    #[must_use]
    pub const fn constant_kind(&self) -> &'static str {
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
    use crate::jvm::bytecode::ParseErrorKind;
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn wire_scalars() {
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
            (RawEntry::Utf8(&[0xc0, 0x80]), &[1, 0, 2, 0xc0, 0x80]),
        ];
        for (entry, bytes) in cases {
            let mut reader = *bytes;
            assert_eq!(&RawEntry::from_bytes(&mut reader).unwrap(), entry);
            for end in 0..bytes.len() {
                assert_eq!(
                    RawEntry::from_bytes(&mut &bytes[..end]).unwrap_err().kind(),
                    ParseErrorKind::Malformed
                );
            }
        }
    }

    #[test]
    fn float_dedup_uses_wire_bits() {
        for entries in [
            [0, 0x8000_0000, 0x7fc0_0001, 0x7fc0_0002]
                .map(|bits| RawEntry::Float(f32::from_bits(bits).into())),
            [
                0,
                0x8000_0000_0000_0000,
                0x7ff8_0000_0000_0001,
                0x7ff8_0000_0000_0002,
            ]
            .map(|bits| RawEntry::Double(f64::from_bits(bits).into())),
        ] {
            let mut pool = RawConstantPool::new();
            for entry in entries {
                let (index, inserted) = pool.put_entry_deduplicated(entry).unwrap();
                assert!(inserted);
                assert_eq!(pool.put_entry_deduplicated(entry).unwrap(), (index, false));
            }
        }
    }

    const MAX_BYTES: usize = 255;

    #[test]
    fn miri_entry_tags_match_encoding() {
        assert_eq!(RawEntry::Integer(42.into()).tag(), 3);
        assert_eq!(
            RawEntry::Class {
                name_index: 1.into()
            }
            .tag(),
            7
        );
        let method_handle = RawEntry::MethodHandle {
            reference_kind: 9,
            reference_index: 1.into(),
        };
        assert_eq!(method_handle.tag(), 15);
    }

    #[test]
    fn deduplicated_entries_keep_the_original_index() {
        let mut pool = RawConstantPool::new();
        let entry = RawEntry::Integer(42.into());

        let first = pool.put_entry_deduplicated(entry).unwrap();
        assert_eq!(first, (1, true));
        assert_eq!(pool.put_entry_deduplicated(entry).unwrap(), (1, false));
        assert_eq!(pool.count(), 2);
    }

    #[test]
    fn long_and_double_entries_reserve_the_following_index() {
        let mut pool = RawConstantPool::new();

        let long_index = pool.put_entry(RawEntry::Long(42.into())).unwrap();
        let integer_index = pool.put_entry(RawEntry::Integer(7.into())).unwrap();
        let double_index = pool.put_entry(RawEntry::Double(3.5.into())).unwrap();

        assert_eq!((long_index, integer_index, double_index), (1, 3, 4));
        assert_eq!(pool.get_entry(2), None);
        assert_eq!(pool.get_entry(3), Some(&RawEntry::Integer(7.into())));
        assert_eq!(pool.get_entry(5), None);
        assert_eq!(pool.count(), 6);
    }

    #[test]
    fn constant_pool_overflow_respects_reserved_slots() {
        let mut pool = RawConstantPool::new();
        for _ in 0..(u16::MAX as usize - 2) {
            pool.put_entry(RawEntry::Integer(0.into())).unwrap();
        }

        assert!(pool.put_entry(RawEntry::Long(1.into())).is_err());
        assert_eq!(pool.count(), u16::MAX - 1);
        assert_eq!(
            pool.put_entry(RawEntry::Integer(1.into())).unwrap(),
            u16::MAX - 1
        );
        assert_eq!(pool.count(), u16::MAX);
        assert!(pool.put_entry(RawEntry::Integer(1.into())).is_err());
        assert!(pool.put_entry(RawEntry::Long(1.into())).is_err());
        assert_eq!(pool.count(), u16::MAX);
    }

    #[test]
    fn invalid_slot_counts() {
        for (count, bytes) in [
            (0, vec![]),
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

    proptest! {

        #[test]
        fn from_bytes((count, bytes) in arb_constant_pool_bytes()) {
            let mut remaining = bytes.as_slice();
            let constant_pool = RawConstantPool::parse(&mut remaining, count).unwrap();
            prop_assert!(remaining.is_empty());
            let public_pool = RawConstantPool::from_bytes(&bytes, count).unwrap();
            prop_assert_eq!(constant_pool, public_pool);
        }

        #[test]
        fn from_bytes_err_on_wrong_count((count, bytes) in arb_constant_pool_bytes()) {
            let reader = bytes.as_slice();
            let constant_pool = RawConstantPool::from_bytes(reader, count + 1);
            assert!(constant_pool.is_err());
        }

        #[test]
        fn constant_kind(bytes in arb_constant_pool_info()) {
            let entry = RawEntry::from_bytes(&mut bytes.as_slice()).unwrap();
            let kind = entry.constant_kind();
            assert!(kind.starts_with("CONSTANT_"));
        }

        #[test]
        fn parse_entry(entry in arb_constant_pool_info()) {
            let mut reader = entry.as_slice();
            let parsed = RawEntry::from_bytes(&mut reader);
            let tag = entry.first().unwrap();
            match tag {
                1 => assert!(matches!(parsed, Ok(RawEntry::Utf8(_)))),
                3 => assert!(matches!(parsed, Ok(RawEntry::Integer(_)))),
                4 => assert!(matches!(parsed, Ok(RawEntry::Float(_)))),
                5 => assert!(matches!(parsed, Ok(RawEntry::Long(_)))),
                6 => assert!(matches!(parsed, Ok(RawEntry::Double(_)))),
                7 => assert!(matches!(parsed, Ok(RawEntry::Class { .. }))),
                8 => assert!(matches!(parsed, Ok(RawEntry::String { .. }))),
                9 => assert!(matches!(parsed, Ok(RawEntry::FieldRef { .. }))),
                10 => assert!(matches!(parsed, Ok(RawEntry::MethodRef { .. }))),
                11 => assert!(matches!(parsed, Ok(RawEntry::InterfaceMethodRef { .. }))),
                12 => assert!(matches!(parsed, Ok(RawEntry::NameAndType { .. }))),
                15 => assert!(matches!(parsed, Ok(RawEntry::MethodHandle { .. }))),
                16 => assert!(matches!(parsed, Ok(RawEntry::MethodType { .. }))),
                17 => assert!(matches!(parsed, Ok(RawEntry::Dynamic { .. }))),
                18 => assert!(matches!(parsed, Ok(RawEntry::InvokeDynamic { .. }))),
                19 => assert!(matches!(parsed, Ok(RawEntry::Module { .. }))),
                20 => assert!(matches!(parsed, Ok(RawEntry::Package { .. }))),
                _ => unreachable!("`arb_constant_pool_info` produces only defined tags")
            }
        }

    }

    prop_compose! {
        pub fn arb_constant_pool_bytes()(
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

    fn tagged(tag: u8, payload: impl IntoIterator<Item = u8>) -> Vec<u8> {
        std::iter::once(tag).chain(payload).collect()
    }

    fn arb_constant_pool_info() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            prop::collection::vec(any::<u8>(), 0..=MAX_BYTES).prop_map(|bytes| {
                let length = u16::try_from(bytes.len()).unwrap();
                tagged(1, length.to_be_bytes().into_iter().chain(bytes))
            }),
            (prop::sample::select(vec![3, 4]), any::<[u8; 4]>())
                .prop_map(|(tag, bytes)| tagged(tag, bytes)),
            (prop::sample::select(vec![5, 6]), any::<[u8; 8]>())
                .prop_map(|(tag, bytes)| tagged(tag, bytes)),
            (prop::sample::select(vec![7, 8, 16, 19, 20]), 1..=u16::MAX)
                .prop_map(|(tag, index)| tagged(tag, index.to_be_bytes())),
            (
                prop::sample::select(vec![9, 10, 11, 12, 17, 18]),
                1..=u16::MAX,
                1..=u16::MAX
            )
                .prop_map(|(tag, first, second)| tagged(
                    tag,
                    first.to_be_bytes().into_iter().chain(second.to_be_bytes())
                )),
            (1..=u8::MAX, 1..=u16::MAX).prop_map(|(kind, index)| tagged(
                15,
                std::iter::once(kind).chain(index.to_be_bytes())
            )),
        ]
    }
}
