//! Module for the APIs for the annotation in JVM.
use std::fmt;

use super::{
    Class, ConstantVal, DebugList, JavaStr,
    code::{LocalVariableId, ProgramCounter},
    constant_pool::{ConstantValueId, FieldTypeId, ReturnTypeId, Utf8Id},
};
use crate::{
    intrinsics::see_jvm_spec,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
    },
};

/// A value of an annotation field.
#[doc = see_jvm_spec!(4, 7, 16, 1)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementValue<'class> {
    /// A constant value in primitive type.
    Primitive(PrimitiveType, ConstantVal<'class>),
    /// A constant value in String type.
    String(&'class JavaStr),
    /// An enum constant.
    EnumConstant {
        /// The type of the enum.
        enum_type: &'class FieldType,
        /// The name of the enum constant.
        const_name: &'class JavaStr,
    },
    /// A class literal.
    Class {
        /// The descriptor of the class literal.
        return_descriptor: &'class ReturnType,
    },
    /// Another annotation.
    AnnotationInterface(Annotation<'class>),
    /// An array of values.
    Array(ElementValues<'class>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ElementValueData {
    Primitive(PrimitiveType, ConstantValueId),
    String(Utf8Id),
    EnumConstant {
        enum_type: FieldTypeId,
        const_name: Utf8Id,
    },
    Class {
        return_descriptor: ReturnTypeId,
    },
    AnnotationInterface(AnnotationData),
    Array(Box<[Self]>),
}

impl<'class> ElementValue<'class> {
    pub(crate) fn new(class: &'class Class, data: &'class ElementValueData) -> Self {
        let pool = &class.constant_pool;
        match data {
            ElementValueData::Primitive(kind, id) => {
                Self::Primitive(*kind, pool.constant_value(*id))
            }
            ElementValueData::String(id) => Self::String(pool.utf8(*id)),
            ElementValueData::EnumConstant {
                enum_type,
                const_name,
            } => Self::EnumConstant {
                enum_type: pool.field_type(*enum_type),
                const_name: pool.utf8(*const_name),
            },
            ElementValueData::Class { return_descriptor } => Self::Class {
                return_descriptor: pool.return_type(*return_descriptor),
            },
            ElementValueData::AnnotationInterface(data) => {
                Self::AnnotationInterface(Annotation::new(class, data))
            }
            ElementValueData::Array(data) => Self::Array(ElementValues { class, data }),
        }
    }
}

/// The values in an annotation array.
#[derive(Clone, Copy)]
pub struct ElementValues<'class> {
    class: &'class Class,
    data: &'class [ElementValueData],
}

impl<'class> ElementValues<'class> {
    /// Returns the array values.
    pub fn iter(self) -> impl ExactSizeIterator<Item = ElementValue<'class>> + Clone {
        self.data
            .iter()
            .map(move |data| ElementValue::new(self.class, data))
    }

    /// Returns the value at an index.
    #[must_use]
    pub fn get(self, index: usize) -> Option<ElementValue<'class>> {
        self.data
            .get(index)
            .map(|data| ElementValue::new(self.class, data))
    }

    /// Returns the number of values.
    #[must_use]
    pub const fn len(self) -> usize {
        self.data.len()
    }

    /// Returns whether the array is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.data.is_empty()
    }
}

impl PartialEq for ElementValues<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for ElementValues<'_> {}

impl fmt::Debug for ElementValues<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnnotationData {
    pub annotation_type: FieldTypeId,
    pub element_value_pairs: Box<[(Utf8Id, ElementValueData)]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeAnnotationData {
    pub annotation_type: FieldTypeId,
    pub target_info: TargetInfo,
    pub target_path: Box<[TypePathElement]>,
    pub element_value_pairs: Box<[(Utf8Id, ElementValueData)]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnnotationsData<T> {
    pub runtime_visible: Box<[T]>,
    pub runtime_invisible: Box<[T]>,
}

impl<T> Default for AnnotationsData<T> {
    fn default() -> Self {
        Self {
            runtime_visible: Box::default(),
            runtime_invisible: Box::default(),
        }
    }
}

/// An annotation on a class, field, method, or parameter.
#[doc = see_jvm_spec!(4, 7, 16)]
#[derive(Clone, Copy)]
pub struct Annotation<'class> {
    class: &'class Class,
    data: &'class AnnotationData,
}

impl<'class> Annotation<'class> {
    /// The default annotation element name.
    pub const DEFAULT_ELEMENT_NAME: &'static str = "value";

