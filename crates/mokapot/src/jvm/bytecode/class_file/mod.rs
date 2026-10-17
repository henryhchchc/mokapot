//! Class-file parsing and model conversion.

mod decode;

use super::{
    ParseError, ParsingContext, attribute::AttributeInfo, class_element::ClassElement,
    constant_pool::RawConstantPool, field_info::FieldInfo, method_info::MethodInfo, raw_attributes,
};
use crate::{
    intrinsics::{extract_attributes, see_jvm_spec},
    jvm::{
        Annotations, Class,
        class::{
            self, BootstrapMethod, ConstantPool, EnclosingMethod, InnerClassInfo,
            NestedClassAccessFlags, RecordComponent, Version,
        },
        errors::ParsingErrorContext,
    },
};

/// The raw representation of a class file.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug)]
pub(crate) struct ClassFile<'a> {
    minor_version: u16,
    major_version: u16,
    constant_pool: RawConstantPool<'a>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: &'a [zerocopy::byteorder::big_endian::U16],
    fields: Box<[FieldInfo<'a>]>,
    methods: Box<[MethodInfo<'a>]>,
    attributes: Box<[AttributeInfo<'a>]>,
}

const JAVA_CLASS_MAGIC: u32 = 0xCAFE_BABE;

impl Class {
    /// Parses exactly one class file from the given bytes.
    /// # Errors
    /// Returns a [`ParseError`] for malformed, truncated, or trailing data.
    /// # Example
    /// ```no_run
    /// use mokapot::jvm::Class;
    ///
    /// let bytes = std::fs::read("path/to/class/file").unwrap();
    /// let class = Class::from_bytes(&bytes).unwrap();
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseError> {
        Self::from_raw(ClassFile::parse(bytes)?)
    }
}

impl Class {
    pub(crate) fn from_raw(raw: ClassFile<'_>) -> Result<Self, ParseError> {
        let ClassFile {
            minor_version,
            major_version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        } = raw;
        let version = Version::from_versions(major_version, minor_version)?;
        let access_flags = class::AccessFlags::from_bits(access_flags)
            .ok_or(ParseError::malform("Invalid class access flags"))?;
        let constant_pool = ConstantPool::from_raw(constant_pool)?;
        let class_name = constant_pool.get_class_name(this_class)?;
        let super_class = match super_class {
            0 if class_name == "java/lang/Object" => None,
            0 if access_flags.contains(class::AccessFlags::MODULE) => None,
            0 => Err(ParseError::malform(
                "Class must have a super type except for java/lang/Object or a module",
            ))?,
            it => Some(constant_pool.get_class_name(it)?),
        };

        let ctx = &ParsingContext {
            constant_pool,
            class_version: version,
            current_class_name: class_name.clone(),
        };

        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;

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
                else let other_attributes
            }
        };

        Ok(Class {
            version,
            access_flags,
            name: class_name,
            super_class,
            interfaces: interfaces
                .iter()
                .map(|it| ctx.constant_pool.get_class_name(it.get()))
                .collect::<Result<_, _>>()?,
            fields: fields
                .into_iter()
                .map(|it| ClassElement::from_raw(it, ctx))
                .collect::<Result<_, _>>()?,
            methods: methods
                .into_iter()
                .map(|it| ClassElement::from_raw(it, ctx))
                .collect::<Result<_, _>>()?,
            source_file,
            inner_classes,
            enclosing_method,
            source_debug_extension,
            annotations: Annotations {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: Annotations {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
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
            other_attributes,
        })
    }
}

impl<'a> ClassElement<'a> for BootstrapMethod {
    type Raw = raw_attributes::BootstrapMethod<'a>;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            bootstrap_method_ref,
            bootstrap_arguments,
        } = raw;
        let method = ctx.constant_pool.get_method_handle(bootstrap_method_ref)?;
        let arguments = bootstrap_arguments
            .iter()
            .map(|it| ctx.constant_pool.get_constant_value(it.get()))
            .collect::<Result<_, _>>()?;
        Ok(Self { method, arguments })
    }
}

impl ClassElement<'_> for InnerClassInfo {
    type Raw = raw_attributes::InnerClass;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            inner_class_info_index,
            outer_class_info_index,
            inner_name_index,
            inner_class_access_flags,
        } = raw;
        let inner_class_info_index = inner_class_info_index.get();
        let outer_class_info_index = outer_class_info_index.get();
        let inner_name_index = inner_name_index.get();
        let inner_class_access_flags = inner_class_access_flags.get();
        let inner_class = ctx.constant_pool.get_class_name(inner_class_info_index)?;
        let outer_class = if outer_class_info_index == 0 {
            None
        } else {
            let the_class = ctx.constant_pool.get_class_name(outer_class_info_index)?;
            Some(the_class)
        };
        let inner_name = if inner_name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(inner_name_index)?.to_owned())
        };
        let access_flags = NestedClassAccessFlags::from_bits(inner_class_access_flags)
            .ok_or(ParseError::malform("Invalid nested class access flags"))?;
        Ok(Self {
            inner_class,
            outer_class,
            inner_name,
            access_flags,
        })
    }
}

impl<'a> ClassElement<'a> for RecordComponent {
    type Raw = raw_attributes::RecordComponentInfo<'a>;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let component_type = ctx
            .constant_pool
            .get_str(descriptor_index)?
            .parse()
            .context("Invalid component type descriptor")?;

        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
        extract_attributes! {
            for attributes in "record_component" {
                let signature: Signature,
                let runtime_visible_annotations : RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations : RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                else let other_attributes
            }
        }

        Ok(RecordComponent {
            name,
            component_type,
            signature,
            annotations: Annotations {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: Annotations {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            other_attributes,
        })
    }
}

impl ClassElement<'_> for EnclosingMethod {
    type Raw = raw_attributes::EnclosingMethod;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            class_index,
            method_index,
        } = raw;
        let class_index = class_index.get();
        let method_index = method_index.get();
        let class = ctx.constant_pool.get_class_name(class_index)?;
        let method_name_and_desc = if method_index > 0 {
            let name_and_desc = ctx.constant_pool.get_method_name_and_type(method_index)?;
            Some(name_and_desc)
        } else {
            None
        };
        Ok(EnclosingMethod {
            class,
            method_name_and_desc,
        })
    }
}
