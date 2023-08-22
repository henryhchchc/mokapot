use crate::{
    elements::{
        class::{BootstrapMethod, InnerClassInfo, RecordComponent},
        class_parser::{ClassFileParsingError, ClassFileParsingResult},
    },
    utils::{read_bytes_vec, read_u16, read_u32},
};

use super::{
    attribute::{Attribute, AttributeList},
    constant_pool::ConstantPool,
};

impl BootstrapMethod {
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bootstrap_method_ref = read_u16(reader)?;
        let method_ref = constant_pool.get_method_handle(&bootstrap_method_ref)?;
        let num_bootstrap_arguments = read_u16(reader)?;
        let mut arguments = Vec::with_capacity(num_bootstrap_arguments as usize);
        for _ in 0..num_bootstrap_arguments {
            let arg_idx = read_u16(reader)?;
            let arg = constant_pool.get_constant_value(&arg_idx)?;
            arguments.push(arg);
        }
        Ok(BootstrapMethod {
            method: method_ref,
            arguments,
        })
    }
}

impl Attribute {
    pub fn parse_source_file<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let sourcefile_index = read_u16(reader)?;
        let file_name = constant_pool.get_string(&sourcefile_index)?;
        Ok(Self::SourceFile(file_name))
    }
    pub fn parse_innner_classes<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_classes = read_u16(reader)?;
        let mut classes = Vec::with_capacity(number_of_classes as usize);
        for _ in 0..number_of_classes {
            let inner_class_info_index = read_u16(reader)?;
            let inner_class = constant_pool.get_class_ref(&inner_class_info_index)?;
            let outer_class_info_index = read_u16(reader)?;
            let outer_class = if outer_class_info_index == 0 {
                None
            } else {
                let the_class = constant_pool.get_class_ref(&outer_class_info_index)?;
                Some(the_class)
            };
            let inner_name_index = read_u16(reader)?;
            let inner_name = if inner_name_index == 0 {
                None
            } else {
                Some(constant_pool.get_string(&inner_name_index)?)
            };
            let inner_class_access_flags = read_u16(reader)?;
            classes.push(InnerClassInfo {
                inner_class,
                outer_class,
                inner_name,
                inner_class_access_flags,
            });
        }
        Ok(Self::InnerClasses(classes))
    }

    pub(super) fn parse_source_debug_extension<R>(
        reader: &mut R,
        _constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let attribute_length = read_u32(reader)?;
        let debug_extension = read_bytes_vec(reader, attribute_length as usize)?;
        Ok(Self::SourceDebugExtension(debug_extension))
    }

    pub(super) fn parse_bootstrap_methods<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let num_bootstrap_methods = read_u16(reader)?;
        let mut bootstrap_methods = Vec::with_capacity(num_bootstrap_methods as usize);
        for _ in 0..num_bootstrap_methods {
            let entry = BootstrapMethod::parse(reader, constant_pool)?;
            bootstrap_methods.push(entry);
        }
        Ok(Self::BootstrapMethods(bootstrap_methods))
    }
    pub(super) fn parse_nest_host<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let nest_host_index = read_u16(reader)?;
        let host_class = constant_pool.get_class_ref(&nest_host_index)?;
        Ok(Self::NestHost(host_class))
    }
    pub(super) fn parse_nest_members<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_classes = read_u16(reader)?;
        let mut classes = Vec::with_capacity(number_of_classes as usize);
        for _ in 0..number_of_classes {
            let class_index = read_u16(reader)?;
            let class = constant_pool.get_class_ref(&class_index)?;
            classes.push(class);
        }
        Ok(Self::NestMembers(classes))
    }
    pub(super) fn parse_record<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let component_count = read_u16(reader)?;
        let mut components = Vec::with_capacity(component_count as usize);
        for _ in 0..component_count {
            let name_index = read_u16(reader)?;
            let name = constant_pool.get_string(&name_index)?;
            let descriptor_index = read_u16(reader)?;
            let descriptor = constant_pool.get_string(&descriptor_index)?;

            let attributes = AttributeList::parse(reader, constant_pool)?;
            let mut signature = None;
            let mut rt_visible_anno = None;
            let mut rt_invisible_anno = None;
            let mut rt_visible_type_anno = None;
            let mut rt_invisible_type_anno = None;
            for attr in attributes.into_iter() {
                match attr {
                    Attribute::Signature(sig) => signature = Some(sig),
                    Attribute::RuntimeVisibleAnnotations(it) => rt_visible_anno = Some(it),
                    Attribute::RuntimeInvisibleAnnotations(it) => rt_invisible_anno = Some(it),
                    Attribute::RuntimeVisibleTypeAnnotations(it) => rt_visible_type_anno = Some(it),
                    Attribute::RuntimeInvisibleTypeAnnotations(it) => {
                        rt_invisible_type_anno = Some(it)
                    }
                    it => Err(ClassFileParsingError::UnexpectedAttribute(
                        format!("{:?}", it),
                        "record".to_string(),
                    ))?,
                }
            }
            components.push(RecordComponent {
                name,
                descriptor,
                signature,
                runtime_visible_annotations: rt_visible_anno.unwrap_or(vec![]),
                runtime_invisible_annotations: rt_invisible_anno.unwrap_or(vec![]),
                runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or(vec![]),
                runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or(vec![]),
            });
        }
        Ok(Self::Record(components))
    }
    pub(super) fn parse_permitted_subclasses<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let number_of_classes = read_u16(reader)?;
        let mut classes = Vec::with_capacity(number_of_classes as usize);
        for _ in 0..number_of_classes {
            let class_index = read_u16(reader)?;
            let class = constant_pool.get_class_ref(&class_index)?;
            classes.push(class);
        }
        Ok(Self::PermittedSubclasses(classes))
    }
}
