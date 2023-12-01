use std::{io::Read, usize};

use super::{errors::ClassFileParsingError, parsing_context::ParsingContext};

/// Reads [N] bytes and advances the reader by [N] bytes.
pub(crate) fn read_bytes<R, const N: usize>(reader: &mut R) -> std::io::Result<[u8; N]>
where
    R: Read,
{
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads a u32 and advances the reader by 4 bytes.
pub(crate) fn read_u32<R>(reader: &mut R) -> std::io::Result<u32>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(u32::from_be_bytes(buf))
}

/// Reads a i32 and advances the reader by 4 bytes.
pub(crate) fn read_i32<R>(reader: &mut R) -> std::io::Result<i32>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(i32::from_be_bytes(buf))
}

/// Reada a u16 and advances the reader by 2 bytes.
pub(crate) fn read_u16<R>(reader: &mut R) -> std::io::Result<u16>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(u16::from_be_bytes(buf))
}

/// Reads a i16 and advances the reader by 2 bytes.
pub(crate) fn read_i16<R>(reader: &mut R) -> std::io::Result<i16>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(i16::from_be_bytes(buf))
}

/// Reads a u8 and advances the reader by 1 byte.
pub(crate) fn read_u8<R>(reader: &mut R) -> std::io::Result<u8>
where
    R: Read,
{
    let buf: [u8; 1] = read_bytes(reader)?;
    Ok(buf[0])
}

/// Reads a i8 and advances the reader by 1 byte.
pub(crate) fn read_i8<R>(reader: &mut R) -> std::io::Result<i8>
where
    R: Read,
{
    let buf: [u8; 1] = read_bytes(reader)?;
    Ok(i8::from_be_bytes(buf))
}

/// Reads [len] bytes and advances the reader by [`len`] bytes.
pub(crate) fn read_bytes_vec<R>(reader: &mut R, len: usize) -> std::io::Result<Vec<u8>>
where
    R: Read,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(buf.as_mut_slice())?;
    Ok(buf)
}

pub(crate) fn parse_multiple<R, T, P>(
    reader: &mut R,
    ctx: &ParsingContext,
    parse: P,
) -> Result<Vec<T>, ClassFileParsingError>
where
    R: std::io::Read,
    P: Fn(&mut R, &ParsingContext) -> Result<T, ClassFileParsingError>,
{
    use std::iter::repeat_with;

    let count = read_u16(reader)?;
    repeat_with(|| parse(reader, ctx))
        .take(count as usize)
        .collect::<Result<_, _>>()
}

#[cfg(test)]
mod test {

    #[test]
    fn read_bytes_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: [u8; 3] = super::read_bytes(&mut reader).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04u8]);
    }

    #[test]
    fn read_bytes_failed() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let err = super::read_bytes::<_, 3>(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02]);
    }

    #[test]
    fn read_u32_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: u32 = super::read_u32(&mut reader).unwrap();
        assert_eq!(buf, 0x01020304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u32_failed() {
        let mut reader = [0x01u8, 0x02, 0x03].as_slice();
        let err = super::read_u32(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02, 0x03]);
    }

    #[test]
    fn read_i32_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: i32 = super::read_i32(&mut reader).unwrap();
        assert_eq!(buf, 0x01020304);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i32_failed() {
        let mut reader = [0x01u8, 0x02, 0x03].as_slice();
        let err = super::read_i32(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02, 0x03]);
    }

    #[test]
    fn read_u16_success() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let buf: u16 = super::read_u16(&mut reader).unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u16_failed() {
        let mut reader = [0x01u8].as_slice();
        let err = super::read_u16(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8]);
    }

    #[test]
    fn read_i16_success() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let buf: i16 = super::read_i16(&mut reader).unwrap();
        assert_eq!(buf, 0x0102);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i16_failed() {
        let mut reader = [0x01u8].as_slice();
        let err = super::read_i16(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8]);
    }

    #[test]
    fn read_u8_success() {
        let mut reader = [0x01u8].as_slice();
        let buf: u8 = super::read_u8(&mut reader).unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_u8_failed() {
        let mut reader = [].as_slice();
        let err = super::read_u8(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_success() {
        let mut reader = [0x01u8].as_slice();
        let buf: i8 = super::read_i8(&mut reader).unwrap();
        assert_eq!(buf, 0x01);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_i8_failed() {
        let mut reader = [].as_slice();
        let err = super::read_i8(&mut reader).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert!(reader.is_empty());
    }

    #[test]
    fn read_bytes_vec_success() {
        let mut reader = [0x01u8, 0x02, 0x03, 0x04].as_slice();
        let buf: Vec<u8> = super::read_bytes_vec(&mut reader, 3).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);
        assert_eq!(reader, [0x04u8]);
    }

    #[test]
    fn read_bytes_vec_failed() {
        let mut reader = [0x01u8, 0x02].as_slice();
        let err = super::read_bytes_vec(&mut reader, 3).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
        assert_eq!(reader, [0x01u8, 0x02]);
    }
}
