use std::{io::Read, usize};

pub(crate) trait ClassReader
where
    Self: Read + Sized,
{
    fn read_value<T>(&mut self) -> std::io::Result<T>
    where
        T: ReadFromReader<Self>,
    {
        T::read_from_reader(self)
    }
}
pub(crate) trait ReadFromReader<R: Read> {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self>
    where
        Self: Sized;
}

impl<T> ClassReader for T where T: Read + Sized {}

impl<R: Read, const N: usize> ReadFromReader<R> for [u8; N] {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let mut buf = [0u8; N];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}

impl<R: Read> ReadFromReader<R> for u8 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for u16 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for u32 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for i8 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for i16 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for i32 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for i64 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}
impl<R: Read> ReadFromReader<R> for f32 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

impl<R: Read> ReadFromReader<R> for f64 {
    fn read_from_reader(reader: &mut R) -> std::io::Result<Self> {
        let buf = reader.read_value()?;
        Ok(Self::from_be_bytes(buf))
    }
}

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
    use crate::jvm::parsing::reader_utils::ClassReader;

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
        assert_eq!(buf, 0x01020304);
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
        assert_eq!(buf, 0x01020304);
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
