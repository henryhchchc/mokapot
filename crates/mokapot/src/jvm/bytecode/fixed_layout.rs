//! Slice decoding for fixed JVM wire layouts.

use zerocopy::{
    FromBytes, Immutable, Unaligned,
    byteorder::big_endian::{F32, F64, I32, I64, U16},
};

use super::{FromBytecode, ParseError, decoder::read_value};

/// Opts a type into bytewise parsing. Fields must use JVM wire order.
pub(super) trait FixedLayout: FromBytes + Immutable + Unaligned {}

impl FixedLayout for U16 {}
impl FixedLayout for I32 {}
impl FixedLayout for F32 {}
impl FixedLayout for I64 {}
impl FixedLayout for F64 {}

impl<T: FixedLayout> FromBytecode for T {
    fn from_bytes(bytes: &mut &[u8]) -> Result<Self, ParseError> {
        read_value::<Self>(bytes)
    }
}
