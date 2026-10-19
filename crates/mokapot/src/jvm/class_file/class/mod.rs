//! Class-file parsing and model conversion.

use crate::jvm::{annotation::AnnotationsData, class::record_component::RecordComponentData};

use zerocopy::byteorder::big_endian::U16;

use crate::jvm::class_file::attribute::extract_attributes;

pub(super) mod raw;

use crate::jvm::class_file::flags::parse_flags;

mod decode;

use super::{
    ParseError, ResolutionContext,
    attribute::AttributeInfo,
    constant_pool::{ConstantPool, RawConstantPool},
    field::FieldInfo,
    method::MethodInfo,
    resolve::Resolve,
};
use crate::{
    intrinsics::{ExactSizeIteratorExt, see_jvm_spec},
    jvm::{
        Class,
        class::{self, Version},
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
    interfaces: &'a [U16],
    fields: Box<[FieldInfo<'a>]>,
    methods: Box<[MethodInfo<'a>]>,
    attributes: Box<[&'a AttributeInfo]>,
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
        Self::from_raw(ClassFile::parse_complete(bytes)?)
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
        let access_flags =
            parse_flags::<class::AccessFlags>(access_flags, "Invalid class access flags")?;
        let constant_pool = ConstantPool::from_raw(constant_pool)?;
        let class_id = constant_pool.class_id(this_class)?;
        let class_name = constant_pool.class_name(class_id);
        let super_class = match super_class {
            0 if class_name.as_java_str() == "java/lang/Object" => None,
            0 if access_flags.contains(class::AccessFlags::MODULE) => None,
            0 => Err(ParseError::malform(
                "Class must have a super type except for java/lang/Object or a module",
            ))?,
            it => Some(constant_pool.class_id(it)?),
        };

        let mut ctx = ResolutionContext {
            constant_pool,
            class_version: version,
        };

        let attributes = attributes.into_iter().map(|it| it.resolve(&mut ctx));

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
            name: class_id,
            super_class,
            interfaces: interfaces
                .iter()
                .map(|it| ctx.constant_pool.class_id(it.get()))
                .try_collect_exact()?,
            fields: fields
                .into_iter()
                .map(|it| it.resolve(&mut ctx))
                .try_collect_exact()?,
            methods: methods
                .into_iter()
                .map(|it| it.resolve(&mut ctx))
                .try_collect_exact()?,
            source_file,
            inner_classes,
            enclosing_method,
            source_debug_extension,
            annotations: crate::jvm::annotation::AnnotationsData {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: crate::jvm::annotation::AnnotationsData {
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
            constant_pool: ctx.constant_pool.finish(),
        })
    }
}

impl Resolve for raw::RecordComponentInfo<'_> {
    type Output = RecordComponentData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw::RecordComponentInfo {
            name_index,
            descriptor_index,
            attributes,
        } = self;
        let name = ctx.constant_pool.utf8_id(name_index)?;
        let component_type = ctx
            .constant_pool
            .intern_field_type(descriptor_index)
            .context("Invalid component type descriptor")?;
        let attributes = attributes.into_iter().map(|it| it.resolve(ctx));

        extract_attributes! {
            for attributes in "record_component" {
                let signature: Signature,
                let runtime_visible_annotations
                    : RuntimeVisibleAnnotations as unwrap_or_default,
                let runtime_invisible_annotations
                    : RuntimeInvisibleAnnotations as unwrap_or_default,
                let runtime_visible_type_annotations
                    : RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations
                    : RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                else let other_attributes
            }
        }

        Ok(RecordComponentData {
            name,
            component_type,
            signature,
            annotations: AnnotationsData {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: AnnotationsData {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            other_attributes,
        })
    }
}
