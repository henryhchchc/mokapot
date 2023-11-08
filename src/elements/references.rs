use crate::types::FieldType;

use super::MethodDescriptor;

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

/// A reference to an interface method.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InterfaceMethodReference {
    /// The reference to the interface.
    pub interface: ClassReference,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
}

/// A reference to a class method.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ClassMethodReference {
    /// The reference to the class.
    pub class: ClassReference,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method.
    pub descriptor: MethodDescriptor,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum MethodReference {
    Class(ClassMethodReference),
    Interface(InterfaceMethodReference),
}

impl MethodReference {
    pub fn descriptor(&self) -> &MethodDescriptor {
        match self {
            Self::Class(ClassMethodReference { descriptor, .. }) => descriptor,
            Self::Interface(InterfaceMethodReference { descriptor, .. }) => descriptor,
        }
    }
}
