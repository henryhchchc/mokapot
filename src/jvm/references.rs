use std::fmt::Display;

use crate::types::FieldType;

use super::{MethodDescriptor, ReturnType, CLASS_INITIALIZER_NAME, CONSTRUCTOR_NAME};

/// A reference to a module in the binary format.
#[derive(Debug)]
pub struct ModuleReference {
    /// The name of the module.
    pub name: String,
}

/// A reference to a package in the binary format.
#[derive(Debug)]
pub struct PackageReference {
    /// The binary name of the package.
    pub binary_name: String,
}

/// A reference to a class in the binary format.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClassReference {
    /// The binary name of the class.
    pub binary_name: String,
}

impl ClassReference {
    /// Creates a new class reference.
    pub fn new<S: Into<String>>(binary_name: S) -> Self {
        ClassReference {
            binary_name: binary_name.into(),
        }
    }
}

impl Display for ClassReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name)
    }
}

/// A reference to a field.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct FieldReference {
    /// A reference to the class that contains the field.
    pub class: ClassReference,
    /// The name of the field.
    pub name: String,

    /// The type of the field.
    pub field_type: FieldType,
}

impl Display for FieldReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.class, self.name)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MethodReference {
    /// The reference to the class.
    pub owner: ClassReference,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
}

impl MethodReference {
    /// Checks if the method reference refers to a constructor.
    pub fn is_constructor(&self) -> bool {
        self.name == CONSTRUCTOR_NAME && matches!(self.descriptor.return_type, ReturnType::Void)
    }

    /// Checks if the method reference refers to a static initializer block.
    pub fn is_static_initializer_block(&self) -> bool {
        self.name == CLASS_INITIALIZER_NAME
            && self.descriptor.parameters_types.is_empty()
            && matches!(self.descriptor.return_type, ReturnType::Void)
    }
}

impl Display for MethodReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.owner, self.name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeReference(pub FieldType);

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_is_constructor() {
        let method = MethodReference {
            owner: ClassReference::new("test"),
            name: CONSTRUCTOR_NAME.to_string(),
            descriptor: MethodDescriptor::from_str("()V").unwrap(),
        };

        assert!(method.is_constructor());
    }

    #[test]
    fn test_is_static_initializer_bolck() {
        let method = MethodReference {
            owner: ClassReference::new("test"),
            name: CLASS_INITIALIZER_NAME.to_string(),
            descriptor: MethodDescriptor::from_str("()V").unwrap(),
        };

        assert!(method.is_static_initializer_block());
    }
}
