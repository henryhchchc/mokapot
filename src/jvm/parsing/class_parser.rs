use super::constant_pool::ConstantPool;
use crate::{
    jvm::{
        class::{Class, ClassAccessFlags, ClassReference, ClassVersion},
        field::Field,
        method::Method,
        ClassFileParsingError, ClassFileParsingResult,
    },
    macros::extract_attributes,
};

use super::{
    parsing_context::ParsingContext,
    reader_utils::{parse_multiple, read_u16, read_u32},
};

impl Class {
    /// Parses a class file from the given reader.
    pub fn from_reader<R>(reader: R) -> ClassFileParsingResult<Class>
    where
        R: std::io::Read,
    {
        let mut reader = reader;
        let magic = read_u32(&mut reader)?;
        if magic != JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::NotAClassFile);
        }
        let version = ClassVersion::parse(&mut reader)?;
        let constant_pool = ConstantPool::parse(&mut reader)?;
        let access = read_u16(&mut reader)?;
        let Some(access_flags) = ClassAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "class"));
        };
        let this_class_idx = read_u16(&mut reader)?;
        let ClassReference { binary_name } = constant_pool.get_class_ref(this_class_idx)?;
        let super_class_idx = read_u16(&mut reader)?;
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

        let interfaces = parse_multiple(&mut reader, &ctx, |reader, ctx| {
            let interface_idx = read_u16(reader)?;
            ctx.constant_pool.get_class_ref(interface_idx)
        })?;
        let fields = parse_multiple(&mut reader, &ctx, Field::parse)?;
        let methods = parse_multiple(&mut reader, &ctx, Method::parse)?;

        let attributes = parse_multiple(&mut reader, &ctx, Attribute::parse)?;

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

impl ClassVersion {
    /// Returns `true` if this class file is compiled with `--enable-preview`.
    pub fn is_preview_enabled(&self) -> bool {
        self.minor == 65535
    }
    fn parse<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let minor = read_u16(reader)?;
        let major = read_u16(reader)?;
        Ok(Self { major, minor })
    }
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;
