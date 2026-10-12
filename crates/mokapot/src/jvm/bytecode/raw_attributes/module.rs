//! Raw attribute types for module-related attributes.

use std::{
    io::{self, Read, Write},
    result::Result,
};

use zerocopy::{
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, byteorder::big_endian::U16,
};

use super::super::fixed_layout::FixedLayout;

use super::super::{
    FromBytecode, GenerationError, ToBytecode, reader::BytecodeReader, write_length,
};
use crate::intrinsics::see_jvm_spec;

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ModuleInfo {
    pub module_name_index: u16,
    pub module_flags: u16,
    pub module_version_index: u16,
    pub requires: Vec<RequiresInfo>,
    pub exports: Vec<ExportsInfo>,
    pub opens: Vec<OpensInfo>,
    pub uses: Vec<u16>,
    pub provides: Vec<ProvidesInfo>,
}

impl FromBytecode for ModuleInfo {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let module_name_index = U16::read_from_io(&mut reader)?.get();
        let module_flags = U16::read_from_io(&mut reader)?.get();
        let module_version_index = U16::read_from_io(&mut reader)?.get();
        let requires_count: u16 = U16::read_from_io(&mut reader)?.get();
        let requires = (0..requires_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let exports_count: u16 = U16::read_from_io(&mut reader)?.get();
        let exports = (0..exports_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let opens_count: u16 = U16::read_from_io(&mut reader)?.get();
        let opens = (0..opens_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let uses_count: u16 = U16::read_from_io(&mut reader)?.get();
        let uses = (0..uses_count)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
        let provides_count: u16 = U16::read_from_io(&mut reader)?.get();
        let provides = (0..provides_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        Ok(Self {
            module_name_index,
            module_flags,
            module_version_index,
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
        writer.write_all(&self.module_name_index.to_be_bytes())?;
        writer.write_all(&self.module_flags.to_be_bytes())?;
        writer.write_all(&self.module_version_index.to_be_bytes())?;
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

#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct RequiresInfo {
    pub requires_index: U16,
    pub requires_flags: U16,
    pub requires_version_index: U16,
}

impl FixedLayout for RequiresInfo {}

#[doc = see_jvm_spec!(4, 7, 25)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct ExportsInfo {
    pub exports_index: u16,
    pub exports_flags: u16,
    pub exports_to: Box<[u16]>,
}

impl FromBytecode for ExportsInfo {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let exports_index = U16::read_from_io(&mut reader)?.get();
        let exports_flags = U16::read_from_io(&mut reader)?.get();
        let exports_to_count: u16 = U16::read_from_io(&mut reader)?.get();
        let exports_to = (0..exports_to_count)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            exports_index,
            exports_flags,
            exports_to,
        })
    }
}

impl ToBytecode for ExportsInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.exports_index.to_be_bytes())?;
        writer.write_all(&self.exports_flags.to_be_bytes())?;
        write_length::<u16>(writer, self.exports_to.len())?;
        for module_index in &self.exports_to {
            writer.write_all(&module_index.to_be_bytes())?;
        }
        Ok(())
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
#[expect(
    clippy::struct_field_names,
    reason = "Use JVMS item names in raw layouts"
)]
pub struct OpensInfo {
    pub opens_index: u16,
    pub opens_flags: u16,
    pub opens_to: Vec<u16>,
}

impl FromBytecode for OpensInfo {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let opens_index = U16::read_from_io(&mut reader)?.get();
        let opens_flags = U16::read_from_io(&mut reader)?.get();
        let opens_to_count: u16 = U16::read_from_io(&mut reader)?.get();
        let opens_to = (0..opens_to_count)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            opens_index,
            opens_flags,
            opens_to,
        })
    }
}

impl ToBytecode for OpensInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.opens_index.to_be_bytes())?;
        writer.write_all(&self.opens_flags.to_be_bytes())?;
        write_length::<u16>(writer, self.opens_to.len())?;
        for module_index in &self.opens_to {
            writer.write_all(&module_index.to_be_bytes())?;
        }
        Ok(())
    }
}

#[doc = see_jvm_spec!(4, 7, 25)]
pub struct ProvidesInfo {
    pub provides_index: u16,
    pub provides_with: Vec<u16>,
}

impl FromBytecode for ProvidesInfo {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let provides_index = U16::read_from_io(&mut reader)?.get();
        let provides_with_count: u16 = U16::read_from_io(&mut reader)?.get();
        let provides_with = (0..provides_with_count)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
        Ok(Self {
            provides_index,
            provides_with,
        })
    }
}

impl ToBytecode for ProvidesInfo {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&self.provides_index.to_be_bytes())?;
        write_length::<u16>(writer, self.provides_with.len())?;
        for class_index in &self.provides_with {
            writer.write_all(&class_index.to_be_bytes())?;
        }
        Ok(())
    }
}
