use std::io::{self, Read};

use crate::{
    jvm::{
        Class,
        class::{
            self, BootstrapMethod, ConstantPool, EnclosingMethod, InnerClassInfo,
            NestedClassAccessFlags, RecordComponent, Version,
        },
        parsing::reader_utils::ValueReaderExt,
        references::ClassRef,
    },
    macros::{extract_attributes, malform, see_jvm_spec},
};

use super::{
    Context, Error, ToWriter, ToWriterError, attribute::AttributeInfo, field_info::FieldInfo,
    jvm_element_parser::ClassElement, method_info::MethodInfo, raw_attributes,
    reader_utils::FromReader, write_length,
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
    /// See [`Error`] for more information.
    pub fn from_reader<R>(reader: &mut R) -> Result<Class, Error>
    where
        R: std::io::Read + ?Sized,
    {
        let class_file = ClassFile::from_reader(reader)?;
        Class::from_raw(class_file)
    }
}

impl FromReader for ClassFile {
    fn from_reader<R: Read + ?Sized>(reader: &mut R) -> io::Result<Self> {
        let magic: u32 = reader.read_value()?;
        if magic != JAVA_CLASS_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "This is not a Java class file",
            ));
        }
        let minor_version = reader.read_value()?;
        let major_version = reader.read_value()?;
        let constant_pool_count = reader.read_value()?;
        let constant_pool = ConstantPool::from_reader(reader, constant_pool_count)?;
        let access_flags = reader.read_value()?;
        let this_class = reader.read_value()?;
        let super_class = reader.read_value()?;
        let interfaces_count: u16 = reader.read_value()?;
        let interfaces = (0..interfaces_count)
            .map(|_| reader.read_value())
            .collect::<io::Result<_>>()?;
        let fields_count: u16 = reader.read_value()?;
        let fields = (0..fields_count)
            .map(|_| FieldInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;
        let methods_count: u16 = reader.read_value()?;
        let methods = (0..methods_count)
            .map(|_| MethodInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;
        let attributes_count: u16 = reader.read_value()?;
        let attributes = (0..attributes_count)
            .map(|_| AttributeInfo::from_reader(reader))
            .collect::<io::Result<_>>()?;

        Ok(Self {
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
        })
    }
}

impl ToWriter for ClassFile {
    fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), ToWriterError> {
        writer.write_all(&JAVA_CLASS_MAGIC.to_be_bytes())?;
        writer.write_all(&self.minor_version.to_be_bytes())?;
        writer.write_all(&self.major_version.to_be_bytes())?;
        self.constant_pool.to_writer(writer)?;
        writer.write_all(&self.access_flags.to_be_bytes())?;
        writer.write_all(&self.this_class.to_be_bytes())?;
        writer.write_all(&self.super_class.to_be_bytes())?;
        write_length::<u16, _>(writer, self.interfaces.len())?;
        for interface_idx in &self.interfaces {
            writer.write_all(&interface_idx.to_be_bytes())?;
        }
        write_length::<u16, _>(writer, self.fields.len())?;
        for field in &self.fields {
            field.to_writer(writer)?;
        }
        write_length::<u16, _>(writer, self.methods.len())?;
        for method in &self.methods {
            method.to_writer(writer)?;
        }
        self.attributes.to_writer(writer)?;
        Ok(())
    }
}

impl Class {
    pub(crate) fn from_raw(raw: ClassFile) -> Result<Self, Error> {
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
            .ok_or(Error::UnknownFlags("ClassAccessFlags", access_flags))?;
        let ClassRef { binary_name } = constant_pool.get_class_ref(this_class)?;
        let super_class = match super_class {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(class::AccessFlags::MODULE) => None,
            0 => malform!("Class must have a super type except for java/lang/Object or a module"),
            it => Some(constant_pool.get_class_ref(it)?),
        };

        let parsing_context = Context {
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

impl ClassElement for BootstrapMethod {
    type Raw = raw_attributes::BootstrapMethod;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
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
}

impl ClassElement for InnerClassInfo {
    type Raw = raw_attributes::InnerClass;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
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
            .ok_or(Error::UnknownFlags("NextClassAccessFlags", access_flags))?;
        Ok(Self {
            inner_class,
            outer_class,
            inner_name,
            access_flags,
        })
    }
}

impl ClassElement for RecordComponent {
    type Raw = raw_attributes::RecordComponentInfo;
    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            name_index,
            descriptor_index,
            attributes,
        } = raw;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let component_type = ctx.constant_pool.get_str(descriptor_index)?.parse()?;

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

impl ClassElement for EnclosingMethod {
    type Raw = raw_attributes::EnclosingMethod;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
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
}
