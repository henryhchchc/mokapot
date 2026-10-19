//! References to JVM elements.

use derive_more::Display;
use std::{
    fmt,
    hash::{Hash, Hasher},
};

use super::{
    JavaStr, JavaString, Method,
    constant_pool::{ConstantPool, FieldNameAndTypeId, MethodNameAndTypeId, ReferenceTypeId},
};
use crate::{
    intrinsics::see_jvm_spec,
    types::{
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
        reference_type::ReferenceType,
    },
};

/// A reference to a [`Field`](crate::jvm::Field).
#[doc = see_jvm_spec!(4, 4, 2)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display("{owner}.{name}")]
pub struct FieldRef {
    /// A reference to the class or array type that contains the field.
    pub owner: ReferenceType,
    /// The name of the field.
    pub name: JavaString,
    /// The type of the field.
    pub field_type: FieldType,
}

/// A reference to a [`Method`].
#[doc = see_jvm_spec!(4, 4, 2)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display("{owner}::{name}")]
pub struct MethodRef {
    /// The reference to the class or array type containing the method.
    pub owner: ReferenceType,
    /// The name of the method.
    pub name: JavaString,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
}

impl MethodRef {
    /// Checks if the method reference refers to a constructor.
    #[must_use]
    pub fn is_constructor(&self) -> bool {
        self.name == Method::CONSTRUCTOR_NAME
            && matches!(self.descriptor.return_type, ReturnType::Void)
    }

    /// Checks if the method reference refers to a static initializer block.
    #[must_use]
    pub fn is_static_initializer_block(&self) -> bool {
        self.name == Method::CLASS_INITIALIZER_NAME
            && self.descriptor.parameters_types.is_empty()
            && matches!(self.descriptor.return_type, ReturnType::Void)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldRefData {
    pub owner: ReferenceTypeId,
    pub name_and_type: FieldNameAndTypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodRefData {
    pub owner: ReferenceTypeId,
    pub name_and_type: MethodNameAndTypeId,
}

/// A borrowed symbolic field reference.
#[derive(Clone, Copy)]
pub struct FieldRefView<'pool> {
    pool: &'pool ConstantPool,
    data: &'pool FieldRefData,
}

impl<'pool> FieldRefView<'pool> {
    pub(crate) const fn new(pool: &'pool ConstantPool, data: &'pool FieldRefData) -> Self {
        Self { pool, data }
    }

    /// Returns the symbolic owner, which may be an array type.
    #[must_use]
    pub fn owner(self) -> &'pool ReferenceType {
        self.pool.reference_type(self.data.owner)
    }

    /// Returns the field name.
    #[must_use]
    pub fn name(self) -> &'pool JavaStr {
        self.pool.field_name_and_type(self.data.name_and_type).0
    }

    /// Returns the field type.
    #[must_use]
    pub fn field_type(self) -> &'pool FieldType {
        self.pool.field_name_and_type(self.data.name_and_type).1
    }
}

impl From<FieldRefView<'_>> for FieldRef {
    fn from(value: FieldRefView<'_>) -> Self {
        Self {
            owner: value.owner().clone(),
            name: value.name().to_owned(),
            field_type: value.field_type().clone(),
        }
    }
}

impl PartialEq for FieldRefView<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.owner(), self.name(), self.field_type())
            == (other.owner(), other.name(), other.field_type())
    }
}
impl Eq for FieldRefView<'_> {}
impl Hash for FieldRefView<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.owner(), self.name(), self.field_type()).hash(state);
    }
}
impl fmt::Debug for FieldRefView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FieldRefView")
            .field("owner", self.owner())
            .field("name", &self.name())
            .field("field_type", self.field_type())
            .finish()
    }
}
impl fmt::Display for FieldRefView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.owner(), self.name())
    }
}

/// A borrowed symbolic method reference.
#[derive(Clone, Copy)]
pub struct MethodRefView<'pool> {
    pool: &'pool ConstantPool,
    data: &'pool MethodRefData,
}

impl<'pool> MethodRefView<'pool> {
    pub(crate) const fn new(pool: &'pool ConstantPool, data: &'pool MethodRefData) -> Self {
        Self { pool, data }
    }

    /// Returns the symbolic owner, which may be an array type.
    #[must_use]
    pub fn owner(self) -> &'pool ReferenceType {
        self.pool.reference_type(self.data.owner)
    }

    /// Returns the method name.
    #[must_use]
    pub fn name(self) -> &'pool JavaStr {
        self.pool.method_name_and_type(self.data.name_and_type).0
    }

    /// Returns the method descriptor.
    #[must_use]
    pub fn descriptor(self) -> &'pool MethodDescriptor {
        self.pool.method_name_and_type(self.data.name_and_type).1
    }

    /// Checks whether this references a constructor.
    #[must_use]
    pub fn is_constructor(self) -> bool {
        self.name() == Method::CONSTRUCTOR_NAME
            && matches!(self.descriptor().return_type, ReturnType::Void)
    }

    /// Checks whether this references a static initializer.
    #[must_use]
    pub fn is_static_initializer_block(self) -> bool {
        self.name() == Method::CLASS_INITIALIZER_NAME
            && self.descriptor().parameters_types.is_empty()
            && matches!(self.descriptor().return_type, ReturnType::Void)
    }
}

impl From<MethodRefView<'_>> for MethodRef {
    fn from(value: MethodRefView<'_>) -> Self {
        Self {
            owner: value.owner().clone(),
            name: value.name().to_owned(),
            descriptor: value.descriptor().clone(),
        }
    }
}

impl PartialEq for MethodRefView<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.owner(), self.name(), self.descriptor())
            == (other.owner(), other.name(), other.descriptor())
    }
}
impl Eq for MethodRefView<'_> {}
impl Hash for MethodRefView<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.owner(), self.name(), self.descriptor()).hash(state);
    }
}
impl fmt::Debug for MethodRefView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodRefView")
            .field("owner", self.owner())
            .field("name", &self.name())
            .field("descriptor", self.descriptor())
            .finish()
    }
}
impl fmt::Display for MethodRefView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.owner(), self.name())
    }
}

/// A reference to a [`Module`](crate::jvm::Module).
#[doc = see_jvm_spec!(4, 4, 11)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display("{name}")]
pub struct ModuleRef {
    /// The name of the module.
    pub name: JavaString,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn method_ref(name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: ReferenceType::Class("org/mokapot/Test".parse().unwrap()),
            name: name.into(),
            descriptor: descriptor.parse().unwrap(),
        }
    }

    #[test]
    fn special_method_reference_checks_include_descriptor() {
        for (name, descriptor, constructor, initializer) in [
            (Method::CONSTRUCTOR_NAME, "()V", true, false),
            (Method::CONSTRUCTOR_NAME, "()I", false, false),
            (Method::CONSTRUCTOR_NAME, "(I)V", true, false),
            (Method::CLASS_INITIALIZER_NAME, "()V", false, true),
            (Method::CLASS_INITIALIZER_NAME, "()I", false, false),
            (Method::CLASS_INITIALIZER_NAME, "(I)V", false, false),
            ("ordinary", "()V", false, false),
        ] {
            let method = method_ref(name, descriptor);
            assert_eq!(method.is_constructor(), constructor);
            assert_eq!(method.is_static_initializer_block(), initializer);
            let mut pool = ConstantPool::new();
            let id = pool.insert_method_ref(method);
            let view = pool.method_ref(id);
            assert_eq!(view.is_constructor(), constructor);
            assert_eq!(view.is_static_initializer_block(), initializer);
        }
    }
}
