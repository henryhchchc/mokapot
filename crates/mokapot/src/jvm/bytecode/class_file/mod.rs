//! Class-file parsing, generation, and model conversion.

mod io;

use std::io::Write;

use itertools::Itertools;

use super::{
    FromBytecode, GenerationError, ParseError, ParsingContext, ToBytecode,
    attribute::{Attribute, AttributeInfo},
    class_element::ClassElement,
    field_info::FieldInfo,
    method_info::MethodInfo,
    raw_attributes,
};
use crate::{
    intrinsics::{attributes_into_iter, extract_attributes, see_jvm_spec},
    jvm::{
        Class,
        class::{
            self, BootstrapMethod, ConstantPool, EnclosingMethod, InnerClassInfo,
            NestedClassAccessFlags, RecordComponent, Version,
        },
        errors::ParsingErrorContext,
        references::ClassRef,
    },
    types::Descriptor,
};

/// The raw representation of a class file.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug)]
pub(crate) struct ClassFile {
    minor_version: u16,
    major_version: u16,
    constant_pool: ConstantPool,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<FieldInfo>,
    methods: Vec<MethodInfo>,
    attributes: Vec<AttributeInfo>,
}

const JAVA_CLASS_MAGIC: u32 = 0xCAFE_BABE;

impl Class {
    /// Parses a class file from the given reader.
    /// # Errors
    /// See [`ParseError`] for more information.
    /// # Example
    /// ```no_run
    /// use mokapot::jvm::Class;
    /// use std::fs::File;
    /// use std::io::BufReader;
    ///
    /// let file = File::open("path/to/class/file").unwrap();
    /// let mut reader = BufReader::new(file);
    /// let class = Class::from_reader(&mut reader).unwrap();
    /// ```
    pub fn from_reader<R>(reader: &mut R) -> Result<Class, ParseError>
    where
        R: std::io::Read + ?Sized,
    {
        let class_file = ClassFile::from_reader(reader)?;
        Class::from_raw(class_file)
    }

    /// Writes the class file to the given writer.
    /// # Errors
    /// See [`GenerationError`] for more information.
    #[instability::unstable(feature = "bytecode-generation")]
    pub fn to_writer<W>(self, writer: &mut W) -> Result<(), GenerationError>
    where
        W: Write + ?Sized,
    {
        let class_file = self.into_raw()?;
        class_file.to_writer(writer)
    }
}

impl Class {
    pub(crate) fn from_raw(raw: ClassFile) -> Result<Self, ParseError> {
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
        let ClassRef(binary_name) = constant_pool.get_class_ref(this_class)?;
        let super_class = match super_class {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(class::AccessFlags::MODULE) => None,
            0 => Err(ParseError::malform(
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

        let interfaces = interfaces
            .into_iter()
            .map(|it| ctx.constant_pool.get_class_ref(it))
            .collect::<Result<_, _>>()?;
        let fields = fields
            .into_iter()
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
        let methods = methods
            .into_iter()
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
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
            other_attributes,
        })
    }

    pub(crate) fn into_raw(self) -> Result<ClassFile, GenerationError> {
        let mut constant_pool = ConstantPool::new();
        let class_ref = self.make_ref();
        let this_class = constant_pool.put_class_ref(&class_ref)?;
        let super_class = self
            .super_class
            .as_ref()
            .map(|it| constant_pool.put_class_ref(it))
            .transpose()?
            .unwrap_or(0);
        let interfaces = self
            .interfaces
            .iter()
            .map(|it| constant_pool.put_class_ref(it))
            .try_collect()?;
        let fields = self
            .fields
            .into_iter()
            .map(|it| it.into_raw(&mut constant_pool))
            .try_collect()?;
        let methods = self
            .methods
            .into_iter()
            .map(|it| it.into_raw(&mut constant_pool))
            .try_collect()?;
        let attributes = [
            self.source_file.map(Attribute::SourceFile),
            Some(self.inner_classes)
                .filter(|it| !it.is_empty())
                .map(Attribute::InnerClasses),
            self.enclosing_method.map(Attribute::EnclosingMethod),
            self.source_debug_extension
                .map(Attribute::SourceDebugExtension),
            Some(self.bootstrap_methods)
                .filter(|it| !it.is_empty())
                .map(Attribute::BootstrapMethods),
            self.module.map(Attribute::Module),
            Some(self.module_packages)
                .filter(|it| !it.is_empty())
                .map(Attribute::ModulePackages),
            self.module_main_class.map(Attribute::ModuleMainClass),
            self.nest_host.map(Attribute::NestHost),
            Some(self.nest_members)
                .filter(|it| !it.is_empty())
                .map(Attribute::NestMembers),
            Some(self.permitted_subclasses)
                .filter(|it| !it.is_empty())
                .map(Attribute::PermittedSubclasses),
            self.signature.map(Attribute::Signature),
            self.record.map(Attribute::Record),
            self.is_synthetic.then_some(Attribute::Synthetic),
            self.is_deprecated.then_some(Attribute::Deprecated),
        ]
        .into_iter()
        .flatten()
        .chain(attributes_into_iter!(self))
        .map(|it| it.into_raw(&mut constant_pool))
        .try_collect()?;
        let raw = ClassFile {
            minor_version: self.version.minor(),
            major_version: self.version.major(),
            constant_pool,
            access_flags: self.access_flags.bits(),
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        };
        Ok(raw)
    }
}

impl ClassElement for BootstrapMethod {
    type Raw = raw_attributes::BootstrapMethod;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            method_ref_idx,
            arguments,
        } = raw;
        let method = ctx.constant_pool.get_method_handle(method_ref_idx)?;
        let arguments = arguments
            .into_iter()
            .map(|it| ctx.constant_pool.get_constant_value(it))
            .collect::<Result<_, _>>()?;
        Ok(Self { method, arguments })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let method_ref_idx = cp.put_method_handle(self.method)?;
        let arguments = self
            .arguments
            .into_iter()
            .map(|arg| cp.put_constant_value(arg))
            .try_collect()?;
        Ok(Self::Raw {
            method_ref_idx,
            arguments,
        })
    }
}

