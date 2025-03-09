use std::{
    io::{self, Read},
    str::FromStr,
};

use super::{
    Error, ToWriter,
    reader_utils::{ValueReaderExt, read_byte_chunk},
    write_length,
};
use crate::{
    jvm::{
        ConstantValue, JavaString,
        class::{ConstantPool, MethodHandle, constant_pool::Entry},
        references::{ClassRef, FieldRef, MethodRef, ModuleRef, PackageRef},
    },
    macros::malform,
    types::field_type::FieldType,
};

#[inline]
const fn mismatch<T>(expected: &'static str, entry: &Entry) -> Result<T, Error> {
    Err(Error::MismatchedConstantPoolEntryType {
        expected,
        found: entry.constant_kind(),
    })
}

impl ConstantPool {
    pub(super) fn get_str(&self, index: u16) -> Result<&str, Error> {
        let entry = self.get_entry(index)?;
        match entry {
            Entry::Utf8(JavaString::Utf8(string)) => Ok(string),
            Entry::Utf8(JavaString::InvalidUtf8(_)) => Err(Error::BrokenUTF8),
            it => mismatch("Utf8", it),
        }
    }

    pub(super) fn put_string(&mut self, value: String) -> Result<u16, Error> {
        let entry = Entry::Utf8(JavaString::Utf8(value));
        self.put_entry(entry).map_err(Into::into)
    }