    pub(crate) const fn new(class: &'class Class, data: &'class AnnotationData) -> Self {
        Self { class, data }
    }

    /// Returns the annotation type.
    #[must_use]
    pub fn annotation_type(self) -> &'class FieldType {
        self.class
            .constant_pool
            .field_type(self.data.annotation_type)
    }

    /// Returns the element names and values.
    pub fn element_value_pairs(
        self,
    ) -> impl ExactSizeIterator<Item = (&'class JavaStr, ElementValue<'class>)> + Clone {
        self.data
            .element_value_pairs
            .iter()
            .map(move |(name, data)| {
                (
                    self.class.constant_pool.utf8(*name),
                    ElementValue::new(self.class, data),
                )
            })
    }

    /// Returns an element by its Java name.
    #[must_use]
    pub fn get_element_value(self, name: &JavaStr) -> Option<ElementValue<'class>> {
        self.element_value_pairs()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value)
    }

    /// Returns an element by its Rust string name.
    #[must_use]
    pub fn get_element_value_str(self, name: &str) -> Option<ElementValue<'class>> {
        self.element_value_pairs()
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value)
    }

    /// Returns the default element value.
    #[must_use]
    pub fn get_value(self) -> Option<ElementValue<'class>> {
        self.get_element_value_str(Self::DEFAULT_ELEMENT_NAME)
    }
}

impl PartialEq for Annotation<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.annotation_type() == other.annotation_type()
            && self.element_value_pairs().eq(other.element_value_pairs())
    }
}

impl Eq for Annotation<'_> {}

impl fmt::Debug for Annotation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Annotation")
            .field("annotation_type", &self.annotation_type())
            .field(
                "element_value_pairs",
                &DebugList::new(|| self.element_value_pairs()),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_equality_compares_values_across_pools() {
        let [(first_class, first_data), (second_class, second_data)] =
            [false, true].map(|offset| {
                let mut class = Class::default();
                let pool = &mut class.constant_pool;
                if offset {
                    pool.insert_field_type("I".parse().unwrap());
                    pool.insert_utf8("unused".to_owned().into());
                }
                let annotation_type = pool.insert_field_type("LExample;".parse().unwrap());
                let name = pool.insert_utf8("value".to_owned().into());
                let value = pool.insert_utf8("answer".to_owned().into());
                let data = AnnotationData {
                    annotation_type,
                    element_value_pairs: Box::new([(name, ElementValueData::String(value))]),
                };
                (class, data)
            });

        assert_ne!(first_data, second_data);
        assert_eq!(
            Annotation::new(&first_class, &first_data),
            Annotation::new(&second_class, &second_data)
        );
    }
}

/// A type annotation.
#[doc = see_jvm_spec!(4, 7, 20)]
#[derive(Clone, Copy)]
pub struct TypeAnnotation<'class> {
    class: &'class Class,
    data: &'class TypeAnnotationData,
}

impl<'class> TypeAnnotation<'class> {
    pub(crate) const fn new(class: &'class Class, data: &'class TypeAnnotationData) -> Self {
        Self { class, data }
    }

    /// Returns the annotation type.
    #[must_use]
    pub fn annotation_type(self) -> &'class FieldType {
        self.class
            .constant_pool
            .field_type(self.data.annotation_type)
    }

    /// Returns the annotated declaration or expression.
    #[must_use]
    pub const fn target_info(self) -> &'class TargetInfo {
        &self.data.target_info
    }

    /// Returns the path to the annotated part of a type.
    #[must_use]
    pub fn target_path(self) -> &'class [TypePathElement] {
        &self.data.target_path
    }

    /// Returns the element names and values.
    pub fn element_value_pairs(
        self,
    ) -> impl ExactSizeIterator<Item = (&'class JavaStr, ElementValue<'class>)> + Clone {
        self.data
            .element_value_pairs
            .iter()
            .map(move |(name, data)| {
                (
                    self.class.constant_pool.utf8(*name),
                    ElementValue::new(self.class, data),
                )
            })
    }
}

impl PartialEq for TypeAnnotation<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.annotation_type() == other.annotation_type()
            && self.target_info() == other.target_info()
            && self.target_path() == other.target_path()
            && self.element_value_pairs().eq(other.element_value_pairs())
    }
}

