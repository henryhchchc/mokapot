use std::{io::Read, str::FromStr};

use super::{
    reader_utils::{read_byte_chunk, ValueReaderExt},
    Error,
};
use crate::{
    jvm::{
        class::{ClassReference, MethodHandle},
        constant_pool::{ConstantPool, Entry},
        field::{ConstantValue, FieldReference, JavaString},
        method::{MethodDescriptor, MethodReference},
        module::{ModuleReference, PackageReference},
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

    pub(super) fn get_class_ref(&self, index: u16) -> Result<ClassReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Class { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(ClassReference::new(name))
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
                .and_then(|it| MethodDescriptor::from_str(it).map_err(Into::into))
                .map(ConstantValue::MethodType),
            &Entry::Class { name_index } => self
                .get_str(name_index)
                .map(ClassReference::new)
                .map(ConstantValue::Class),
            Entry::MethodHandle { .. } => self
                .get_method_handle(value_index)
                .map(ConstantValue::Handle),
            &Entry::Dynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
                let descriptor = FieldType::from_str(descriptor)?;
                Ok(ConstantValue::Dynamic(
                    bootstrap_method_attr_index,
                    name.to_owned(),
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

    pub(super) fn get_module_ref(&self, index: u16) -> Result<ModuleReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Module { name_index } = entry {
            let name = self.get_str(name_index)?.to_owned();
            Ok(ModuleReference { name })
        } else {
            mismatch("Module", entry)
        }
    }

    pub(super) fn get_package_ref(&self, index: u16) -> Result<PackageReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Package { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(PackageReference {
                binary_name: name.to_owned(),
            })
        } else {
            mismatch("Package", entry)
        }
    }

    pub(super) fn get_field_ref(&self, index: u16) -> Result<FieldReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::FieldRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let class = self.get_class_ref(class_index)?;
            let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
            let field_type = FieldType::from_str(descriptor)?;
            Ok(FieldReference {
                class,
                name: name.to_owned(),
                field_type,
            })
        } else {
            mismatch("Field", entry)
        }
    }

    pub(super) fn get_name_and_type(&self, index: u16) -> Result<(&str, &str), Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::NameAndType {
            name_index,
            descriptor_index,
        } = entry
        {
            let name = self.get_str(name_index)?;
            let descriptor = self.get_str(descriptor_index)?;
            Ok((name, descriptor))
        } else {
            mismatch("NameAndType", entry)
        }
    }

    pub(super) fn get_method_ref(&self, index: u16) -> Result<MethodReference, Error> {
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
            let (name, descriptor_str) = self.get_name_and_type(name_and_type_index)?;
            let name = name.to_owned();
            let descriptor = MethodDescriptor::from_str(descriptor_str)?;
            Ok(MethodReference {
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
        let ClassReference { binary_name: name } = self.get_class_ref(index)?;
        let field_type = if name.starts_with('[') {
            FieldType::from_str(name.as_str())?
        } else {
            FieldType::Object(ClassReference::new(name))
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
