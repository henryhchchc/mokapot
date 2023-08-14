

use crate::utils::read_u16;

use super::{attributes::{AttributeList}, class_file::{ClassFileParsingError}, constant_pool::{ConstantPool}};

pub struct Field {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
    pub constant_value: Option<ConstantValue>,
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
    attributes: AttributeList
}
impl FieldInfo {
    pub(crate) fn to_field(&self, constant_pool: &ConstantPool) -> Result<Field, ClassFileParsingError> {
        let access_flags = self.access_flags;
        let name = constant_pool.get_string(self.name_index)?;
        let descriptor = constant_pool.get_string(self.descriptor_index)?;
        let constant_value = None;
        Ok(Field {
            access_flags,
            name,
            descriptor,
            constant_value,
        })
    }

    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        fields_count: u16,
        constant_pool: &ConstantPool,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut fields = Vec::with_capacity(fields_count as usize);
        for _ in 0..fields_count {
            fields.push(Self::parse(reader, constant_pool)?);
        }
        Ok(fields)
    }

    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> Result<Self, ClassFileParsingError>
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
