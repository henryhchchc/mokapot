use std::{io::Read, usize};

use crate::jvm::code::ProgramCounter;

pub(crate) trait ValueReaderExt
where
    Self: Read + Sized,
{
    fn read_value<T: Readable>(&mut self) -> std::io::Result<T>;
}
pub(crate) trait Readable {
    fn read_from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized;
}

impl<R: Read + Sized> ValueReaderExt for R {
    fn read_value<T: Readable>(&mut self) -> std::io::Result<T>
    where
        T: Readable,
    {
        T::read_from_reader(self)
    }
}

impl<const N: usize> Readable for [u8; N] {
    fn read_from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut buf = [0u8; N];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl Readable for ProgramCounter {
    fn read_from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let inner = u16::read_from_reader(reader)?;
        Ok(inner.into())
    }
}

macro_rules! impl_readable_for {
    ($($t:ty),*) => {
        $(
            impl Readable for $t {
                fn read_from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
                    let buf = reader.read_value()?;
                    Ok(Self::from_be_bytes(buf))
                }
            }
        )*
    };
}

impl_readable_for!(u8, u16, u32, i8, i16, i32, i64, f32, f64);

/// Reads [len] bytes and advances the reader by [`len`] bytes.
pub(crate) fn read_byte_chunk<R>(reader: &mut R, len: usize) -> std::io::Result<Vec<u8>>
where
    R: Read,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(buf.as_mut_slice())?;
    Ok(buf)
}

#[cfg(test)]
mod test {
    use super::ValueReaderExt;

    #[test]
    fn read_bytes_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = reader.read_value().unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04u8]);
    }

    #[test]
    fn read_bytes_failed() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let err = reader.read_value::<[u8; 3]>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02]);
    }

    #[test]
    fn read_u32_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: u32 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102_0304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u32_failed() {
        let mut reader = [0x01u8, 0x02, 0x03].as_slice();
        let err = reader.read_value::<u32>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02, 0x03]);
    }

    #[test]
    fn read_i32_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: i32 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102_0304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i32_failed() {
        let mut reader = [0x01u8, 0x02, 0x03].as_slice();
        let err = reader.read_value::<i32>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02, 0x03]);
    }

    #[test]
    fn read_u16_success() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let buf: u16 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u16_failed() {
        let mut reader = [0x01u8].as_slice();
        let err = reader.read_value::<u16>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8]);
    }

    #[test]
    fn read_i16_success() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let buf: i16 = reader.read_value().unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i16_failed() {
        let mut reader = [0x01u8].as_slice();
        let err = reader.read_value::<i16>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8]);
    }

    #[test]
    fn read_u8_success() {
        let mut reader = [0x01u8].as_slice();
        let buf: u8 = reader.read_value().unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u8_failed() {
        let mut reader = [].as_slice();
        let err = reader.read_value::<u8>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_success() {
        let mut reader = [0x01u8].as_slice();
        let buf: i8 = reader.read_value().unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_failed() {
        let mut reader = [].as_slice();
        let err = reader.read_value::<u32>().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_bytes_vec_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: Vec<u8> = super::read_byte_chunk(&mut reader, 3).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04u8]);
    }

    #[test]
    fn read_bytes_vec_failed() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let err = super::read_byte_chunk(&mut reader, 3).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02]);
    }
}
