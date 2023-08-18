use super::field::ConstantValue;

#[derive(Debug)]
pub struct Annotation {
    pub annotation_type_desc: String,
    pub element_value_pairs: Vec<(String, ElementValue)>,
}

#[derive(Debug)]
pub enum TargetInfo {
    TypeParameter(u8),
    SuperType(u16),
    TypeParameterBound(u8, u8),
    Empty,
    FormalParameter(u8),
    Throws(u16),
    LocalVar(Vec<(u16, u16, u16)>),
    Catch(u16),
    Offset(u16),
    TypeArgument(u16, u8),
}

#[derive(Debug)]
pub enum TypePathKind {
    Array = 0x00,
    Nested = 0x01,
    Bound = 0x02,
    TypeArgument = 0x03,
}

#[derive(Debug)]
pub struct TypePathElement {
    pub kind: TypePathKind,
    pub argument_index: u8,
}

#[derive(Debug)]
pub struct TypeAnnotation {
    pub target_info: TargetInfo,
    pub target_path: Vec<TypePathElement>,
    pub type_index: u16,
    pub element_value_pairs: Vec<(String, ElementValue)>,
}

#[derive(Debug)]
pub enum ElementValue {
    Constant(ConstantValue),
    EnumConstant {
        type_name: String,
        const_name: String,
    },
    Class {
        return_descriptor: String,
    },
    AnnotationInterface(Annotation),
    Array(Vec<ElementValue>),
}
