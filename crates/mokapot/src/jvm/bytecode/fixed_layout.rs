//! Stream adapters for fixed JVM wire layouts.

use std::io::{self, Read, Write};

use zerocopy::{
    FromBytes, Immutable, IntoBytes, Unaligned,
    byteorder::big_endian::{F32, F64, I32, I64, U16},
};

use super::{FromBytecode, GenerationError, ToBytecode};

/// Opts a type into bytewise I/O. Fields must use JVM wire order.
pub(super) trait FixedLayout: FromBytes + IntoBytes + Immutable + Unaligned {}

impl FixedLayout for U16 {}
impl FixedLayout for I32 {}
impl FixedLayout for F32 {}
impl FixedLayout for I64 {}
impl FixedLayout for F64 {}

impl<T: FixedLayout> FromBytecode for T {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let mut value = Self::new_zeroed();
        reader.read_exact(value.as_mut_bytes())?;
        Ok(value)
    }
}

impl<T: FixedLayout> ToBytecode for T {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
}
