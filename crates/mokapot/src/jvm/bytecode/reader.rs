use std::{
    io::{Read, Result, Write},
    ops::AddAssign,
};

use trait_gen::trait_gen;

use super::FromBytecode;
use crate::jvm::code::ProgramCounter;

pub(crate) trait BytecodeReader: Read {
    fn decode_value<T: FromBytecode>(&mut self) -> Result<T>;
}

impl<R: Read + ?Sized> BytecodeReader for R {
    fn decode_value<T: FromBytecode>(&mut self) -> Result<T> {
        T::from_reader(self)
    }
}

impl<const N: usize> FromBytecode for [u8; N] {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; N];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl FromBytecode for ProgramCounter {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let inner = u16::from_reader(reader)?;
        Ok(inner.into())
    }
}

#[trait_gen(T -> u8, u16, u32, i8, i16, i32, i64, f32, f64)]
impl FromBytecode for T {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let buf = reader.decode_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

/// Reads [len] bytes and advances the reader by [`len`] bytes.
pub(crate) fn read_vec<R>(reader: &mut R, len: usize) -> Result<Vec<u8>>
where
    R: Read + ?Sized,
{
    let mut buf = Vec::with_capacity(len);
    unsafe {
        // SAFETY: We are going to read exactly `len` bytes into the buffer.
        //         Otherwise, `read_exact` will return an error.
        //         Therefore, we will never return a `Vec` with uninitialized memory.
        buf.set_len(len);
        reader.read_exact(buf.as_mut_slice())
    }?;
    Ok(buf)
}

#[derive(Debug)]
pub(super) struct PositionTracker<Inner> {
    inner: Inner,
    position: usize,
}

impl<Inner> PositionTracker<Inner> {
    pub const fn new(reader: Inner) -> Self {
        Self {
            inner: reader,
            position: 0,
        }
    }

    pub const fn position(&self) -> usize {
        self.position
    }
}

impl<R> Read for PositionTracker<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let Self { inner, position } = self;
        inner.read(buf).inspect(|n| position.add_assign(n))
    }
}

impl<W> Write for PositionTracker<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let Self { inner, position } = self;
        inner.write(buf).inspect(|n| position.add_assign(n))
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod test {
    use std::io::ErrorKind::UnexpectedEof;

    use super::BytecodeReader;
    use proptest::prelude::*;

    #[test]
    fn read_bytes_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = reader.decode_value().unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04]);
    }

    #[test]
    fn read_bytes_failed() {
        let mut reader = [0x01, 0x02].as_slice();
        let err = reader.decode_value::<[u8; 3]>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    macro_rules! integer_decode_properties {
        ($success:ident, $truncated:ident, $type:ty) => {
            proptest! {
                #[test]
                fn $success(value in any::<$type>(), suffix in prop::collection::vec(any::<u8>(), 0..8)) {
                    let mut bytes = value.to_be_bytes().to_vec();
                    bytes.extend(&suffix);
                    let mut reader = bytes.as_slice();

                    let decoded: $type = reader.decode_value().unwrap();

                    prop_assert_eq!(decoded, value);
                    prop_assert_eq!(reader, suffix.as_slice());
                }

                #[test]
                fn $truncated(
                    value in any::<$type>(),
                    available in 0usize..size_of::<$type>(),
                ) {
                    let bytes = value.to_be_bytes();
                    let mut reader = &bytes[..available];

                    let error = reader.decode_value::<$type>().unwrap_err();

                    prop_assert_eq!(error.kind(), UnexpectedEof);
                }
            }
        };
    }

    integer_decode_properties!(read_u32_success, read_u32_failed, u32);
    integer_decode_properties!(read_i32_success, read_i32_failed, i32);
    integer_decode_properties!(read_u16_success, read_u16_failed, u16);
    integer_decode_properties!(read_i16_success, read_i16_failed, i16);
    integer_decode_properties!(read_u8_success, read_u8_failed, u8);
    integer_decode_properties!(read_i8_success, read_i8_failed, i8);

    proptest! {
        #[test]
        fn read_bytes_vec_success(
            expected in prop::collection::vec(any::<u8>(), 0..64),
            suffix in prop::collection::vec(any::<u8>(), 0..16),
        ) {
            let mut bytes = expected.clone();
            bytes.extend(&suffix);
            let mut reader = bytes.as_slice();

            let decoded = super::read_vec(&mut reader, expected.len()).unwrap();

            prop_assert_eq!(decoded, expected);
            prop_assert_eq!(reader, suffix.as_slice());
        }

        #[test]
        fn read_bytes_vec_failed(
            available in prop::collection::vec(any::<u8>(), 0..64),
            missing in 1usize..16,
        ) {
            let requested = available.len() + missing;
            let mut reader = available.as_slice();

            let error = super::read_vec(&mut reader, requested).unwrap_err();

            prop_assert_eq!(error.kind(), UnexpectedEof);
        }
    }
}
