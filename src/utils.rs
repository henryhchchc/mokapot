pub(crate) fn read_u32<R>(reader: &mut R) -> std::io::Result<u32>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

pub(crate) fn read_u16<R>(reader: &mut R) -> std::io::Result<u16>
where
    R: std::io::Read,
{
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

pub(crate) fn read_bytes<R>(reader: &mut R, len: usize) -> std::io::Result<Vec<u8>>
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
