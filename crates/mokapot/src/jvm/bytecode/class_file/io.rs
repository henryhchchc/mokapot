use std::io::{self, Read, Write};

use super::{ClassFile, JAVA_CLASS_MAGIC};
use crate::jvm::{
    bytecode::{
        FromBytecode, GenerationError, ToBytecode, attribute::AttributeInfo, field_info::FieldInfo,
        method_info::MethodInfo, reader::BytecodeReader, write_length,
    },
    class::ConstantPool,
};

impl FromBytecode for ClassFile {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let magic: u32 = reader.decode_value()?;
        if magic != JAVA_CLASS_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "This is not a Java class file",
            ));
        }
        let minor_version = reader.decode_value()?;
        let major_version = reader.decode_value()?;
        let constant_pool_count = reader.decode_value()?;
        let constant_pool = ConstantPool::from_reader(reader, constant_pool_count)?;
        let access_flags = reader.decode_value()?;
        let this_class = reader.decode_value()?;
        let super_class = reader.decode_value()?;
        let interfaces_count: u16 = reader.decode_value()?;
        let interfaces = (0..interfaces_count)
            .map(|_| reader.decode_value())
            .collect::<io::Result<_>>()?;
        let fields_count: u16 = reader.decode_value()?;
        let fields = (0..fields_count)
            .map(|_| FieldInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;
        let methods_count: u16 = reader.decode_value()?;
        let methods = (0..methods_count)
            .map(|_| MethodInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;
        let attributes_count: u16 = reader.decode_value()?;
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_reader(reader))
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
