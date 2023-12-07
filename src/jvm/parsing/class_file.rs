use std::str::FromStr;

use crate::{
    jvm::{
        class::{
            BootstrapMethod, Class, ClassAccessFlags, ClassReference, ClassVersion, ConstantPool,
            InnerClassInfo, NestedClassAccessFlags, RecordComponent,
        },
        field::Field,
        method::Method,
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
    types::field_type::FieldType,
};

use super::{
    attribute::Attribute,
    parsing_context::ParsingContext,
    reader_utils::{parse_multiple, read_bytes_vec, read_u16, read_u32},
};

impl Class {
    const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

    pub(crate) fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let magic = read_u32(reader)?;
        if magic != Self::JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::NotAClassFile);
        }
        let version = ClassVersion::parse(reader)?;
        let constant_pool = ConstantPool::parse(reader)?;
        let access = read_u16(reader)?;
        let Some(access_flags) = ClassAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "class"));
        };
        let this_class_idx = read_u16(reader)?;
        let ClassReference { binary_name } = constant_pool.get_class_ref(this_class_idx)?;
        let super_class_idx = read_u16(reader)?;
        let super_class = match super_class_idx {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(ClassAccessFlags::MODULE) => None,
            0 => Err(ClassFileParsingError::MalformedClassFile(
                "Class must have a super type except for java/lang/Object or a module",
            ))?,
            it => Some(constant_pool.get_class_ref(it)?),
        };

        let ctx = ParsingContext {
            constant_pool,
            class_version: version,
            current_class_binary_name: binary_name.clone(),
        };

        let interfaces = parse_multiple(reader, &ctx, |reader, ctx| {
            let interface_idx = read_u16(reader)?;
            ctx.constant_pool.get_class_ref(interface_idx)
        })?;
        let fields = parse_multiple(reader, &ctx, Field::parse)?;
        let methods = parse_multiple(reader, &ctx, Method::parse)?;

        let attributes = parse_multiple(reader, &ctx, Attribute::parse)?;

        let mut may_remain: [u8; 1] = [0];
        let remain = reader.read(&mut may_remain)?;
        if remain == 1 {
            return Err(ClassFileParsingError::UnexpectedData);
        }

        extract_attributes! {
            for attributes in "class_file" by {
                let source_file <= SourceFile,
                let inner_classes <= InnerClasses,
                let enclosing_method <= EnclosingMethod,
                let source_debug_extension <= SourceDebugExtension,
                let rt_visible_anno <= RuntimeVisibleAnnotations,
                let rt_invisible_anno <= RuntimeInvisibleAnnotations,
                let rt_visible_type_anno <= RuntimeVisibleTypeAnnotations,
                let rt_invisible_type_anno <= RuntimeInvisibleTypeAnnotations,
                let bootstrap_methods <= BootstrapMethods,
                let module <= Module,
                let module_packages <= ModulePackages,
                let module_main_class <= ModuleMainClass,
                let nest_host <= NestHost,
                let nest_members <= NestMembers,
                let permitted_subclasses <= PermittedSubclasses,
                let signature <= Signature,
                let record <= Record,
                if Synthetic => is_synthetic = true,
                if Deprecated => is_deprecated = true,
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
            inner_classes: inner_classes.unwrap_or_default(),
            enclosing_method,
            source_debug_extension: source_debug_extension.unwrap_or_default(),
            runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
            runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
            runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
            runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
            bootstrap_methods: bootstrap_methods.unwrap_or_default(),
            module,
            module_packages: module_packages.unwrap_or_default(),
            module_main_class,
            nest_host,
            nest_members: nest_members.unwrap_or_default(),
            permitted_subclasses: permitted_subclasses.unwrap_or_default(),
            is_synthetic,
            is_deprecated,
            signature,
            record,
        })
    }
}

impl BootstrapMethod {
    fn parse<R>(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bootstrap_method_ref = read_u16(reader)?;
        let method_ref = ctx.constant_pool.get_method_handle(bootstrap_method_ref)?;
        let num_bootstrap_arguments = read_u16(reader)?;
        let arguments = (0..num_bootstrap_arguments)
            .map(|_| {
                let arg_idx = read_u16(reader)?;
                ctx.constant_pool.get_constant_value(arg_idx)
            })
            .collect::<Result<_, _>>()?;
        Ok(BootstrapMethod {
            method: method_ref,
            arguments,
        })
    }
}