impl Eq for TypeAnnotation<'_> {}

impl fmt::Debug for TypeAnnotation<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeAnnotation")
            .field("annotation_type", &self.annotation_type())
            .field("target_info", self.target_info())
            .field("target_path", &self.target_path())
            .field(
                "element_value_pairs",
                &DebugList::new(|| self.element_value_pairs()),
            )
            .finish()
    }
}

/// Runtime-visible and runtime-invisible annotations.
#[derive(Clone, Copy)]
pub struct Annotations<'class> {
    class: &'class Class,
    data: &'class AnnotationsData<AnnotationData>,
}

impl<'class> Annotations<'class> {
    pub(crate) const fn new(
        class: &'class Class,
        data: &'class AnnotationsData<AnnotationData>,
    ) -> Self {
        Self { class, data }
    }

    /// Returns the runtime-visible annotations.
    pub fn runtime_visible(self) -> impl ExactSizeIterator<Item = Annotation<'class>> + Clone {
        self.data
            .runtime_visible
            .iter()
            .map(move |data| Annotation::new(self.class, data))
    }

    /// Returns the runtime-invisible annotations.
    pub fn runtime_invisible(self) -> impl ExactSizeIterator<Item = Annotation<'class>> + Clone {
        self.data
            .runtime_invisible
            .iter()
            .map(move |data| Annotation::new(self.class, data))
    }
}

impl PartialEq for Annotations<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_visible().eq(other.runtime_visible())
            && self.runtime_invisible().eq(other.runtime_invisible())
    }
}

impl Eq for Annotations<'_> {}

impl fmt::Debug for Annotations<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Annotations")
            .field(
                "runtime_visible",
                &DebugList::new(|| self.runtime_visible()),
            )
            .field(
                "runtime_invisible",
                &DebugList::new(|| self.runtime_invisible()),
            )
            .finish()
    }
}

/// Runtime-visible and runtime-invisible type annotations.
#[derive(Clone, Copy)]
pub struct TypeAnnotations<'class> {
    class: &'class Class,
    data: &'class AnnotationsData<TypeAnnotationData>,
}

impl<'class> TypeAnnotations<'class> {
    pub(crate) const fn new(
        class: &'class Class,
        data: &'class AnnotationsData<TypeAnnotationData>,
    ) -> Self {
        Self { class, data }
    }

    /// Returns the runtime-visible type annotations.
    pub fn runtime_visible(self) -> impl ExactSizeIterator<Item = TypeAnnotation<'class>> + Clone {
        self.data
            .runtime_visible
            .iter()
            .map(move |data| TypeAnnotation::new(self.class, data))
    }

    /// Returns the runtime-invisible type annotations.
    pub fn runtime_invisible(
        self,
    ) -> impl ExactSizeIterator<Item = TypeAnnotation<'class>> + Clone {
        self.data
            .runtime_invisible
            .iter()
            .map(move |data| TypeAnnotation::new(self.class, data))
    }
}

impl PartialEq for TypeAnnotations<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_visible().eq(other.runtime_visible())
            && self.runtime_invisible().eq(other.runtime_invisible())
    }
}

impl Eq for TypeAnnotations<'_> {}

impl fmt::Debug for TypeAnnotations<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeAnnotations")
            .field(
                "runtime_visible",
                &DebugList::new(|| self.runtime_visible()),
            )
            .field(
                "runtime_invisible",
                &DebugList::new(|| self.runtime_invisible()),
            )
            .finish()
    }
}

/// Annotations grouped by method parameter.
#[derive(Clone, Copy)]
pub struct ParameterAnnotations<'class> {
    class: &'class Class,
    data: &'class AnnotationsData<Box<[AnnotationData]>>,
}

impl<'class> ParameterAnnotations<'class> {
    pub(crate) const fn new(
        class: &'class Class,
        data: &'class AnnotationsData<Box<[AnnotationData]>>,
    ) -> Self {
        Self { class, data }
    }

    /// Returns runtime-visible annotations for each parameter.
    pub fn runtime_visible(
        self,
    ) -> impl ExactSizeIterator<Item = impl ExactSizeIterator<Item = Annotation<'class>> + Clone> + Clone
    {
        self.data.runtime_visible.iter().map(move |annotations| {
            annotations
                .iter()
                .map(move |data| Annotation::new(self.class, data))
        })
    }

