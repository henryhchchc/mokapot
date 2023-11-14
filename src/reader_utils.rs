use std::{
    io::{Read, Result},
    usize,
};

/// Reads [N] bytes and advances the reader by [N] bytes.
pub(crate) fn read_bytes<R, const N: usize>(reader: &mut R) -> Result<[u8; N]>
where
    R: Read,
{
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads a u32 and advances the reader by 4 bytes.
pub(crate) fn read_u32<R>(reader: &mut R) -> Result<u32>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(u32::from_be_bytes(buf))
}

/// Reads a i32 and advances the reader by 4 bytes.
pub(crate) fn read_i32<R>(reader: &mut R) -> Result<i32>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(i32::from_be_bytes(buf))
}

/// Reada a u16 and advances the reader by 2 bytes.
pub(crate) fn read_u16<R>(reader: &mut R) -> Result<u16>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(u16::from_be_bytes(buf))
}

/// Reads a i16 and advances the reader by 2 bytes.
pub(crate) fn read_i16<R>(reader: &mut R) -> Result<i16>
where
    R: Read,
{
    let buf = read_bytes(reader)?;
    Ok(i16::from_be_bytes(buf))
}

/// Reads a u8 and advances the reader by 1 byte.
pub(crate) fn read_u8<R>(reader: &mut R) -> Result<u8>
where
    R: Read,
{
    let buf: [u8; 1] = read_bytes(reader)?;
    Ok(buf[0])
}

/// Reads a i8 and advances the reader by 1 byte.
pub(crate) fn read_i8<R>(reader: &mut R) -> Result<i8>
where
    R: Read,
{
    let buf: [u8; 1] = read_bytes(reader)?;
    Ok(i8::from_be_bytes(buf))
}

/// Reads [len] bytes and advances the reader by [`len`] bytes.
pub(crate) fn read_bytes_vec<R>(reader: &mut R, len: usize) -> Result<Vec<u8>>
where
    R: Read,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(buf.as_mut_slice())?;
    Ok(buf)
}
