use super::*;
use crate::jvm::{
    JavaString,
    references::{FieldRef, MethodRef},
};

impl<T> ValueCache<T> {
    fn extend_sources(&mut self, count: u16) {
        self.indices.resize_with(usize::from(count), || None);
    }
}

impl ConstantPool {
    pub(crate) fn insert_field_ref(&mut self, value: FieldRef) -> FieldRefId {
        let owner = self.insert_reference_type(value.owner);
        let name_and_type = self.insert_field_name_and_type(value.name, value.field_type);
        let index = self.inner.push(Entry::FieldRef(FieldRefData {
            owner,
            name_and_type,
        }));
        self.extend_test_sources();
        FieldRefId(NonZeroU16::new(index).expect("nonzero test index"))
    }
    pub(crate) fn insert_method_ref(&mut self, value: MethodRef) -> MethodRefId {
        let owner = self.insert_reference_type(value.owner);
        let name_and_type = self.insert_method_name_and_type(value.name, value.descriptor);
        let index = self.inner.push(Entry::MethodRef(MethodRefData {
            owner,
            name_and_type,
        }));
        self.extend_test_sources();
        MethodRefId(NonZeroU16::new(index).expect("nonzero test index"))
    }
    fn insert_field_name_and_type(
        &mut self,
        name: JavaString,
        ty: FieldType,
    ) -> FieldNameAndTypeId {
        let name = self.insert_utf8(name);
        let id = FieldTypeId(u16::try_from(self.field_types.values.len()).unwrap());
        self.field_types.values.push(ty);
        let index = self
            .inner
            .push(Entry::NameAndType(NameAndType::Field(name, id)));
        self.extend_test_sources();
        FieldNameAndTypeId(NonZeroU16::new(index).unwrap())
    }

    fn insert_method_name_and_type(
        &mut self,
        name: JavaString,
        descriptor: MethodDescriptor,
    ) -> MethodNameAndTypeId {
        let name = self.insert_utf8(name);
        let id = MethodDescriptorId(u16::try_from(self.method_descriptors.values.len()).unwrap());
        self.method_descriptors.values.push(descriptor);
        let index = self
            .inner
            .push(Entry::NameAndType(NameAndType::Method(name, id)));
        self.extend_test_sources();
        MethodNameAndTypeId(NonZeroU16::new(index).unwrap())
    }

    pub(crate) fn insert_reference_type(&mut self, value: ReferenceType) -> ReferenceTypeId {
        let index = self.inner.push(Entry::Class {
            reference_type: value,
            is_binary_name: false,
        });
        self.extend_test_sources();
        ReferenceTypeId(NonZeroU16::new(index).expect("nonzero test index"))
    }
    fn extend_test_sources(&mut self) {
        let count = self.inner.count();
        self.method_descriptors.extend_sources(count);
        self.field_types.extend_sources(count);
        self.return_types.extend_sources(count);
    }

    pub(crate) fn insert_class_name(&mut self, value: ClassName) -> ClassId {
        let index = self.inner.push(Entry::Class {
            reference_type: ReferenceType::Class(value),
            is_binary_name: true,
        });
        self.extend_test_sources();
        ClassId(NonZeroU16::new(index).expect("nonzero test index"))
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "preserve the owned fixture API"
    )]
    pub(crate) fn insert_utf8(&mut self, value: JavaString) -> Utf8Id {
        let range = self.append_utf8(&value);
        let index = self.inner.push(Entry::Utf8(range));
        self.extend_test_sources();
        Utf8Id(NonZeroU16::new(index).expect("nonzero test index"))
    }

    pub(crate) fn insert_method_descriptor(
        &mut self,
        value: MethodDescriptor,
    ) -> MethodDescriptorId {
        let id = u16::try_from(self.method_descriptors.values.len())
            .expect("test descriptors fit in u16");
        self.method_descriptors.values.push(value);
        MethodDescriptorId(id)
    }

    pub fn new() -> Self {
        Self {
            inner: PoolStorage::with_padding(1),
            utf8_bytes: Vec::new(),
            method_descriptors: ValueCache::new(1),
            field_types: ValueCache::new(1),
            return_types: ValueCache::new(1),
        }
    }

    pub fn from_bytes(bytes: &[u8], constant_pool_count: u16) -> Result<Self, ParseError> {
        Self::from_raw(RawConstantPool::from_bytes(bytes, constant_pool_count)?)
    }

    pub const fn count(&self) -> u16 {
        self.inner.count()
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}
