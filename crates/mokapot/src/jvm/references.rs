//! References to JVM elements.

use derive_more::Display;

use super::Method;
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
    pub name: String,
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
#[derive(Debug, PartialEq, Eq, Hash, Clone, Display)]
#[display("{name}")]
pub struct ModuleRef {
    /// The name of the module.
    pub name: String,
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::types::class_name::ClassName;

    proptest! {

        #[test]
        fn test_is_constructor(class_name in any::<ClassName>()) {
            let method = MethodRef {
                owner: ReferenceType::Class(class_name),
                name: Method::CONSTRUCTOR_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_constructor());
        }

        #[test]
        fn test_is_static_initializer_bolck(class_name in any::<ClassName>()) {
            let method = MethodRef {
                owner: ReferenceType::Class(class_name),
                name: Method::CLASS_INITIALIZER_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_static_initializer_block());
        }
    }
}
