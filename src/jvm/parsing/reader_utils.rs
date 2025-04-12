use std::io::{Read, Result, Write};

use crate::jvm::code::ProgramCounter;

pub(super) trait ValueReaderExt: Read {
    fn read_value<T: FromReader>(&mut self) -> Result<T>;
}
pub(super) trait FromReader {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self>
    where
        Self: Sized;
}

impl<R: Read + ?Sized> ValueReaderExt for R {
    fn read_value<T: FromReader>(&mut self) -> Result<T> {
        T::from_reader(self)
    }
}

impl<const N: usize> FromReader for [u8; N] {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; N];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl FromReader for ProgramCounter {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let inner = u16::from_reader(reader)?;
        Ok(inner.into())
    }
}

#[trait_gen::trait_gen(T ->
    u8, u16, u32, i8, i16, i32, i64, f32, f64
)]
impl FromReader for T {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

/// Reads [len] bytes and advances the reader by [`len`] bytes.
pub(super) fn read_byte_chunk<R>(reader: &mut R, len: usize) -> Result<Vec<u8>>
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
        let n = self.inner.read(buf)?;
        self.position += n;
        Ok(n)
    }
}

impl<W> Write for PositionTracker<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let n = self.inner.write(buf)?;
        self.position += n;
        Ok(n)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod test {
    use std::io::ErrorKind::UnexpectedEof;

    use super::ValueReaderExt;

    #[test]
    fn read_bytes_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = reader.read_value().unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04]);
    }

    #[test]
    fn read_bytes_failed() {
        let mut reader = [0x01, 0x02].as_slice();
        let err = reader.read_value::<[u8; 3]>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_u32_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: u32 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102_0304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u32_failed() {
        let mut reader = [0x01, 0x02, 0x03].as_slice();
        let err = reader.read_value::<u32>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_i32_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: i32 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102_0304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i32_failed() {
        let mut reader = [0x01, 0x02, 0x03].as_slice();
        let err = reader.read_value::<i32>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_u16_success() {
        let mut reader = [0x01, 0x02].as_slice();
        let buf: u16 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u16_failed() {
        let mut reader = [0x01].as_slice();
        let err = reader.read_value::<u16>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_i16_success() {
        let mut reader = [0x01, 0x02].as_slice();
        let buf: i16 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i16_failed() {
        let mut reader = [0x01].as_slice();
        let err = reader.read_value::<i16>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_u8_success() {
        let mut reader = [0x01].as_slice();
        let buf: u8 = reader.read_value().unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u8_failed() {
        let mut reader = [].as_slice();
        let err = reader.read_value::<u8>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_success() {
        let mut reader = [0x01].as_slice();
        let buf: i8 = reader.read_value().unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_failed() {
        let mut reader = [].as_slice();
        let err = reader.read_value::<u32>().unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }

    #[test]
    fn read_bytes_vec_success() {
        let mut reader = [0x01, 0x02, 0x03, 0x04].as_slice();
        let buf: Vec<u8> = super::read_byte_chunk(&mut reader, 3).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04]);
    }

    #[test]
    fn read_bytes_vec_failed() {
        let mut reader = [0x01, 0x02].as_slice();
        let err = super::read_byte_chunk(&mut reader, 3).unwrap_err();
        assert_eq!(err.kind(), UnexpectedEof);
    }
}
