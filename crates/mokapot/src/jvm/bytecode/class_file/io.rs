use zerocopy::FromBytes;
use zerocopy::byteorder::big_endian::{U16, U32};

use std::io::{self, Read};

use super::{ClassFile, JAVA_CLASS_MAGIC};
use crate::jvm::bytecode::{
    FromBytecode, attribute::AttributeInfo, constant_pool::RawConstantPool, field_info::FieldInfo,
    method_info::MethodInfo,
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