impl Attribute {
    pub fn parse_source_file<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let sourcefile_index = read_u16(reader)?;
        let file_name = ctx.constant_pool.get_str(sourcefile_index)?;
        Ok(Self::SourceFile(file_name.to_owned()))
    }
    pub fn parse_innner_classes<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let number_of_classes = read_u16(reader)?;
        let mut classes = Vec::with_capacity(number_of_classes as usize);
        for _ in 0..number_of_classes {
            let inner_class_info_index = read_u16(reader)?;
            let inner_class = ctx.constant_pool.get_class_ref(inner_class_info_index)?;
            let outer_class_info_index = read_u16(reader)?;
            let outer_class = if outer_class_info_index == 0 {
                None
            } else {
                let the_class = ctx.constant_pool.get_class_ref(outer_class_info_index)?;
                Some(the_class)
            };
            let inner_name_index = read_u16(reader)?;
            let inner_name = if inner_name_index == 0 {
                None
            } else {
                Some(ctx.constant_pool.get_str(inner_name_index)?.to_owned())
            };
            let access_flags = read_u16(reader)?;
            let Some(inner_class_access_flags) = NestedClassAccessFlags::from_bits(access_flags)
            else {
                return Err(ClassFileParsingError::UnknownFlags(
                    access_flags,
                    "inner class",
                ));
            };
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
        _ctx: &ParsingContext,
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
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let num_bootstrap_methods = read_u16(reader)?;
        let bootstrap_methods = (0..num_bootstrap_methods)
            .map(|_| BootstrapMethod::parse(reader, ctx))
            .collect::<Result<_, _>>()?;
        Ok(Self::BootstrapMethods(bootstrap_methods))
    }
    pub(super) fn parse_nest_host<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let nest_host_index = read_u16(reader)?;
        let host_class = ctx.constant_pool.get_class_ref(nest_host_index)?;
        Ok(Self::NestHost(host_class))
    }
    pub(super) fn parse_nest_members<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let number_of_classes = read_u16(reader)?;
        let classes = (0..number_of_classes)
            .map(|_| {
                let class_index = read_u16(reader)?;
                ctx.constant_pool.get_class_ref(class_index)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self::NestMembers(classes))
    }
    pub(super) fn parse_record<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let component_count = read_u16(reader)?;
        let components = (0..component_count)
            .map(|_| {
                let name_index = read_u16(reader)?;
                let name = ctx.constant_pool.get_str(name_index)?.to_owned();
                let descriptor_index = read_u16(reader)?;
                let descriptor = ctx.constant_pool.get_str(descriptor_index)?;
                let component_type = FieldType::from_str(descriptor)?;

                let attributes = parse_multiple(reader, ctx, Attribute::parse)?;
                extract_attributes! {
                    for attributes in "record_component" by {
                        let signature <= Signature,
                        let rt_visible_anno <= RuntimeVisibleAnnotations,
                        let rt_invisible_anno <= RuntimeInvisibleAnnotations,
                        let rt_visible_type_anno <= RuntimeVisibleTypeAnnotations,
                        let rt_invisible_type_anno <= RuntimeInvisibleTypeAnnotations,
                    }
                }

                Ok(RecordComponent {
                    name,
                    component_type,
                    signature,
                    runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
                    runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
                    runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
                    runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
                })
            })
            .collect::<ClassFileParsingResult<_>>()?;
        Ok(Self::Record(components))
    }

    pub(super) fn parse_permitted_subclasses<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let number_of_classes = read_u16(reader)?;
        let classes = (0..number_of_classes)
            .map(|_| {
                let class_index = read_u16(reader)?;
                ctx.constant_pool.get_class_ref(class_index)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self::PermittedSubclasses(classes))
    }
}

impl ClassVersion {
    fn parse<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let minor = read_u16(reader)?;
        let major = read_u16(reader)?;
        Ok(Self { major, minor })
    }
}
