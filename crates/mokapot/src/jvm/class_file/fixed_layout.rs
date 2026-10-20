//! Slice decoding for fixed JVM wire layouts.

use zerocopy::{
    FromBytes, Immutable, Unaligned,
    byteorder::big_endian::{F32, F64, I16, I32, I64, U16, U32},
};

/// Opts a type into parsing its representation as JVM wire bytes.
///
/// Zerocopy compatibility alone does not establish JVM field order or endianness.
/// Multi-byte fields must use big-endian wire types.
pub(super) trait FixedLayout: FromBytes + Immutable + Unaligned {}

impl FixedLayout for u8 {}
impl FixedLayout for i8 {}
impl<T: FixedLayout, const N: usize> FixedLayout for [T; N] {}
impl FixedLayout for U16 {}
impl FixedLayout for U32 {}
impl FixedLayout for I16 {}
impl FixedLayout for I32 {}
impl FixedLayout for F32 {}
impl FixedLayout for I64 {}
impl FixedLayout for F64 {}
