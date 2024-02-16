use std::str::FromStr;

use super::{
    reader_utils::{read_byte_chunk, ClassReader},
    Error,
};
use crate::{
    jvm::{
        class::{ClassReference, MethodHandle},
        constant_pool::{ConstantPool, Entry, Slot},
        field::{ConstantValue, FieldReference, JavaString},
        method::{MethodDescriptor, MethodReference},
        module::{ModuleReference, PackageReference},
    },
    types::field_type::{FieldType, TypeReference},
};

impl ConstantPool {
    pub(crate) fn get_str(&self, index: u16) -> Result<&str, Error> {
        let entry = self.get_entry(index)?;
        match entry {
            Entry::Utf8(JavaString::ValidUtf8(string)) => Ok(string),
            Entry::Utf8(JavaString::InvalidUtf8(_)) => Err(Error::BrokenUTF8),
            _ => Err(Error::MismatchedConstantPoolEntryType {
                expected: "Utf8",
                found: entry.constant_kind(),
            }),
        }
    }

    pub(crate) fn get_class_ref(&self, index: u16) -> Result<ClassReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Class { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(ClassReference::new(name))
        } else {
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "Class",
                found: entry.constant_kind(),
            })
        }
    }

    pub(crate) fn get_constant_value(&self, value_index: u16) -> Result<ConstantValue, Error> {
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
                Err(Error::MismatchedConstantPoolEntryType {
                    expected: "Utf8",
                    found: entry.constant_kind(),
                })
            }
        }
        &Entry::MethodType { descriptor_index } => {
            let descriptor_str = self.get_str(descriptor_index)?;
            let descriptor = MethodDescriptor::from_str(descriptor_str)?;
            Ok(ConstantValue::MethodType(descriptor))
        }
        Entry::Class { .. } => {
            let class = self.get_class_ref(value_index)?;
            Ok(ConstantValue::Class(class))
        }
        Entry::MethodHandle { .. } => {
            let method_handle = self.get_method_handle(value_index)?;
            Ok(ConstantValue::Handle(method_handle))
        }
        &Entry::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            let (name, descriptor_str) = self.get_name_and_type(name_and_type_index)?;
            let descriptor = FieldType::from_str(descriptor_str)?;
            Ok(ConstantValue::Dynamic(
                bootstrap_method_attr_index,
                name.to_owned(),
                descriptor,
            ))
        }
        unexpected => Err(Error::MismatchedConstantPoolEntryType{
            expected: "Integer | Long | Float | Double | String | MethodType | Class | MethodHandle | Dynamic",
            found: unexpected.constant_kind(),
        })
    }
    }

    pub(crate) fn get_module_ref(&self, index: u16) -> Result<ModuleReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Module { name_index } = entry {
            let name = self.get_str(name_index)?.to_owned();
            Ok(ModuleReference { name })
        } else {
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "Module",
                found: entry.constant_kind(),
            })
        }
    }

    pub(crate) fn get_package_ref(&self, index: u16) -> Result<PackageReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::Package { name_index } = entry {
            let name = self.get_str(name_index)?;
            Ok(PackageReference {
                binary_name: name.to_owned(),
            })
        } else {
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "Package",
                found: entry.constant_kind(),
            })
        }
    }

    pub(crate) fn get_field_ref(&self, index: u16) -> Result<FieldReference, Error> {
        let entry = self.get_entry(index)?;
        if let &Entry::FieldRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let class = self.get_class_ref(class_index)?;
            let (name, descriptor) = self.get_name_and_type(name_and_type_index)?;
            let field_type = FieldType::from_str(descriptor)?;
            return Ok(FieldReference {
                class,
                name: name.to_owned(),
                field_type,
            });
        }
        Err(Error::MismatchedConstantPoolEntryType {
            expected: "Field",
            found: entry.constant_kind(),
        })
    }

    pub(crate) fn get_name_and_type(&self, index: u16) -> Result<(&str, &str), Error> {
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
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "NameAndType",
                found: entry.constant_kind(),
            })
        }
    }

    pub(crate) fn get_method_ref(&self, index: u16) -> Result<MethodReference, Error> {
        let entry = self.get_entry(index)?;
        match entry {
            &Entry::MethodRef {
                class_index,
                name_and_type_index,
            }
            | &Entry::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => {
                let owner = self.get_class_ref(class_index)?;
                let (name, descriptor_str) = self.get_name_and_type(name_and_type_index)?;
                let name = name.to_owned();
                let descriptor = MethodDescriptor::from_str(descriptor_str)?;
                Ok(MethodReference {
                    owner,
                    name,
                    descriptor,
                })
            }
            _ => Err(Error::MismatchedConstantPoolEntryType {
                expected: "MethodRef | InterfaceMethodRef",
                found: entry.constant_kind(),
            }),
        }
    }

    pub(crate) fn get_method_handle(&self, index: u16) -> Result<MethodHandle, Error> {
        use MethodHandle::{
            RefGetField, RefGetStatic, RefInvokeInterface, RefInvokeSpecial, RefInvokeStatic,
            RefInvokeVirtual, RefNewInvokeSpecial, RefPutField, RefPutStatic,
        };

        let entry = self.get_entry(index)?;
        let &Entry::MethodHandle {
            reference_kind,
            reference_index: idx,
        } = entry
        else {
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "MethodHandle",
                found: entry.constant_kind(),
            })?
        };

        let result = match reference_kind {
            1 => RefGetField(self.get_field_ref(idx)?),
            2 => RefGetStatic(self.get_field_ref(idx)?),
            3 => RefPutField(self.get_field_ref(idx)?),
            4 => RefPutStatic(self.get_field_ref(idx)?),
            5 => RefInvokeVirtual(self.get_method_ref(idx)?),
            6 => RefInvokeStatic(self.get_method_ref(idx)?),
            7 => RefInvokeSpecial(self.get_method_ref(idx)?),
            8 => RefNewInvokeSpecial(self.get_method_ref(idx)?),
            9 => RefInvokeInterface(self.get_method_ref(idx)?),
            _ => Err(Error::MalformedClassFile(
                "Invalid reference kind in method handle",
            ))?,
        };
        Ok(result)
    }

    pub(crate) fn get_type_ref(&self, index: u16) -> Result<TypeReference, Error> {
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
    pub(crate) fn parse_multiple<R>(reader: &mut R, count: u16) -> Result<Vec<Slot>, Error>
    where
        R: std::io::Read,
    {
        // The `constant_pool` table is indexed from `1` to `constant_pool_count - 1`.
        let count: usize = count.into();
        let mut result = Vec::with_capacity(count);
        result.push(Slot::Padding);
        while result.len() < count {
            let entry = Self::parse(reader)?;
            if let entry @ (Entry::Long(_) | Entry::Double(_)) = entry {
                result.push(Slot::Entry(entry));
                result.push(Slot::Padding);
            } else {
                result.push(Slot::Entry(entry));
            }
        }
        Ok(result)
    }

    fn parse<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let tag = reader.read_value()?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => Self::parse_integer(reader),
            4 => Self::parse_float(reader),
            5 => Self::parse_long(reader),
            6 => Self::parse_double(reader),
            7 => Self::parse_class(reader),
            8 => Self::parse_string(reader),
            9 => Self::parse_field_ref(reader),
            10 => Self::parse_method_ref(reader),
            11 => Self::parse_interface_method_ref(reader),
            12 => Self::parse_name_and_type(reader),
            15 => Self::parse_method_handle(reader),
            16 => Self::parse_method_type(reader),
            17 => Self::parse_dynamic(reader),
            18 => Self::parse_invoke_dynamic(reader),
            19 => Self::parse_module(reader),
            20 => Self::parse_package(reader),
            it => Err(Error::UnexpectedConstantPoolTag(it)),
        }
    }

    fn parse_utf8<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let length: u16 = reader.read_value()?;
        let cesu8_content = read_byte_chunk(reader, length as usize)?;
        match cesu8::from_java_cesu8(cesu8_content.as_slice()) {
            Ok(result) => Ok(Self::Utf8(JavaString::ValidUtf8(result.into_owned()))),
            Err(_) => Ok(Self::Utf8(JavaString::InvalidUtf8(cesu8_content))),
        }
    }

    fn parse_integer<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        Ok(Self::Integer(reader.read_value()?))
    }

    fn parse_float<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        Ok(Self::Float(reader.read_value()?))
    }

    fn parse_long<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        Ok(Self::Long(reader.read_value()?))
    }

    fn parse_double<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        Ok(Self::Double(reader.read_value()?))
    }

    fn parse_class<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Class { name_index })
    }

    fn parse_string<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let string_index = reader.read_value()?;
        Ok(Self::String { string_index })
    }

    fn parse_field_ref<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::FieldRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_method_ref<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::MethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_interface_method_ref<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_name_and_type<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        let descriptor_index = reader.read_value()?;
        Ok(Self::NameAndType {
            name_index,
            descriptor_index,
        })
    }

    fn parse_method_handle<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let reference_kind = reader.read_value()?;
        let reference_index = reader.read_value()?;
        Ok(Self::MethodHandle {
            reference_kind,
            reference_index,
        })
    }

    fn parse_method_type<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let descriptor_index = reader.read_value()?;
        Ok(Self::MethodType { descriptor_index })
    }

    fn parse_dynamic<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_invoke_dynamic<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_module<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Module { name_index })
    }

    fn parse_package<R>(reader: &mut R) -> Result<Self, Error>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Package { name_index })
    }
}
