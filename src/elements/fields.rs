use crate::utils::read_u16;

use super::{
    attributes::{AttributeList, Attribute, annotation::{Annotation, TypeAnnotation}},
    class_file::{ClassFileParsingError, ClassFileParsingResult},
    constant_pool::ConstantPool,
};

pub struct Field {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
    pub constant_value: Option<ConstantValue>,
    pub is_synthetic: bool,
    pub is_deperecated: bool,
    pub signature: Option<String>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

#[derive(Debug, PartialEq)]
pub enum ConstantValue {
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    String(String),
}

#[derive(Debug)]
pub struct FieldInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: AttributeList,
}
impl FieldInfo {
    pub(crate) fn to_field(self, constant_pool: &ConstantPool) -> ClassFileParsingResult<Field> {
        let access_flags = self.access_flags;
        let name = constant_pool.get_string(self.name_index)?;
        let descriptor = constant_pool.get_string(self.descriptor_index)?;

        let mut constant_value = None;
        let mut is_synthetic = false;
        let mut is_deperecated = false;
        let mut signature = None;
        let mut runtime_visible_annotations = None;
        let mut runtime_invisible_annotations = None;
        let mut runtime_visible_type_annotations = None;
        let mut runtime_invisible_type_annotations = None;
        for attr in self.attributes.into_iter() {
            match attr {
                Attribute::ConstantValue(v) => constant_value = Some(v),
                Attribute::Synthetic => is_synthetic = true,
                Attribute::Deprecated => is_deperecated = true,
                Attribute::Signature(s) => signature = Some(s),
                Attribute::RuntimeVisibleAnnotations(a) => runtime_visible_annotations = Some(a),
                Attribute::RuntimeInvisibleAnnotations(a) => runtime_invisible_annotations = Some(a),
                Attribute::RuntimeVisibleTypeAnnotations(a) => runtime_visible_type_annotations = Some(a),
                Attribute::RuntimeInvisibleTypeAnnotations(a) => runtime_invisible_type_annotations = Some(a),
                _ => Err(ClassFileParsingError::UnexpectedAttribute)?,
            }
        }

        Ok(Field {
            access_flags,
            name,
            descriptor,
            constant_value,
            is_synthetic,
            is_deperecated,
            signature,
            runtime_visible_annotations: runtime_visible_annotations.unwrap_or_default(),
            runtime_invisible_annotations: runtime_invisible_annotations.unwrap_or_default(),
            runtime_visible_type_annotations: runtime_visible_type_annotations.unwrap_or_default(),
            runtime_invisible_type_annotations: runtime_invisible_type_annotations.unwrap_or_default(),
        })
    }

    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        fields_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Self>>
    where
        R: std::io::Read,
    {
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            fields.push(Self::parse(reader, constant_pool)?);
        }
        Ok(fields)
    }

    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        let attributes = AttributeList::parse(reader, constant_pool)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}