    pub(super) fn get_class_ref(&self, index: u16) -> Result<ClassRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Class { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(ClassRef::new(name))
        } else {
            mismatch("Class", entry)
        }
    }

    pub(super) fn put_class_ref(&mut self, value: ClassRef) -> Result<u16, Error> {
        let name_index = self.put_string(value.binary_name)?;
        let entry = Entry::Class { name_index };
        let idx = self.put_entry(entry)?;
        Ok(idx)
    }

    pub(super) fn put_field_ref(&mut self, value: FieldRef) -> Result<u16, Error> {
        let class_index = self.put_class_ref(value.owner)?;
        let name_and_type_index = self.put_name_and_type(value.name, value.field_type)?;
        self.put_entry(Entry::FieldRef {
            class_index,
            name_and_type_index,
        })
        .map_err(Into::into)
    }

    pub(super) fn put_method_ref(&mut self, value: MethodRef) -> Result<u16, Error> {
        let class_index = self.put_class_ref(value.owner)?;
        let name_and_type_index = self.put_name_and_type(value.name, value.descriptor)?;
        self.put_entry(Entry::MethodRef {
            class_index,
            name_and_type_index,
        })
        .map_err(Into::into)
    }

    pub(super) fn get_constant_value(&self, value_index: u16) -> Result<ConstantValue, Error> {
        let entry = self.get_entry(value_index)?;
        match *entry {
            Entry::Integer(it) => Ok(ConstantValue::Integer(it)),
            Entry::Long(it) => Ok(ConstantValue::Long(it)),
            Entry::Float(it) => Ok(ConstantValue::Float(it)),
            Entry::Double(it) => Ok(ConstantValue::Double(it)),
            Entry::String { string_index } => {
                if let Entry::Utf8(java_str) = self.get_entry(string_index)? {
                    Ok(ConstantValue::String(java_str.clone()))
                } else {
                    mismatch("Utf8", entry)
                }
            }
            Entry::MethodType { descriptor_index } => self
                .get_str(descriptor_index)
                .and_then(|it| it.parse().map_err(Into::into))
                .map(ConstantValue::MethodType),
            Entry::Class { name_index } => self
                .get_str(name_index)
                .map(ClassRef::new)
                .map(ConstantValue::Class),
            Entry::MethodHandle { .. } => self
                .get_method_handle(value_index)
                .map(ConstantValue::Handle),
            Entry::Dynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
                Ok(ConstantValue::Dynamic(
                    bootstrap_method_attr_index,
                    name,
                    descriptor,
                ))
            }
            ref unexpected => mismatch(
                concat!(
                    "Integer | Long | Float | Double | String ",
                    "| MethodType | Class | MethodHandle | Dynamic"
                ),
                unexpected,
            ),
        }
    }

    pub(super) fn put_constant_value(&mut self, value: ConstantValue) -> Result<u16, Error> {
        let entry = match value {
            ConstantValue::Integer(val) => Entry::Integer(val),
            ConstantValue::Long(val) => Entry::Long(val),
            ConstantValue::Float(val) => Entry::Float(val),
            ConstantValue::Double(val) => Entry::Double(val),
            ConstantValue::String(java_string) => {
                let utf8_entry = Entry::Utf8(java_string);
                let string_index = self.put_entry(utf8_entry)?;
                Entry::String { string_index }
            }
            ConstantValue::Class(value) => return self.put_class_ref(value),
            ConstantValue::Handle(method_handle) => return self.put_method_handle(method_handle),
            ConstantValue::MethodType(method_descriptor) => {
                let descriptor_index = self.put_string(method_descriptor.to_string())?;
                Entry::MethodType { descriptor_index }
            }
            ConstantValue::Dynamic(bsm_idx, name, field_type) => {
                let name_and_type_index = self.put_name_and_type(name, field_type)?;
                Entry::Dynamic {
                    bootstrap_method_attr_index: bsm_idx,
                    name_and_type_index,
                }
            }
            ConstantValue::Null => {
                return Err(Error::Other("Null should not be put into constant pull"));
            }
        };
        self.put_entry(entry).map_err(Into::into)
    }

    pub(super) fn get_module_ref(&self, index: u16) -> Result<ModuleRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Module { name_index } = entry {
            let name = self.get_str(name_index)?.to_owned();
            Ok(ModuleRef { name })
        } else {
            mismatch("Module", entry)
        }
    }

    pub(super) fn put_module_ref(&mut self, value: ModuleRef) -> Result<u16, Error> {
        let name_index = self.put_string(value.name)?;
        let entry = Entry::Module { name_index };
        self.put_entry(entry).map_err(Into::into)
    }

    pub(super) fn get_package_ref(&self, index: u16) -> Result<PackageRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Package { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(PackageRef {
                binary_name: name.to_owned(),
            })
        } else {
            mismatch("Package", entry)
        }
    }

    pub(super) fn put_package_ref(&mut self, value: PackageRef) -> Result<u16, Error> {
        let name_index = self.put_string(value.binary_name)?;
        let entry = Entry::Package { name_index };
        self.put_entry(entry).map_err(Into::into)
    }

    pub(super) fn get_field_ref(&self, index: u16) -> Result<FieldRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::FieldRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let owner = self.get_class_ref(class_index)?;
            let (name, field_type) = self.get_name_and_type(name_and_type_index)?;
            Ok(FieldRef {
                owner,
                name,
                field_type,
            })
        } else {
            mismatch("Field", entry)
        }
    }

    pub(super) fn get_name_and_type<Descriptor>(
        &self,
        index: u16,
    ) -> Result<(String, Descriptor), Error>
    where
        Descriptor: FromStr,
        <Descriptor as FromStr>::Err: Into<Error>,
    {
        let entry = self.get_entry(index)?;
        if let &Entry::NameAndType {
            name_index,
            descriptor_index,
        } = entry
        {
            let name = self.get_str(name_index)?;
            let descriptor = self
                .get_str(descriptor_index)?
                .parse()
                .map_err(Into::into)?;
            Ok((name.to_owned(), descriptor))
        } else {
            mismatch("NameAndType", entry)
        }
    }

    pub(super) fn put_name_and_type<T>(&mut self, name: String, descriptor: T) -> Result<u16, Error>
    where
        T: ToString,
    {
        let name_index = self.put_string(name)?;
        let descriptor_index = self.put_string(descriptor.to_string())?;
        self.put_entry(Entry::NameAndType {
            name_index,
            descriptor_index,
        })
        .map_err(Into::into)
    }

    pub(super) fn get_method_ref(&self, index: u16) -> Result<MethodRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::MethodRef {
            class_index,
            name_and_type_index,
        }
        | &Entry::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let owner = self.get_class_ref(class_index)?;
            let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
            Ok(MethodRef {
                owner,
                name,
                descriptor,
            })
        } else {
            mismatch("MethodRef | InterfaceMethodRef", entry)
        }
    }

    pub(super) fn get_method_handle(&self, index: u16) -> Result<MethodHandle, Error> {
        #[allow(clippy::enum_glob_use)]
        use MethodHandle::*;

        let entry = self.get_entry(index)?;
        let &Entry::MethodHandle {
            reference_kind,
            reference_index: idx,
        } = entry
        else {
            return mismatch("MethodHandle", entry);
        };
        match reference_kind {
            1 => self.get_field_ref(idx).map(RefGetField),
            2 => self.get_field_ref(idx).map(RefGetStatic),
            3 => self.get_field_ref(idx).map(RefPutField),
            4 => self.get_field_ref(idx).map(RefPutStatic),
            5 => self.get_method_ref(idx).map(RefInvokeVirtual),
            6 => self.get_method_ref(idx).map(RefInvokeStatic),
            7 => self.get_method_ref(idx).map(RefInvokeSpecial),
            8 => self.get_method_ref(idx).map(RefNewInvokeSpecial),
            9 => self.get_method_ref(idx).map(RefInvokeInterface),
            _ => malform!("Invalid reference kind in method handle"),
        }
    }

    pub(super) fn put_method_handle(&mut self, value: MethodHandle) -> Result<u16, Error> {
        let reference_kind = value.reference_kind();
        let reference_index = match value {
            MethodHandle::RefGetField(f)
            | MethodHandle::RefGetStatic(f)
            | MethodHandle::RefPutField(f)
            | MethodHandle::RefPutStatic(f) => self.put_field_ref(f)?,
            MethodHandle::RefInvokeVirtual(m)
            | MethodHandle::RefInvokeStatic(m)
            | MethodHandle::RefInvokeSpecial(m)
            | MethodHandle::RefNewInvokeSpecial(m)
            | MethodHandle::RefInvokeInterface(m) => self.put_method_ref(m)?,
        };
        self.put_entry(Entry::MethodHandle {
            reference_kind,
            reference_index,
        })
        .map_err(Into::into)
    }

    pub(super) fn get_type_ref(&self, index: u16) -> Result<FieldType, Error> {
        let ClassRef { binary_name: name } = self.get_class_ref(index)?;
        let field_type = if name.starts_with('[') {
            FieldType::from_str(name.as_str())?
        } else {
            FieldType::Object(ClassRef::new(name))
        };
        Ok(field_type)
    }
}

