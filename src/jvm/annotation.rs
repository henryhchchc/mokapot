//! Module for the APIs for the annotation in JVM.
use crate::{
    macros::see_jvm_spec,
    types::{field_type::PrimitiveType, method_descriptor::ReturnType},
};

use super::{
    code::{LocalVariableId, ProgramCounter},
    field::ConstantValue,
    Annotation,
};

/// A value of an annotation field.
#[doc = see_jvm_spec!(4, 7, 16, 1)]
#[derive(Debug, Clone)]
pub enum ElementValue {
    /// A constant value in primitive type.
    Primitive(PrimitiveType, ConstantValue),
    /// A constant value in String type.
    String(ConstantValue),
    /// An enum constant.
    EnumConstant {
        /// The name of the enum type.
        enum_type_name: String,
        /// The name of the enum constant.
        const_name: String,
    },
    /// A class literal.
    Class {
        /// The descriptor of the class literal.
        return_descriptor: ReturnType,
    },
    /// Another annotation.
    AnnotationInterface(Annotation),
    /// An array of values.
    Array(Vec<ElementValue>),
}

/// Information about the target of a [`TypeAnnotation`](super::TypeAnnotation).
#[doc = see_jvm_spec!(4, 7, 20, 1)]
#[derive(Debug, Clone)]
pub enum TargetInfo {
    /// Idicates an annotation appears on a type parameter declaration of a generic class, interface, method, or constructor.
    TypeParameter {
        /// The index of the type parameter declaration.
        index: u8,
    },
    /// Indicates that an annotation appears on a type in the `extends` or `implements` clause of a class or interface declaration.
    SuperType {
        /// The index of the type in the `implements` clause.
        /// A value of [`u16::MAX`] specifies that the annotation appears on the superclass in an extends clause of a class declaration.
        index: u16,
    },
    /// Indicates that an annotation appears on a bound of a type parameter declaration of a generic class, interface, method, or constructor.
    TypeParameterBound {
        /// The index of the type parameter declaration.
        type_parameter_index: u8,
        /// The index of the bound of the type parameter declaration.
        bound_index: u8,
    },
    /// Indicates that an annotation appears on either the type in a field declaration, the type in a record component declaration,
    /// the return type of a method, the type of a newly constructed object, or the receiver type of a method or constructor.
    Empty,
    /// Indicates that an annotation appears on the type in a formal parameter declaration of a method, constructor, or lambda expression.
    FormalParameter {
        /// The index of the formal parameter declaration.
        index: u8,
    },
    /// Indicates that anannotation appears on a type in the throws clause of a method or constructor declaration.
    Throws {
        /// The index of the type in the throws clause.
        index: u16,
    },
    /// Indicates that an annotation appears on the type in a local variable declaration, including a variable declared as a resource in a `try-with-resources` statement.
    LocalVar(Vec<LocalVariableId>),

    /// Indicates that an annotation appears on a type in an exception parameter declaration.
    Catch {
        /// The index in the exception index.
        index: u16,
    },
    /// Indicates that an annotation appears on either the type in an `instanceof`` expression or a `new` expression,
    /// or the type before the `::` in a method reference expression.
    Offset(u16),
    /// Indicates that an annotation appears on a type in a cast expression,
    /// or on a type argument in the explicit type argument list for any of the following:
    /// - A new expression
    /// - An explicit constructor invocation statement
    /// - A method invocation expression
    /// - A method reference expression.
    TypeArgument {
        /// The location of the instruction
        offset: ProgramCounter,
        /// The index of the type argument.
        index: u8,
    },
}

/// Identifies a part of a type that is annotated.
#[doc = see_jvm_spec!(4, 7, 20, 2)]
#[derive(Debug, Clone)]
pub enum TypePathElement {
    /// Annotation is deeper in an array type.
    Array,
    /// Annotation is deeper in a nested type.
    Nested,
    /// Annotation is on the bound of a wildcard type argument of a parameterized type.
    Bound,
    /// Annotation is on a type argument of a parameterized type.
    TypeArgument(u8),
}
