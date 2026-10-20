//! References to JVM elements.

use derive_more::Display;

use super::{
    JavaStr, JavaString, Method,
    constant_pool::{FieldNameAndTypeId, MethodNameAndTypeId, ReferenceTypeId},
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
pub struct FieldReference {
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
pub struct MethodReference {
    /// The reference to the class or array type containing the method.
    pub owner: ReferenceType,
    /// The name of the method.
    pub name: JavaString,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
}

impl MethodReference {
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display)]
#[display("{owner}.{name}")]
pub struct FieldRef<'pool> {
    pub(super) owner: &'pool ReferenceType,
    pub(super) name: &'pool JavaStr,
    pub(super) field_type: &'pool FieldType,
}

impl<'pool> FieldRef<'pool> {
    /// Returns the symbolic owner, which may be an array type.
    #[must_use]
    pub const fn owner(self) -> &'pool ReferenceType {
        self.owner
    }

    /// Returns the field name.
    #[must_use]
    pub const fn name(self) -> &'pool JavaStr {
        self.name
    }

    /// Returns the field type.
    #[must_use]
    pub const fn field_type(self) -> &'pool FieldType {
        self.field_type
    }

    /// Converts this borrowed field reference into an owned value.
    #[must_use]
    pub fn into_owned(self) -> FieldReference {
        FieldReference {
            owner: self.owner().clone(),
            name: self.name().to_owned(),
            field_type: self.field_type().clone(),
        }
    }
}

/// A borrowed symbolic method reference.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Display)]
#[display("{owner}::{name}")]
pub struct MethodRef<'pool> {
    pub(super) owner: &'pool ReferenceType,
    pub(super) name: &'pool JavaStr,
    pub(super) descriptor: &'pool MethodDescriptor,
}

impl<'pool> MethodRef<'pool> {
    /// Returns the symbolic owner, which may be an array type.
    #[must_use]
    pub const fn owner(self) -> &'pool ReferenceType {
        self.owner
    }

    /// Returns the method name.
    #[must_use]
    pub const fn name(self) -> &'pool JavaStr {
        self.name
    }

    /// Returns the method descriptor.
    #[must_use]
    pub const fn descriptor(self) -> &'pool MethodDescriptor {
        self.descriptor
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

    /// Converts this borrowed method reference into an owned value.
    #[must_use]
    pub fn into_owned(self) -> MethodReference {
        MethodReference {
            owner: self.owner().clone(),
            name: self.name().to_owned(),
            descriptor: self.descriptor().clone(),
        }
    }
}
