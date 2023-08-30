use std::usize;

use crate::elements::{
    class_parser::ClassFileParsingError, instruction::ProgramCounter,
};

pub(crate) fn read_u32<R>(reader: &mut R) -> std::io::Result<u32>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub(crate) fn read_i32<R>(reader: &mut R) -> std::io::Result<i32>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_be_bytes(buf))
}

pub(crate) fn read_u16<R>(reader: &mut R) -> std::io::Result<u16>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub(crate) fn read_i16<R>(reader: &mut R) -> std::io::Result<i16>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_be_bytes(buf))
}

pub(crate) fn read_bytes<R, const N: usize>(reader: &mut R) -> std::io::Result<[u8; N]>
where
    R: std::io::Read,
{
    let mut buf = [0u8; N];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub(crate) fn read_bytes_vec<R>(reader: &mut R, len: usize) -> std::io::Result<Vec<u8>>
where
    R: std::io::Read,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub(crate) fn read_u8<R>(reader: &mut R) -> std::io::Result<u8>
where
    R: std::io::Read,
{
    let mut buf: [u8; 1] = [0];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

pub(crate) fn read_i8<R>(reader: &mut R) -> std::io::Result<i8>
where
    R: std::io::Read,
{
    let mut buf: [u8; 1] = [0];
    reader.read_exact(&mut buf)?;
    Ok(i8::from_be_bytes(buf))
}

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