    /// Returns runtime-invisible annotations for each parameter.
    pub fn runtime_invisible(
        self,
    ) -> impl ExactSizeIterator<Item = impl ExactSizeIterator<Item = Annotation<'class>> + Clone> + Clone
    {
        self.data.runtime_invisible.iter().map(move |annotations| {
            annotations
                .iter()
                .map(move |data| Annotation::new(self.class, data))
        })
    }
}

impl PartialEq for ParameterAnnotations<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.runtime_visible().len() == other.runtime_visible().len()
            && self.runtime_invisible().len() == other.runtime_invisible().len()
            && self
                .runtime_visible()
                .zip(other.runtime_visible())
                .all(|(a, b)| a.eq(b))
            && self
                .runtime_invisible()
                .zip(other.runtime_invisible())
                .all(|(a, b)| a.eq(b))
    }
}

impl Eq for ParameterAnnotations<'_> {}

impl fmt::Debug for ParameterAnnotations<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParameterAnnotations")
            .field(
                "runtime_visible",
                &DebugList::new(|| {
                    self.runtime_visible()
                        .map(|annotations| DebugList::new(move || annotations.clone()))
                }),
            )
            .field(
                "runtime_invisible",
                &DebugList::new(|| {
                    self.runtime_invisible()
                        .map(|annotations| DebugList::new(move || annotations.clone()))
                }),
            )
            .finish()
    }
}

/// Represents the location of a type parameter in a class or method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParameterLocation {
    /// The type parameter is declared in a class.
    Class,
    /// The type parameter is declared in a method.
    Method,
}

/// Represents the kind of a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableKind {
    /// A local variable.
    Local,
    /// A resource variable declared in a `try-with-resources` statement.
    Resource,
}

/// Represents the offset of a type annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffsetOf {
    /// Offset for an `instanceof` expression.
    InstanceOf,
    /// Offset for a `new` expression.
    New,
    /// Offset for a method reference expression.
    MethodReference,
    /// Offset for a constructor reference expression.
    ConstructorReference,
}

/// Represents the location of a type argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeArgumentLocation {
    /// Type argument in a cast expression.
    Cast,
    /// Type argument in a constructor invocation.
    Constructor,
    /// Type argument in a method call.
    MethodCall,
    /// Type argument in a constructor reference expression.
    ConstructorReference,
    /// Type argument in a method reference expression.
    MethodReference,
}

/// Information about the target of a [`TypeAnnotation`].
#[doc = see_jvm_spec!(4, 7, 20, 1)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetInfo {
    /// Indicates an annotation appears on a type parameter declaration of a generic class, interface, method, or constructor.
    TypeParameter {
        /// The location of the type parameter declaration.
        location: TypeParameterLocation,
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
        /// The location of the type parameter which the bound applies to.
        location: TypeParameterLocation,
        /// The index of the type parameter declaration.
        type_parameter_index: u8,
        /// The index of the bound of the type parameter declaration.
        bound_index: u8,
    },
    /// Indicates that an annotation appears on a field declaration or a record component declaration.
    Field,
    /// The return type of a method, or the type of a newly constructed object.
    MethodReturn,
    /// Indicates that an annotation appears on the receiver type of a method.
    Receiver,
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
    LocalVar(VariableKind, Box<[LocalVariableId]>),

    /// Indicates that an annotation appears on a type in an exception parameter declaration.
    Catch {
        /// The index in the exception index.
        index: u16,
    },
    /// Indicates that an annotation appears on either the type in an `instanceof` expression
    /// or a `new` expression, or the type before the `::` in a method reference expression.
    Offset(OffsetOf, u16),
    /// Indicates that an annotation appears on a type in a cast expression,
    /// or on a type argument in the explicit type argument list for any of the following:
    /// - A new expression
    /// - An explicit constructor invocation statement
    /// - A method invocation expression
    /// - A method reference expression.
    TypeArgument {
        /// The location of the type argument.
        location: TypeArgumentLocation,
        /// The location of the instruction
        offset: ProgramCounter,
        /// The index of the type argument.
        index: u8,
    },
}

/// Identifies a part of a type that is annotated.
#[doc = see_jvm_spec!(4, 7, 20, 2)]
#[derive(Debug, Clone, PartialEq, Eq)]
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
