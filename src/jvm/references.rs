//! References to JVM elements.
use std::fmt::Display;

use crate::types::{
    field_type::FieldType,
    method_descriptor::{MethodDescriptor, ReturnType},
};

use super::method::Method;

/// A reference to a [`Class`](crate::jvm::class::Class).
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClassRef {
    /// The binary name of the class.
    pub binary_name: String,
}

impl ClassRef {
    /// Creates a new [`ClassRef`] from a binary name.
    pub fn new<S: Into<String>>(binary_name: S) -> Self {
        ClassRef {
            binary_name: binary_name.into(),
        }
    }
}

impl Display for ClassRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name)
    }
}

/// A reference to a [`Field`](crate::jvm::field::Field).
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct FieldRef {
    /// A reference to the class that contains the field.
    pub owner: ClassRef,
    /// The name of the field.
    pub name: String,

    /// The type of the field.
    pub field_type: FieldType,
}

impl Display for FieldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.owner, self.name)
    }
}

/// A reference to a [`Method`].
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MethodRef {
    /// The reference to the class containing the method.
    pub owner: ClassRef,
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

impl Display for MethodRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.owner, self.name)
    }
}

/// A reference to a [`Module`](crate::jvm::module::Module).
#[derive(Debug, Clone)]
pub struct ModuleRef {
    /// The name of the module.
    pub name: String,
}

/// A reference to a package.
#[derive(Debug, Clone)]
pub struct PackageRef {
    /// The binary name of the package.
    pub binary_name: String,
}

#[cfg(test)]
mod tests {
    use crate::tests::arb_class_name;

    use super::*;
    use proptest::prelude::*;

    proptest! {

        #[test]
        fn test_is_constructor(class_name in arb_class_name()) {
            let method = MethodRef {
                owner: ClassRef::new(class_name),
                name: Method::CONSTRUCTOR_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_constructor());
        }

        #[test]
        fn test_is_static_initializer_bolck(class_name in arb_class_name()) {
            let method = MethodRef {
                owner: ClassRef::new(class_name),
                name: Method::CLASS_INITIALIZER_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_static_initializer_block());
        }
    }
}