impl ClassElement for InnerClassInfo {
    type Raw = raw_attributes::InnerClass;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            info_index,
            outer_class_info_index,
            inner_name_index,
            access_flags,
        } = raw;
        let inner_class = ctx.constant_pool.get_class_ref(info_index)?;
        let outer_class = if outer_class_info_index == 0 {
            None
        } else {
            let the_class = ctx.constant_pool.get_class_ref(outer_class_info_index)?;
            Some(the_class)
        };
        let inner_name = if inner_name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(inner_name_index)?.to_owned())
        };
        let access_flags = NestedClassAccessFlags::from_bits(access_flags)
            .ok_or(ParseError::malform("Invalid nested class access flags"))?;
        Ok(Self {
            inner_class,
            outer_class,
            inner_name,
            access_flags,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let info_index = cp.put_class_ref(&self.inner_class)?;
        let outer_class_info_index = self
            .outer_class
            .as_ref()
            .map(|it| cp.put_class_ref(it))
            .transpose()?
            .unwrap_or(0);
        let inner_name_index = self
            .inner_name
            .map(|it| cp.put_string(it))
            .transpose()?
            .unwrap_or(0);
        let access_flags = self.access_flags.into_raw(cp)?;
        Ok(Self::Raw {
            info_index,
            outer_class_info_index,
            inner_name_index,
            access_flags,
        })
    }
}

impl ClassElement for RecordComponent {
    type Raw = raw_attributes::RecordComponentInfo;
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
            runtime_visible_annotations,
            runtime_invisible_annotations,
            runtime_visible_type_annotations,
            runtime_invisible_type_annotations,
            other_attributes,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let name_index = cp.put_string(self.name)?;
        let descriptor_index = cp.put_string(self.component_type.descriptor())?;
        let attributes = self
            .signature
            .map(Attribute::Signature)
            .into_iter()
            .chain(attributes_into_iter!(self))
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        Ok(Self::Raw {
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

impl ClassElement for EnclosingMethod {
    type Raw = raw_attributes::EnclosingMethod;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            class_index,
            method_index,
        } = raw;
        let class = ctx.constant_pool.get_class_ref(class_index)?;
        let method_name_and_desc = if method_index > 0 {
            let name_and_desc = ctx.constant_pool.get_name_and_type(method_index)?;
            Some(name_and_desc)
        } else {
            None
        };
        Ok(EnclosingMethod {
            class,
            method_name_and_desc,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let class_index = cp.put_class_ref(&self.class)?;
        let method_index = self
            .method_name_and_desc
            .map(|(name, desc)| cp.put_name_and_type(name, &desc))
            .transpose()?
            .unwrap_or(0);

        Ok(Self::Raw {
            class_index,
            method_index,
        })
    }
}
