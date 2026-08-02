//! Raw attribute types for module-related attributes.

use std::io::{self, Read, Write};
use std::result::Result;

use super::super::{
    FromBytecode, ToBytecode, errors::GenerationError, reader_utils::BytecodeReader, write_length,
};

pub struct ModuleInfo {
    pub info_index: u16,
    pub flags: u16,
    pub version_index: u16,
    pub requires: Vec<RequiresInfo>,
    pub exports: Vec<ExportsInfo>,
    pub opens: Vec<OpensInfo>,
    pub uses: Vec<u16>,
    pub provides: Vec<ProvidesInfo>,
}

impl FromBytecode for ModuleInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let info_index = reader.decode_value()?;
        let flags = reader.decode_value()?;
        let version_index = reader.decode_value()?;
        let requires_count: u16 = reader.decode_value()?;
        let requires = (0..requires_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let exports_count: u16 = reader.decode_value()?;
        let exports = (0..exports_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let opens_count: u16 = reader.decode_value()?;
        let opens = (0..opens_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let uses_count: u16 = reader.decode_value()?;
        let uses = (0..uses_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let provides_count: u16 = reader.decode_value()?;
        let provides = (0..provides_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            info_index,
            flags,
            version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}

impl ToBytecode for ModuleInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.info_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.version_index.to_be_bytes())?;
        write_length::<u16>(writer, self.requires.len())?;
        for require in &self.requires {
            require.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.exports.len())?;
        for export in &self.exports {
            export.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.opens.len())?;
        for open in &self.opens {
            open.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.uses.len())?;
        for use_ in &self.uses {
            writer.write_all(&use_.to_be_bytes())?;
        }
        write_length::<u16>(writer, self.provides.len())?;
        for provide in &self.provides {
            provide.to_writer(writer)?;
        }
        Ok(())
    }
}

pub struct RequiresInfo {
    pub requires_index: u16,
    pub flags: u16,
    pub version_index: u16,
}

impl FromBytecode for RequiresInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        Ok(Self {
            requires_index: reader.decode_value()?,
            flags: reader.decode_value()?,
            version_index: reader.decode_value()?,
        })
    }
}

impl ToBytecode for RequiresInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.requires_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        writer.write_all(&self.version_index.to_be_bytes())?;
        Ok(())
    }
}

pub struct ExportsInfo {
    pub exports_index: u16,
    pub flags: u16,
    pub to: Vec<u16>,
}

impl FromBytecode for ExportsInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let exports_index = reader.decode_value()?;
        let flags = reader.decode_value()?;
        let to_count: u16 = reader.decode_value()?;
        let to = (0..to_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            exports_index,
            flags,
            to,
        })
    }
}

impl ToBytecode for ExportsInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.exports_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        write_length::<u16>(writer, self.to.len())?;
        for to in &self.to {
            writer.write_all(&to.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct OpensInfo {
    pub opens_index: u16,
    pub flags: u16,
    pub to: Vec<u16>,
}

impl FromBytecode for OpensInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let opens_index = reader.decode_value()?;
        let flags = reader.decode_value()?;
        let to_count: u16 = reader.decode_value()?;
        let to = (0..to_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            opens_index,
            flags,
            to,
        })
    }
}

impl ToBytecode for OpensInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.opens_index.to_be_bytes())?;
        writer.write_all(&self.flags.to_be_bytes())?;
        write_length::<u16>(writer, self.to.len())?;
        for to in &self.to {
            writer.write_all(&to.to_be_bytes())?;
        }
        Ok(())
    }
}

pub struct ProvidesInfo {
    pub provides_index: u16,
    pub with: Vec<u16>,
}

impl FromBytecode for ProvidesInfo {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let provides_index = reader.decode_value()?;
        let with_count: u16 = reader.decode_value()?;
        let with = (0..with_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            provides_index,
            with,
        })
    }
}

impl ToBytecode for ProvidesInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.provides_index.to_be_bytes())?;
        write_length::<u16>(writer, self.with.len())?;
        for with in &self.with {
            writer.write_all(&with.to_be_bytes())?;
        }
        Ok(())
    }
}
