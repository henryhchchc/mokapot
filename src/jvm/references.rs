//! References to JVM elements.

use crate::types::{
    field_type::FieldType,
    method_descriptor::{MethodDescriptor, ReturnType},
};

use super::Method;

/// A reference to a [`Class`](crate::jvm::Class).
#[derive(Debug, PartialEq, Eq, Clone, Hash, PartialOrd, Ord, derive_more::Display)]
#[display(fmt = "{binary_name}")]
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

/// A reference to a [`Field`](crate::jvm::Field).
#[derive(Debug, PartialEq, Eq, Hash, Clone, derive_more::Display)]
#[display(fmt = "{owner}.{name}")]
pub struct FieldRef {
    /// A reference to the class that contains the field.
    pub owner: ClassRef,
    /// The name of the field.
    pub name: String,
    /// The type of the field.
    pub field_type: FieldType,
}

/// A reference to a [`Method`].
#[derive(Debug, PartialEq, Eq, Hash, Clone, derive_more::Display)]
#[display(fmt = "{owner}::{name}")]
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

/// A reference to a [`Module`](crate::jvm::Module).
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
pub(crate) mod tests {
    use crate::tests::{arb_field_type, arb_identifier};

    use super::*;
    use proptest::prelude::*;

    pub(crate) fn arb_class_ref() -> impl Strategy<Value = ClassRef> {
        arb_identifier().prop_map(ClassRef::new)
    }

    pub(crate) fn arb_field_ref() -> impl Strategy<Value = FieldRef> {
        (arb_class_ref(), any::<String>(), arb_field_type()).prop_map(
            |(owner, name, field_type)| FieldRef {
                owner,
                name,
                field_type,
            },
        )
    }

    proptest! {

        #[test]
        fn test_is_constructor(class_name in arb_identifier()) {
            let method = MethodRef {
                owner: ClassRef::new(class_name),
                name: Method::CONSTRUCTOR_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_constructor());
        }

        #[test]
        fn test_is_static_initializer_bolck(class_name in arb_identifier()) {
            let method = MethodRef {
                owner: ClassRef::new(class_name),
                name: Method::CLASS_INITIALIZER_NAME.to_string(),
                descriptor: "()V".parse().unwrap(),
            };

            assert!(method.is_static_initializer_block());
        }
    }
}