impl Entry {
    pub(crate) fn parse<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let tag: u8 = reader.read_value()?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => reader.read_value().map(Self::Integer),
            4 => reader.read_value().map(Self::Float),
            5 => reader.read_value().map(Self::Long),
            6 => reader.read_value().map(Self::Double),
            7 => reader
                .read_value()
                .map(|name_index| Self::Class { name_index }),
            8 => reader
                .read_value()
                .map(|string_index| Self::String { string_index }),
            9 => Ok(Self::FieldRef {
                class_index: reader.read_value()?,
                name_and_type_index: reader.read_value()?,
            }),
            10 => Ok(Self::MethodRef {
                class_index: reader.read_value()?,
                name_and_type_index: reader.read_value()?,
            }),
            11 => Ok(Self::InterfaceMethodRef {
                class_index: reader.read_value()?,
                name_and_type_index: reader.read_value()?,
            }),
            12 => Ok(Self::NameAndType {
                name_index: reader.read_value()?,
                descriptor_index: reader.read_value()?,
            }),
            15 => Ok(Self::MethodHandle {
                reference_kind: reader.read_value()?,
                reference_index: reader.read_value()?,
            }),
            16 => Ok(Self::MethodType {
                descriptor_index: reader.read_value()?,
            }),
            17 => Ok(Self::Dynamic {
                bootstrap_method_attr_index: reader.read_value()?,
                name_and_type_index: reader.read_value()?,
            }),
            18 => Ok(Self::InvokeDynamic {
                bootstrap_method_attr_index: reader.read_value()?,
                name_and_type_index: reader.read_value()?,
            }),
            19 => reader
                .read_value()
                .map(|name_index| Self::Module { name_index }),
            20 => reader
                .read_value()
                .map(|name_index| Self::Package { name_index }),
            it => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid constant pool tag: {it}"),
            )),
        }
    }

    fn parse_utf8<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let length: u16 = reader.read_value()?;
        let cesu8_content = read_byte_chunk(reader, length.into())?;
        match cesu8::from_java_cesu8(cesu8_content.as_slice()) {
            Ok(result) => Ok(Self::Utf8(JavaString::Utf8(result.into_owned()))),
            Err(_) => Ok(Self::Utf8(JavaString::InvalidUtf8(cesu8_content))),
        }
    }
}

