use std::{io::Read, iter::repeat_with};

use itertools::Itertools;

use crate::{
    jvm::{
        class::{
            BootstrapMethod, Class, ClassAccessFlags, ClassRef, ClassVersion, InnerClassInfo,
            RecordComponent, SourceDebugExtension,
        },
        constant_pool::ConstantPool,
        parsing::{jvm_element_parser::parse_flags, reader_utils::ValueReaderExt},
    },
    macros::{extract_attributes, malform},
};

use super::{jvm_element_parser::JvmElement, parsing_context::ParsingContext, Error};

impl Class {
    const JAVA_CLASS_MAIGC: u32 = 0xCAFE_BABE;

    pub(crate) fn parse<R: Read + ?Sized>(reader: &mut R) -> Result<Self, Error> {
        let magic: u32 = reader.read_value()?;
        if magic != Self::JAVA_CLASS_MAIGC {
            return Err(Error::NotAClassFile);
        }
        let minor_version = reader.read_value()?;
        let major_version = reader.read_value()?;
        let version = ClassVersion::from_versions(major_version, minor_version)?;
        let constant_pool_count: u16 = reader.read_value()?;
        let constant_pool = ConstantPool::from_reader(reader, constant_pool_count)?;

        let access_flags: ClassAccessFlags = parse_flags(reader)?;
        let this_class_idx = reader.read_value()?;
        let ClassRef { binary_name } = constant_pool.get_class_ref(this_class_idx)?;
        let super_class_idx = reader.read_value()?;
        let super_class = match super_class_idx {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(ClassAccessFlags::MODULE) => None,
            0 => malform!("Class must have a super type except for java/lang/Object or a module"),
            it => Some(constant_pool.get_class_ref(it)?),
        };

        let parsing_context = ParsingContext {
            constant_pool,
            class_version: version,
            current_class_binary_name: binary_name.clone(),
        };

        let ctx = &parsing_context;

        let interfaces = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        let fields = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        let methods = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        let attributes: Vec<Attribute> = JvmElement::parse_vec::<u16, _>(reader, ctx)?;

        let has_unread_data = {
            let mut may_remain = [0u8];
            let remain = Read::read(reader, &mut may_remain)?;
            remain == 1
        };
        if has_unread_data {
            return Err(Error::UnexpectedData);
        }

        extract_attributes! {
            for attributes in "class_file" {
                let source_file: SourceFile,
                let inner_classes: InnerClasses as unwrap_or_default,
                let enclosing_method: EnclosingMethod,
                let source_debug_extension: SourceDebugExtension,
                let bootstrap_methods: BootstrapMethods as unwrap_or_default,
                let runtime_visible_annotations: RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations: RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations: RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations: RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                let module: Module,
                let module_packages: ModulePackages as unwrap_or_default,
                let module_main_class: ModuleMainClass,
                let nest_host: NestHost,
                let nest_members: NestMembers as unwrap_or_default,
                let permitted_subclasses: PermittedSubclasses as unwrap_or_default,
                let signature: Signature,
                let record: Record,
                if let is_synthetic: Synthetic,
                if let is_deprecated: Deprecated,
                else let free_attributes
            }
        };

        Ok(Class {
            version,
            access_flags,
            binary_name,
            super_class,
            interfaces,
            fields,
            methods,
            source_file,
            inner_classes,
            enclosing_method,
            source_debug_extension,
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            bootstrap_methods,
            module,
            module_packages,
            module_main_class,
            nest_host,
            nest_members,
            permitted_subclasses,
            is_synthetic,
            is_deprecated,
            signature,
            record,
            free_attributes,
        })
    }
}

impl JvmElement for BootstrapMethod {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let bootstrap_method_ref = reader.read_value()?;
        let method_ref = ctx.constant_pool.get_method_handle(bootstrap_method_ref)?;
        let num_bootstrap_arguments: u16 = reader.read_value()?;
        let arguments = repeat_with(|| {
            let arg_idx = reader.read_value()?;
            ctx.constant_pool.get_constant_value(arg_idx)
        })
        .take(num_bootstrap_arguments.into())
        .try_collect()?;
        Ok(Self {
            method: method_ref,
            arguments,
        })
    }
}

impl JvmElement for InnerClassInfo {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let inner_class = JvmElement::parse(reader, ctx)?;
        let outer_class_info_index = reader.read_value()?;
        let outer_class = if outer_class_info_index == 0 {
            None
        } else {
            let the_class = ctx.constant_pool.get_class_ref(outer_class_info_index)?;
            Some(the_class)
        };
        let inner_name_index = reader.read_value()?;
        let inner_name = if inner_name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(inner_name_index)?.to_owned())
        };
        let inner_class_access_flags = parse_flags(reader)?;
        Ok(Self {
            inner_class,
            outer_class,
            inner_name,
            inner_class_access_flags,
        })
    }
}

impl JvmElement for RecordComponent {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let name = JvmElement::parse(reader, ctx)?;
        let component_type = JvmElement::parse(reader, ctx)?;

        let attributes: Vec<Attribute> = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        extract_attributes! {
            for attributes in "record_component" {
                let signature: Signature,
                let runtime_visible_annotations : RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations : RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                else let free_attributes
            }
        }

        Ok(RecordComponent {
            name,
            component_type,
            signature,
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            free_attributes,
        })
    }
}

impl JvmElement for ClassRef {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let class_info_idx = reader.read_value()?;
        ctx.constant_pool.get_class_ref(class_info_idx)
    }
}

impl JvmElement for SourceDebugExtension {
    fn parse<R: Read + ?Sized>(reader: &mut R, _ctx: &ParsingContext) -> Result<Self, Error> {
        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;
        Ok(Self::new(content))
    }
}
