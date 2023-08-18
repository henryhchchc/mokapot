use crate::{
    elements::{
        class_file::{ClassFileParsingError, ClassFileParsingResult, ClassReference},
        constant_pool::{ConstantPool, ConstantPoolEntry},
    },
    utils::{read_bytes_vec, read_u16, read_u32},
};

use super::{
    annotation::{Annotation, TypeAnnotation},
    Attribute, AttributeList,
};

#[derive(Debug)]
pub struct InnerClassInfo {
    pub inner_class: ClassReference,
    pub outer_class: Option<ClassReference>,
    pub inner_name: String,
    pub inner_class_access_flags: u16,
}

#[derive(Debug)]
pub enum MethodHandle {
    GetField(u16),
    GetStatic(u16),
    PutField(u16),
    PutStatic(u16),
    InvokeVirtual(u16),
    InvokeStatic(u16),
    InvokeSpecial(u16),
    NewInvokeSpecial(u16),
    InvokeInterface(u16),
}

impl MethodHandle {
    pub(super) fn form_cp_idx(
        index: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self> {
        let ConstantPoolEntry::MethodHandle { reference_kind,  reference_index } = constant_pool.get_entry(index)? else {
            Err(ClassFileParsingError::MidmatchedConstantPoolTag)?
        };
        let result = match reference_kind {
            1 => Self::GetField(*reference_index),
            2 => Self::GetStatic(*reference_index),
            3 => Self::PutField(*reference_index),
            4 => Self::PutStatic(*reference_index),
            5 => Self::InvokeVirtual(*reference_index),
            6 => Self::InvokeStatic(*reference_index),
            7 => Self::InvokeSpecial(*reference_index),
            8 => Self::NewInvokeSpecial(*reference_index),
            9 => Self::InvokeInterface(*reference_index),
            _ => Err(ClassFileParsingError::MalformedClassFile)?,
        };
        Ok(result)
    }
}

#[derive(Debug)]
pub struct BootstrapArgument {}

#[derive(Debug)]
pub struct BootstrapMethod {
    pub method: MethodHandle,
    pub argument_indeices: Vec<u16>,
}
impl BootstrapMethod {
    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bootstrap_method_ref = read_u16(reader)?;
        let method_ref = MethodHandle::form_cp_idx(bootstrap_method_ref, constant_pool)?;
        let num_bootstrap_arguments = read_u16(reader)?;
        let mut argument_indeices = Vec::with_capacity(num_bootstrap_arguments as usize);
        for _ in 0..num_bootstrap_arguments {
            let arg_idx = read_u16(reader)?;
            let _entry = constant_pool.get_entry(arg_idx)?;
            argument_indeices.push(arg_idx);
        }
        Ok(BootstrapMethod {
            method: method_ref,
            argument_indeices,
        })
    }
}



#[derive(Debug)]
pub struct RecordComponent {
    pub name: String,
    pub descriptor: String,
    pub signature: Option<String>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

impl Attribute {
    pub(super) fn parse_source_file<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let sourcefile_index = read_u16(reader)?;
        let file_name = constant_pool.get_string(sourcefile_index)?;
        Ok(Self::SourceFile(file_name))
    }
    pub(super) fn parse_innner_classes<R>(
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
            let inner_class = constant_pool.get_class_ref(inner_class_info_index)?;
            let outer_class_info_index = read_u16(reader)?;
            let outer_class = if outer_class_info_index == 0 {
                None
            } else {
                let the_class = constant_pool.get_class_ref(outer_class_info_index)?;
                Some(the_class)
            };
            let inner_name_index = read_u16(reader)?;
            let inner_name = constant_pool.get_string(inner_name_index)?;
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
        let host_class = constant_pool.get_class_ref(nest_host_index)?;
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
            let class = constant_pool.get_class_ref(class_index)?;
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
            let name = constant_pool.get_string(name_index)?;
            let descriptor_index = read_u16(reader)?;
            let descriptor = constant_pool.get_string(descriptor_index)?;

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
                    Attribute::RuntimeInvisibleTypeAnnotations(it) => rt_invisible_type_anno = Some(it),
                    _ => Err(ClassFileParsingError::UnexpectedAttribute)?,
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
            let class = constant_pool.get_class_ref(class_index)?;
            classes.push(class);
        }
        Ok(Self::PermittedSubclasses(classes))
    }
}