impl ToWriter for JavaString {
    fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), super::ToWriterError> {
        match self {
            Self::Utf8(str) => {
                let crsu8_bytes = cesu8::to_java_cesu8(str.as_str());
                write_length::<u16, _>(writer, crsu8_bytes.len())?;
                writer.write_all(crsu8_bytes.as_ref())?;
            }
            Self::InvalidUtf8(bytes) => {
                write_length::<u16, _>(writer, bytes.len())?;
                writer.write_all(bytes)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {

    use crate::jvm::parsing::ToWriter;

    use super::*;
    use proptest::prelude::*;

    const MAX_BYTES: usize = 255;

    proptest! {

        #[test]
        fn parse_entry(entry in arb_constant_pool_info()) {
            let mut reader = entry.as_slice();
            let parsed = Entry::parse(&mut reader);
            let tag = entry.first().unwrap();
            match tag {
                1 => assert!(matches!(parsed, Ok(Entry::Utf8(_)))),
                3 => assert!(matches!(parsed, Ok(Entry::Integer(_)))),
                4 => assert!(matches!(parsed, Ok(Entry::Float(_)))),
                5 => assert!(matches!(parsed, Ok(Entry::Long(_)))),
                6 => assert!(matches!(parsed, Ok(Entry::Double(_)))),
                7 => assert!(matches!(parsed, Ok(Entry::Class { .. }))),
                8 => assert!(matches!(parsed, Ok(Entry::String { .. }))),
                9 => assert!(matches!(parsed, Ok(Entry::FieldRef { .. }))),
                10 => assert!(matches!(parsed, Ok(Entry::MethodRef { .. }))),
                11 => assert!(matches!(parsed, Ok(Entry::InterfaceMethodRef { .. }))),
                12 => assert!(matches!(parsed, Ok(Entry::NameAndType { .. }))),
                15 => assert!(matches!(parsed, Ok(Entry::MethodHandle { .. }))),
                16 => assert!(matches!(parsed, Ok(Entry::MethodType { .. }))),
                17 => assert!(matches!(parsed, Ok(Entry::Dynamic { .. }))),
                18 => assert!(matches!(parsed, Ok(Entry::InvokeDynamic { .. }))),
                19 => assert!(matches!(parsed, Ok(Entry::Module { .. }))),
                20 => assert!(matches!(parsed, Ok(Entry::Package { .. }))),
                _ => unreachable!()
            }
        }

        #[test]
        fn read_write((count, content) in arb_constant_pool_bytes()) {
            let mut reader = content.as_slice();
            let pool = ConstantPool::from_reader(&mut reader, count).unwrap();
            let mut buf = Vec::new();
            pool.to_writer(&mut buf)?;
            let (len_bytes, written) = buf.split_at(2);
            let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]);
            assert_eq!(len, count);
            assert_eq!(written, content);
        }
    }

    prop_compose! {
        pub fn arb_constant_pool_bytes()(
            entries in prop::collection::vec(arb_constant_pool_info(), 1..=50)
        ) -> (u16, Vec<u8>) {
            let count = {
                let mut len = entries.len();
                len += entries.iter().filter(|&it| {
                    it.first().is_some_and(|&it| it == 5 || it == 6)
                }).count();
                len += 1;
                u16::try_from(len).unwrap()
            };
            let bytes = entries.into_iter().flatten().collect();
            (count, bytes)
        }
    }

    prop_compose! {
        fn arb_constant_info_utf8()(
            content in prop::collection::vec(any::<u8>(), 1..=MAX_BYTES)
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(content.len() + 3);
            result.push(1);
            let len = u16::try_from(content.len()).unwrap();
            result.extend(len.to_be_bytes());
            result.extend(content);
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_integer()(
            value in any::<i32>()
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(3);
            result.extend(value.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_float()(
            value in any::<f32>()
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(4);
            result.extend(value.to_be_bytes());
            result
        }

    }

    prop_compose! {
        fn arb_constant_info_long()(
            value in any::<i64>()
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(9);
            result.push(5);
            result.extend(value.to_be_bytes());
            result
        }

    }

    prop_compose! {
        fn arb_constant_info_double()(
            value in any::<f64>()
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(9);
            result.push(6);
            result.extend(value.to_be_bytes());
            result
        }

    }

    prop_compose! {
        fn arb_constant_info_class()(
            name_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(3);
            result.push(7);
            result.extend(name_index.to_be_bytes());
            result
        }

    }

    prop_compose! {
        fn arb_constant_info_string()(
            string_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(3);
            result.push(8);
            result.extend(string_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_field_ref()(
            class_index in 1..=u16::MAX,
            name_and_type_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(9);
            result.extend(class_index.to_be_bytes());
            result.extend(name_and_type_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_method_ref()(
            class_index in 1..=u16::MAX,
            name_and_type_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(10);
            result.extend(class_index.to_be_bytes());
            result.extend(name_and_type_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_interface_method_ref()(
            class_index in 1..=u16::MAX,
            name_and_type_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(11);
            result.extend(class_index.to_be_bytes());
            result.extend(name_and_type_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_name_and_type()(
            name_index in 1..=u16::MAX,
            descriptor_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(12);
            result.extend(name_index.to_be_bytes());
            result.extend(descriptor_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_method_handle()(
            reference_kind in 1..=u8::MAX,
            reference_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(15);
            result.push(reference_kind);
            result.extend(reference_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_method_type()(
            descriptor_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(3);
            result.push(16);
            result.extend(descriptor_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_dynamic()(
            bootstrap_method_attr_index in 1..=u16::MAX,
            name_and_type_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(17);
            result.extend(bootstrap_method_attr_index.to_be_bytes());
            result.extend(name_and_type_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_invoke_dynamic()(
            bootstrap_method_attr_index in 1..=u16::MAX,
            name_and_type_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(5);
            result.push(18);
            result.extend(bootstrap_method_attr_index.to_be_bytes());
            result.extend(name_and_type_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_module()(
            name_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(3);
            result.push(19);
            result.extend(name_index.to_be_bytes());
            result
        }
    }

    prop_compose! {
        fn arb_constant_info_package()(
            name_index in 1..=u16::MAX
        ) -> Vec<u8> {
            let mut result = Vec::with_capacity(3);
            result.push(20);
            result.extend(name_index.to_be_bytes());
            result
        }
    }

    pub(crate) fn arb_constant_pool_info() -> impl Strategy<Value = Vec<u8>> {
        prop_oneof![
            arb_constant_info_utf8(),
            arb_constant_info_integer(),
            arb_constant_info_float(),
            arb_constant_info_long(),
            arb_constant_info_double(),
            arb_constant_info_class(),
            arb_constant_info_string(),
            arb_constant_info_field_ref(),
            arb_constant_info_method_ref(),
            arb_constant_info_interface_method_ref(),
            arb_constant_info_name_and_type(),
            arb_constant_info_method_handle(),
            arb_constant_info_method_type(),
            arb_constant_info_dynamic(),
            arb_constant_info_invoke_dynamic(),
            arb_constant_info_module(),
            arb_constant_info_package(),
        ]
    }
}
