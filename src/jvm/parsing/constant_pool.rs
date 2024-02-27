use std::{io::Read, str::FromStr};

use super::{
    reader_utils::{read_byte_chunk, ValueReaderExt},
    Error,
};
use crate::{
    jvm::{
        class::{ClassRef, MethodHandle},
        constant_pool::{ConstantPool, Entry},
        field::{ConstantValue, FieldRef, JavaString},
        method::MethodRef,
        module::{ModuleRef, PackageRef},
    },
    macros::malform,
    types::field_type::{FieldType, TypeReference},
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

    pub(super) fn get_class_ref(&self, index: u16) -> Result<ClassRef, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Class { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(ClassRef::new(name))
        } else {
            mismatch("Class", entry)
        }
    }

    pub(super) fn get_constant_value(&self, value_index: u16) -> Result<ConstantValue, Error> {
        let entry = self.get_entry(value_index)?;
        match entry {
            &Entry::Integer(it) => Ok(ConstantValue::Integer(it)),
            &Entry::Long(it) => Ok(ConstantValue::Long(it)),
            &Entry::Float(it) => Ok(ConstantValue::Float(it)),
            &Entry::Double(it) => Ok(ConstantValue::Double(it)),
            &Entry::String { string_index } => {
                if let Entry::Utf8(java_str) = self.get_entry(string_index)? {
                    Ok(ConstantValue::String(java_str.clone()))
                } else {
                    mismatch("Utf8", entry)
                }
            }
            &Entry::MethodType { descriptor_index } => self
                .get_str(descriptor_index)
                .and_then(|it| it.parse().map_err(Into::into))
                .map(ConstantValue::MethodType),
            &Entry::Class { name_index } => self
                .get_str(name_index)
                .map(ClassRef::new)
                .map(ConstantValue::Class),
            Entry::MethodHandle { .. } => self
                .get_method_handle(value_index)
                .map(ConstantValue::Handle),
            &Entry::Dynamic {
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
            unexpected => mismatch(
                concat!(
                    "Integer | Long | Float | Double | String ",
                    "| MethodType | Class | MethodHandle | Dynamic"
                ),
                unexpected,
            ),
        }
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

    pub(super) fn get_name_and_type<T>(&self, index: u16) -> Result<(String, T), Error>
    where
        T: FromStr,
        <T as FromStr>::Err: Into<Error>,
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

    pub(super) fn get_type_ref(&self, index: u16) -> Result<TypeReference, Error> {
        let ClassRef { binary_name: name } = self.get_class_ref(index)?;
        let field_type = if name.starts_with('[') {
            FieldType::from_str(name.as_str())?
        } else {
            FieldType::Object(ClassRef::new(name))
        };
        Ok(TypeReference(field_type.clone()))
    }
}

impl Entry {
    pub(crate) fn parse<R: Read + ?Sized>(reader: &mut R) -> Result<Self, Error> {
        let tag = reader.read_value()?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => reader.read_value().map(Self::Integer).map_err(Into::into),
            4 => reader.read_value().map(Self::Float).map_err(Into::into),
            5 => reader.read_value().map(Self::Long).map_err(Into::into),
            6 => reader.read_value().map(Self::Double).map_err(Into::into),
            7 => Ok(Self::Class {
                name_index: reader.read_value()?,
            }),
            8 => Ok(Self::String {
                string_index: reader.read_value()?,
            }),
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
            19 => Ok(Self::Module {
                name_index: reader.read_value()?,
            }),
            20 => Ok(Self::Package {
                name_index: reader.read_value()?,
            }),
            it => Err(Error::UnexpectedConstantPoolTag(it)),
        }
    }

    fn parse_utf8<R: Read + ?Sized>(reader: &mut R) -> Result<Self, Error> {
        let length: u16 = reader.read_value()?;
        let cesu8_content = read_byte_chunk(reader, length.into())?;
        match cesu8::from_java_cesu8(cesu8_content.as_slice()) {
            Ok(result) => Ok(Self::Utf8(JavaString::Utf8(result.into_owned()))),
            Err(_) => Ok(Self::Utf8(JavaString::InvalidUtf8(cesu8_content))),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {

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
