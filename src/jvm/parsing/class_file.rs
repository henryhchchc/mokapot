use std::iter::repeat_with;

use crate::{
    jvm::{
        class::{
            constant_pool::ConstantPool, BootstrapMethod, Class, ClassAccessFlags, ClassReference,
            ClassVersion, InnerClassInfo, RecordComponent, SourceDebugExtension,
        },
        parsing::{
            jvm_element_parser::{parse_flags, parse_jvm_element},
            reader_utils::ClassReader,
        },
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
};

use super::{jvm_element_parser::ParseJvmElement, parsing_context::ParsingContext};

impl Class {
    const JAVA_CLASS_MAIGC: u32 = 0xCAFE_BABE;

    pub(crate) fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let magic: u32 = reader.read_value()?;
        if magic != Self::JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::NotAClassFile);
        }
        let version = ClassVersion::parse(reader)?;
        let constant_pool = ConstantPool::parse(reader)?;
        let access_flags: ClassAccessFlags = parse_flags(reader)?;
        let this_class_idx = reader.read_value()?;
        let ClassReference { binary_name } = constant_pool.get_class_ref(this_class_idx)?;
        let super_class_idx = reader.read_value()?;
        let super_class = match super_class_idx {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(ClassAccessFlags::MODULE) => None,
            0 => Err(ClassFileParsingError::MalformedClassFile(
                "Class must have a super type except for java/lang/Object or a module",
            ))?,
            it => Some(constant_pool.get_class_ref(it)?),
        };

        let parsing_context = ParsingContext {
            constant_pool,
            class_version: version,
            current_class_binary_name: binary_name.clone(),
        };

        let ctx = &parsing_context;

        let interfaces = parse_jvm_element(reader, ctx)?;
        let fields = parse_jvm_element(reader, ctx)?;
        let methods = parse_jvm_element(reader, ctx)?;
        let attributes: Vec<Attribute> = parse_jvm_element(reader, ctx)?;

        let mut may_remain: [u8; 1] = [0];
        let remain = std::io::Read::read(reader, &mut may_remain)?;
        if remain == 1 {
            return Err(ClassFileParsingError::UnexpectedData);
        }

        extract_attributes! {
            for attributes in "class_file" by {
                let source_file <= SourceFile,
                let inner_classes unwrap_or_default <= InnerClasses,
                let enclosing_method <= EnclosingMethod,
                let source_debug_extension <= SourceDebugExtension,
                let bootstrap_methods unwrap_or_default <= BootstrapMethods,
                let runtime_visible_annotations unwrap_or_default <= RuntimeVisibleAnnotations,
                let runtime_invisible_annotations unwrap_or_default <= RuntimeInvisibleAnnotations,
                let runtime_visible_type_annotations unwrap_or_default <= RuntimeVisibleTypeAnnotations,
                let runtime_invisible_type_annotations unwrap_or_default <= RuntimeInvisibleTypeAnnotations,
                let module <= Module,
                let module_packages unwrap_or_default <= ModulePackages,
                let module_main_class <= ModuleMainClass,
                let nest_host <= NestHost,
                let nest_members unwrap_or_default <= NestMembers,
                let permitted_subclasses unwrap_or_default <= PermittedSubclasses,
                let signature <= Signature,
                let record <= Record,
                if Synthetic => let is_synthetic = true,
                if Deprecated => let is_deprecated = true,
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
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for BootstrapMethod {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let bootstrap_method_ref = reader.read_value()?;
        let method_ref = ctx.constant_pool.get_method_handle(bootstrap_method_ref)?;
        let num_bootstrap_arguments: u16 = reader.read_value()?;
        let arguments = repeat_with(|| {
            let arg_idx = reader.read_value()?;
            ctx.constant_pool.get_constant_value(arg_idx)
        })
        .take(num_bootstrap_arguments as usize)
        .collect::<Result<_, _>>()?;
        Ok(Self {
            method: method_ref,
            arguments,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for InnerClassInfo {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let inner_class = parse_jvm_element(reader, ctx)?;
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

impl<R: std::io::Read> ParseJvmElement<R> for RecordComponent {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let name = parse_jvm_element(reader, ctx)?;
        let component_type = parse_jvm_element(reader, ctx)?;

        let attributes: Vec<Attribute> = parse_jvm_element(reader, ctx)?;
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
    }
}

impl ClassVersion {
    fn parse<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let minor = reader.read_value()?;
        let major = reader.read_value()?;
        Ok(Self { major, minor })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ClassReference {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let class_info_idx = reader.read_value()?;
        ctx.constant_pool.get_class_ref(class_info_idx)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for SourceDebugExtension {
    fn parse(reader: &mut R, _ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let mut content = Vec::new();
        reader.read_to_end(&mut content)?;
        Ok(Self::new(content))
    }
}
