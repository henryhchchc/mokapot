use crate::utils::{read_u16, read_u32};

use super::{
    class::{Class, ClassVersion},
    field::Field,
    method::Method,
    parsing::{
        attribute::{Attribute, AttributeList},
        constant_pool::ConstantPool,
    },
};

pub struct ClassParser<'a> {
    reader: &'a mut dyn std::io::Read,
}

impl<'a> ClassParser<'a> {
    pub fn parse(mut self) -> Result<Class, ClassFileParsingError> {
        let reader = &mut self.reader;

        let magic = read_u32(reader)?;
        if magic != JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::MalformedClassFile);
        }
        let version = ClassVersion::parse(reader)?;
        let constant_pool = ConstantPool::parse(reader, version)?;
        let access_flags = read_u16(reader)?;
        let this_class_idx = read_u16(reader)?;
        let this_class = constant_pool.get_class_ref(&this_class_idx)?;
        let super_class_idx = read_u16(reader)?;
        let super_class = constant_pool.get_class_ref(&super_class_idx)?;
        let interfaces_count = read_u16(reader)?;
        let mut interfaces = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            let interface_idx = read_u16(reader)?;
            let interface_ref = constant_pool.get_class_ref(&interface_idx)?;
            interfaces.push(interface_ref);
        }
        let fields_count = read_u16(reader)?;
        let fields = Field::parse_multiple(reader, fields_count, &constant_pool)?;
        let methods_count = read_u16(reader)?;
        let methods = Method::parse_multiple(reader, methods_count, &constant_pool)?;
        let attributes = AttributeList::parse(reader, &constant_pool)?;
        let mut may_remain: [u8; 1] = [0];
        let remain = reader.read(&mut may_remain)?;
        if remain == 1 {
            return Err(ClassFileParsingError::UnexpectedData);
        }

        let mut source_file = None;
        let mut inner_classes = None;
        let mut enclosing_method = None;
        let mut source_debug_extension = None;
        let mut rt_visible_anno = None;
        let mut rt_invisible_anno = None;
        let mut rt_visible_type_anno = None;
        let mut rt_invisible_type_anno = None;
        let mut bootstrap_methods = None;
        let mut module = None;
        let mut module_packages = None;
        let mut module_main_class = None;
        let mut nest_host = None;
        let mut nest_members = None;
        let mut permitted_subclasses = None;
        let mut is_synthetic = false;
        let mut is_deprecated = false;
        let mut signature = None;
        let mut record = None;
        for attr in attributes.into_iter() {
            match attr {
                Attribute::SourceFile(file_name) => source_file = Some(file_name),
                Attribute::InnerClasses(it) => inner_classes = Some(it),
                Attribute::EnclosingMethod(em) => enclosing_method = Some(em),
                Attribute::SourceDebugExtension(sde) => source_debug_extension = Some(sde),
                Attribute::RuntimeVisibleAnnotations(rv) => rt_visible_anno = Some(rv),
                Attribute::RuntimeInvisibleAnnotations(ri) => rt_invisible_anno = Some(ri),
                Attribute::RuntimeVisibleTypeAnnotations(rv) => rt_visible_type_anno = Some(rv),
                Attribute::RuntimeInvisibleTypeAnnotations(ri) => rt_invisible_type_anno = Some(ri),
                Attribute::BootstrapMethods(bm) => bootstrap_methods = Some(bm),
                Attribute::Module(m) => module = Some(m),
                Attribute::ModulePackages(mp) => module_packages = Some(mp),
                Attribute::ModuleMainClass(mmc) => module_main_class = Some(mmc),
                Attribute::NestHost(nh) => nest_host = Some(nh),
                Attribute::NestMembers(nm) => nest_members = Some(nm),
                Attribute::PermittedSubclasses(ps) => permitted_subclasses = Some(ps),
                Attribute::Synthetic => is_synthetic = true,
                Attribute::Deprecated => is_deprecated = true,
                Attribute::Signature(sig) => signature = Some(sig),
                Attribute::Record(rec) => record = Some(rec),
                _ => Err(ClassFileParsingError::UnexpectedAttribute)?,
            }
        }
        Ok(Class {
            version,
            access_flags,
            this_class,
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
            record: record.unwrap_or_default(),
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

impl Method {
    pub(crate) fn parse_multiple<R>(
        reader: &mut R,
        methods_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<Self>>
    where
        R: std::io::Read,
    {
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            let method = Self::parse(reader, constant_pool)?;
            methods.push(method);
        }
        Ok(methods)
    }
    pub(crate) fn parse<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(&name_index)?;
        let descriptor_index = read_u16(reader)?;
        let descriptor = constant_pool.get_string(&descriptor_index)?;

        let attributes = AttributeList::parse(reader, constant_pool)?;
        let mut body = None;
        let mut exceptions = None;
        let mut rt_visible_anno = None;
        let mut rt_invisible_anno = None;
        let mut rt_visible_type_anno = None;
        let mut rt_invisible_type_anno = None;
        let mut annotation_default = None;
        let mut method_parameters = None;
        let mut is_synthetic = false;
        let mut is_deprecated = false;
        let mut signature = None;
        for attr in attributes.into_iter() {
            match attr {
                Attribute::Code(b) => body = Some(b),
                Attribute::Exceptions(ex) => exceptions = Some(ex),
                Attribute::RuntimeVisibleAnnotations(rv) => rt_visible_anno = Some(rv),
                Attribute::RuntimeInvisibleAnnotations(ri) => rt_invisible_anno = Some(ri),
                Attribute::RuntimeVisibleTypeAnnotations(rt) => rt_visible_type_anno = Some(rt),
                Attribute::RuntimeInvisibleTypeAnnotations(rt) => rt_invisible_type_anno = Some(rt),
                Attribute::AnnotationDefault(ad) => annotation_default = Some(ad),
                Attribute::MethodParameters(mp) => method_parameters = Some(mp),
                Attribute::Synthetic => is_synthetic = true,
                Attribute::Deprecated => is_deprecated = true,
                Attribute::Signature(sig) => signature = Some(sig),
                _ => Err(ClassFileParsingError::UnexpectedAttribute)?,
            }
        }

        Ok(Method {
            access_flags,
            name,
            descriptor,
            body,
            excaptions: exceptions.unwrap_or_default(),
            runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
            runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
            runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
            runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
            annotation_default,
            parameters: method_parameters.unwrap_or_default(),
            is_synthetic,
            is_deprecated,
            signature,
        })
    }
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

impl<'a> ClassParser<'a> {
    pub fn from_reader<'r, R>(reader: &'r mut R) -> ClassParser<'r>
    where
        R: std::io::Read,
    {
        ClassParser { reader }
    }
}

#[derive(Debug)]
pub enum ClassFileParsingError {
    MalformedClassFile,
    MidmatchedConstantPoolTag,
    BadConstantPoolIndex,
    UnknownAttributeName(String),
    InvalidAttributeLength { expected: u32, actual: u32 },
    UnexpectedAttribute,
    UnexpectedData,
    InvalidElementValueTag(u8),
    InvalidTargetType(u8),
    InvalidTypePathKind,
    UnknownStackMapFrameType(u8),
    InvalidVerificationTypeInfoTag(u8),
    UnexpectedOpCode,
}

impl From<std::io::Error> for ClassFileParsingError {
    fn from(_value: std::io::Error) -> Self {
        ClassFileParsingError::MalformedClassFile
    }
}

impl std::fmt::Display for ClassFileParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oops")
    }
}

impl std::error::Error for ClassFileParsingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

pub type ClassFileParsingResult<T> = Result<T, ClassFileParsingError>;
