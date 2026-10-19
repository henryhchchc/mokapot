use super::*;
use crate::jvm::{JavaString, references::MethodRef};

impl ConstantPool {
    pub(crate) fn new() -> Self {
        Self::with_slots(1, 0)
    }

    pub(crate) fn insert_utf8(&mut self, value: JavaString) -> Utf8Id {
        let start = self.utf8_bytes.len();
        self.utf8_bytes.extend(value.into_bytes());
        let range = start..self.utf8_bytes.len();
        let index = self.inner.push(Entry::Utf8(range));
        Utf8Id(NonZeroU16::new(index).expect("nonzero test index"))
    }

    pub(crate) fn insert_class_name(&mut self, value: ClassName) -> ClassId {
        let index = self.inner.push(Entry::Class {
            reference_type: ReferenceType::Class(value),
            is_binary_name: true,
        });
        ClassId(NonZeroU16::new(index).expect("nonzero test index"))
    }

    pub(crate) fn insert_reference_type(&mut self, value: ReferenceType) -> ReferenceTypeId {
        let index = self.inner.push(Entry::Class {
            reference_type: value,
            is_binary_name: false,
        });
        ReferenceTypeId(NonZeroU16::new(index).expect("nonzero test index"))
    }

    pub(crate) fn insert_field_type(&mut self, value: FieldType) -> FieldTypeId {
        let index = u16::try_from(self.field_types.len()).expect("test types fit in u16");
        self.field_types.push(value);
        FieldTypeId(index)
    }

    pub(crate) fn insert_method_descriptor(
        &mut self,
        value: MethodDescriptor,
    ) -> MethodDescriptorId {
        let index =
            u16::try_from(self.method_descriptors.len()).expect("test descriptors fit in u16");
        self.method_descriptors.push(value);
        MethodDescriptorId(index)
    }

    pub(crate) fn insert_method_ref(&mut self, value: MethodRef) -> MethodRefId {
        let owner = self.insert_reference_type(value.owner);
        let name = self.insert_utf8(value.name);
        let descriptor = self.insert_method_descriptor(value.descriptor);
        let name_and_type = self.insert_method_name_and_type(name, descriptor);
        let index = self.inner.push(Entry::MethodRef(MethodRefData {
            owner,
            name_and_type,
        }));
        MethodRefId(NonZeroU16::new(index).expect("nonzero test index"))
    }

    fn insert_method_name_and_type(
        &mut self,
        name: Utf8Id,
        descriptor: MethodDescriptorId,
    ) -> MethodNameAndTypeId {
        let index = self
            .inner
            .push(Entry::NameAndType(NameAndType::Method(name, descriptor)));
        MethodNameAndTypeId(NonZeroU16::new(index).expect("nonzero test index"))
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}
