//! Class-file parsing and model conversion.

use crate::jvm::{annotation::AnnotationsData, class::record_component::RecordComponentData};

use zerocopy::byteorder::big_endian::U16;

use crate::jvm::class_file::attribute::extract_attributes;

pub(super) mod raw;

use crate::jvm::class_file::flags::parse_flags;

mod decode;

use super::{
    ParseError, ResolutionContext, attribute::AttributeInfo, constant_pool::RawConstantPool,
    field::FieldInfo, method::MethodInfo, resolve::Resolve,
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

const PREVIEW_FEATURES: u16 = u16::MAX;

impl Version {
    pub(super) fn from_versions(major: u16, minor: u16) -> Result<Self, ParseError> {
        match (major, minor) {
            (45, minor) => Ok(Version::Jdk1_1(minor)),
            (46, 0x0000) => Ok(Version::Jdk1_2),
            (47, 0x0000) => Ok(Version::Jdk1_3),
            (48, 0x0000) => Ok(Version::Jdk1_4),
            (49, 0x0000) => Ok(Version::Jdk5),
            (50, 0x0000) => Ok(Version::Jdk6),
            (51, 0x0000) => Ok(Version::Jdk7),
            (52, 0x0000) => Ok(Version::Jdk8),
            (53, 0x0000) => Ok(Version::Jdk9),
            (54, 0x0000) => Ok(Version::Jdk10),
            (55, 0x0000) => Ok(Version::Jdk11),
            (56, 0x0000) => Ok(Version::Jdk12(false)),
            (56, PREVIEW_FEATURES) => Ok(Version::Jdk12(true)),
            (57, 0x0000) => Ok(Version::Jdk13(false)),
            (57, PREVIEW_FEATURES) => Ok(Version::Jdk13(true)),
            (58, 0x0000) => Ok(Version::Jdk14(false)),
            (58, PREVIEW_FEATURES) => Ok(Version::Jdk14(true)),
            (59, 0x0000) => Ok(Version::Jdk15(false)),
            (59, PREVIEW_FEATURES) => Ok(Version::Jdk15(true)),
            (60, 0x0000) => Ok(Version::Jdk16(false)),
            (60, PREVIEW_FEATURES) => Ok(Version::Jdk16(true)),
            (61, 0x0000) => Ok(Version::Jdk17(false)),
            (61, PREVIEW_FEATURES) => Ok(Version::Jdk17(true)),
            (62, 0x0000) => Ok(Version::Jdk18(false)),
            (62, PREVIEW_FEATURES) => Ok(Version::Jdk18(true)),
            (63, 0x0000) => Ok(Version::Jdk19(false)),
            (63, PREVIEW_FEATURES) => Ok(Version::Jdk19(true)),
            (64, 0x0000) => Ok(Version::Jdk20(false)),
            (64, PREVIEW_FEATURES) => Ok(Version::Jdk20(true)),
            (65, 0x0000) => Ok(Version::Jdk21(false)),
            (65, PREVIEW_FEATURES) => Ok(Version::Jdk21(true)),
            (66, 0x0000) => Ok(Version::Jdk22(false)),
            (66, PREVIEW_FEATURES) => Ok(Version::Jdk22(true)),
            (67, 0x0000) => Ok(Version::Jdk23(false)),
            (67, PREVIEW_FEATURES) => Ok(Version::Jdk23(true)),
            (68, 0x0000) => Ok(Version::Jdk24(false)),
            (68, PREVIEW_FEATURES) => Ok(Version::Jdk24(true)),
            (69, 0x0000) => Ok(Version::Jdk25(false)),
            (69, PREVIEW_FEATURES) => Ok(Version::Jdk25(true)),
            (70, 0x0000) => Ok(Version::Jdk26(false)),
            (70, PREVIEW_FEATURES) => Ok(Version::Jdk26(true)),
            (71, 0x0000) => Ok(Version::Jdk27(false)),
            (71, PREVIEW_FEATURES) => Ok(Version::Jdk27(true)),
            (major, _) if major > Version::LATEST.major() => {
                Err(ParseError::malform("Unsupported class version"))
            }
            _ => Err(ParseError::malform("Invalid class version")),
        }
    }
}

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
        let constant_pool = super::constant_pool::resolve(constant_pool)?;
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
            annotations: AnnotationsData {
                runtime_visible: runtime_visible_annotations,
                runtime_invisible: runtime_invisible_annotations,
            },
            type_annotations: AnnotationsData {
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
