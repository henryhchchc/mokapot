use zerocopy::FromBytes;
use zerocopy::byteorder::big_endian::{U16, U32};

use std::io::{self, Read, Write};

use super::{ClassFile, JAVA_CLASS_MAGIC};
use crate::jvm::bytecode::{
    FromBytecode, GenerationError, ToBytecode, attribute::AttributeInfo,
    constant_pool::RawConstantPool, field_info::FieldInfo, method_info::MethodInfo, write_length,
};

impl FromBytecode for ClassFile {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let magic: u32 = U32::read_from_io(&mut reader)?.get();
        if magic != JAVA_CLASS_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "This is not a Java class file",
            ));
        }
        let minor_version = U16::read_from_io(&mut reader)?.get();
        let major_version = U16::read_from_io(&mut reader)?.get();
        let constant_pool_count = U16::read_from_io(&mut reader)?.get();
        let constant_pool = RawConstantPool::from_reader(&mut reader, constant_pool_count)?;
        let access_flags = U16::read_from_io(&mut reader)?.get();
        let this_class = U16::read_from_io(&mut reader)?.get();
        let super_class = U16::read_from_io(&mut reader)?.get();
        let interfaces_count: u16 = U16::read_from_io(&mut reader)?.get();
        let interfaces = (0..interfaces_count)
            .map(|_| U16::read_from_io(&mut reader).map(u16::from))
            .collect::<io::Result<_>>()?;
        let fields_count: u16 = U16::read_from_io(&mut reader)?.get();
        let fields = (0..fields_count)
            .map(|_| FieldInfo::from_reader(&mut reader))
            .collect::<io::Result<_>>()?;
        let methods_count: u16 = U16::read_from_io(&mut reader)?.get();
        let methods = (0..methods_count)
            .map(|_| MethodInfo::from_reader(&mut reader))
            .collect::<io::Result<_>>()?;
        let attributes_count: u16 = U16::read_from_io(&mut reader)?.get();
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_reader(&mut reader))
            .collect::<io::Result<_>>()?;

        Ok(Self {
            minor_version,
            major_version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
    }
}

impl ToBytecode for ClassFile {
    fn to_writer<W: Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        writer.write_all(&JAVA_CLASS_MAGIC.to_be_bytes())?;
        writer.write_all(&self.minor_version.to_be_bytes())?;
        writer.write_all(&self.major_version.to_be_bytes())?;
        self.constant_pool.to_writer(writer)?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        writer.write_all(&self.this_class.to_be_bytes())?;
        writer.write_all(&self.super_class.to_be_bytes())?;
        write_length::<u16>(writer, self.interfaces.len())?;
        for interface_idx in &self.interfaces {
            writer.write_all(&interface_idx.to_be_bytes())?;
        }
        write_length::<u16>(writer, self.fields.len())?;
        for field in &self.fields {
            field.to_writer(writer)?;
        }
        write_length::<u16>(writer, self.methods.len())?;
        for method in &self.methods {
            method.to_writer(writer)?;
        }
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_magic() {
        let reader = io::Cursor::new([0, 0, 0, 0]);
        let error = ClassFile::from_reader(reader).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
