//! Stream adapters for fixed JVM wire layouts.

use std::io::{self, Read};

use zerocopy::{
    FromBytes, Immutable, Unaligned,
    byteorder::big_endian::{F32, F64, I32, I64, U16},
};

use super::FromBytecode;

/// Opts a type into bytewise parsing. Fields must use JVM wire order.
pub(super) trait FixedLayout: FromBytes + Immutable + Unaligned {}

impl FixedLayout for U16 {}
impl FixedLayout for I32 {}
impl FixedLayout for F32 {}
impl FixedLayout for I64 {}
impl FixedLayout for F64 {}

impl<T: FixedLayout> FromBytecode for T {
    fn from_reader<R: Read>(reader: R) -> io::Result<Self> {
        Self::read_from_io(reader)
    }
}
