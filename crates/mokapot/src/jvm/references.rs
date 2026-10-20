//! References to JVM elements.

use derive_more::Display;
use std::fmt;

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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

impl fmt::Display for FieldRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.owner(), self.name())
    }
}

/// A borrowed symbolic method reference.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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

impl fmt::Display for MethodRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.owner(), self.name())
    }
}

/// A reference to a [`Module`](crate::jvm::Module).
#[doc = see_jvm_spec!(4, 4, 11)]
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display("{name}")]
pub struct ModuleReference {
    /// The name of the module.
    pub name: JavaString,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::jvm::constant_pool::ConstantPool;

    fn method_ref(name: &str, descriptor: &str) -> MethodReference {
        MethodReference {
            owner: ReferenceType::Class("org/mokapot/Test".parse().unwrap()),
            name: name.to_owned().into(),
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
