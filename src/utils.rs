use std::usize;

use crate::elements::{
    class_parser::ClassFileParsingError, instruction::ProgramCounter,
};

/// Reads a u32 and advances the reader by 4 bytes.
pub(crate) fn read_u32<R>(reader: &mut R) -> std::io::Result<u32>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

/// Reads a i32 and advances the reader by 4 bytes.
pub(crate) fn read_i32<R>(reader: &mut R) -> std::io::Result<i32>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

/// Reada a u16 and advances the reader by 2 bytes.
pub(crate) fn read_u16<R>(reader: &mut R) -> std::io::Result<u16>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

/// Reads a i16 and advances the reader by 2 bytes.
pub(crate) fn read_i16<R>(reader: &mut R) -> std::io::Result<i16>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

/// Reads [N] bytes and advances the reader by [N] bytes.
pub(crate) fn read_bytes<R, const N: usize>(reader: &mut R) -> std::io::Result<[u8; N]>
where
    R: std::io::Read,
{
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads [len] bytes and advances the reader by [len] bytes.
pub(crate) fn read_bytes_vec<R>(reader: &mut R, len: usize) -> std::io::Result<Vec<u8>>
where
    R: std::io::Read,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

/// Reads a u8 and advances the reader by 1 byte.
pub(crate) fn read_u8<R>(reader: &mut R) -> std::io::Result<u8>
where
    R: std::io::Read,
{
    let mut buf: [u8; 1] = [0];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

/// Reads a i8 and advances the reader by 1 byte.
pub(crate) fn read_i8<R>(reader: &mut R) -> std::io::Result<i8>
where
    R: std::io::Read,
{
    let mut buf: [u8; 1] = [0];
    reader.read_exact(&mut buf)?;
    Ok(i8::from_be_bytes(buf))
}

/// Reads an i32 offset form the reader, advances the reader by 4 bytes, and applies the offset to [current_pc].
pub(crate) fn read_offset32<R>(
    reader: &mut R,
    current_pc: u16,
) -> Result<ProgramCounter, ClassFileParsingError>
where
    R: std::io::Read,
{
    let offset = read_i32(reader)?;
    let target_pc_signed = (current_pc as i32) + offset;
    u16::try_from(target_pc_signed)
        .map(ProgramCounter)
        .map_err(|_| ClassFileParsingError::InvalidJumpTarget)
}

/// Reads an i16 offset form the reader, advances the reader by 2 bytes, and applies the offset to [current_pc].
pub(crate) fn read_offset16<R>(
    reader: &mut R,
    current_pc: u16,
) -> Result<ProgramCounter, ClassFileParsingError>
where
    R: std::io::Read,
{
    let offset = read_i16(reader)?;
    let target_pc_signed = (current_pc as i16) + offset;
    u16::try_from(target_pc_signed)
        .map(ProgramCounter)
        .map_err(|_| ClassFileParsingError::InvalidJumpTarget)
}
