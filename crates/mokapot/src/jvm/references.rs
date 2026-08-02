//! References to JVM elements.

use std::str::FromStr;

use super::Method;
use crate::{
    intrinsics::see_jvm_spec,
    types::{
        binary_name::{BinaryName, InvalidBinaryName},
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
        reference_type::ReferenceType,
    },
};

/// A reference to a [`Class`](crate::jvm::Class).
#[doc = see_jvm_spec!(4, 4, 1)]
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord, derive_more::Display)]
#[display("{_0}")]
pub struct ClassRef(pub BinaryName);

impl ClassRef {
    /// Creates a new [`ClassRef`] from a binary name.
    #[must_use]
    pub const fn new(binary_name: BinaryName) -> Self {
        ClassRef(binary_name)
    }
}

impl FromStr for ClassRef {
    type Err = InvalidBinaryName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BinaryName::from_str(s).map(ClassRef)
    }
}

/// A reference to a [`Field`](crate::jvm::Field).
#[doc = see_jvm_spec!(4, 4, 2)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display("{owner}.{name}")]
pub struct FieldRef {
    /// A reference to the class or array type that contains the field.
    pub owner: ReferenceType,
    /// The name of the field.
    pub name: String,
    /// The type of the field.
    pub field_type: FieldType,
}

/// A reference to a [`Method`].
#[doc = see_jvm_spec!(4, 4, 2)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display("{owner}::{name}")]
pub struct MethodRef {
    /// The reference to the class or array type containing the method.
    pub owner: ReferenceType,
    /// The name of the method.
    pub name: String,
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

/// A reference to a [`Module`](crate::jvm::Module).
#[doc = see_jvm_spec!(4, 4, 11)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display("{name}")]
pub struct ModuleRef {
    /// The name of the module.
    pub name: String,
}

/// A reference to a package.
#[doc = see_jvm_spec!(4, 4, 12)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, derive_more::Display)]
#[display("{_0}")]
pub struct PackageRef(pub BinaryName);

impl FromStr for PackageRef {
    type Err = InvalidBinaryName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BinaryName::from_str(s).map(PackageRef)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::tests::{arb_binary_name, arb_field_type};

    pub(crate) fn arb_class_ref() -> impl Strategy<Value = ClassRef> {
        arb_binary_name().prop_map(ClassRef)
    }

    pub(crate) fn arb_field_ref() -> impl Strategy<Value = FieldRef> {
        (arb_class_ref(), any::<String>(), arb_field_type()).prop_map(
            |(owner, name, field_type)| FieldRef {
                owner: ReferenceType::Class(owner),
                name,
                field_type,
            },
        )
    }

    proptest! {

        #[test]
        fn test_is_constructor(class_name in arb_binary_name()) {
            let method = MethodRef {
                owner: ReferenceType::Class(ClassRef(class_name)),
                name: Method::CONSTRUCTOR_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_constructor());
        }

        #[test]
        fn test_is_static_initializer_bolck(class_name in arb_binary_name()) {
            let method = MethodRef {
                owner: ReferenceType::Class(ClassRef(class_name)),
                name: Method::CLASS_INITIALIZER_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_static_initializer_block());
        }
    }
}
