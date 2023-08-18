use crate::utils::{read_u16, read_u32};

use super::{
    attributes::{
        annotation::{Annotation, ElementValue, TypeAnnotation},
        class_file::{BootstrapMethod, InnerClassInfo, RecordComponent},
        methods::{MethodBody, MethodParameter},
        module::{Module, PackageReference},
        Attribute, AttributeList, EnclosingMethod,
    },
    constant_pool::ConstantPool,
    fields::{Field, FieldInfo},
};

#[derive(Debug)]
pub struct ClassFile {
    version: ClassFileVersion,
    constant_pool: ConstantPool,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<FieldInfo>,
    methods: Vec<MethodInfo>,
    attributes: AttributeList,
}

pub struct Class {
    pub version: ClassFileVersion,
    pub access_flags: u16,
    pub this_class: ClassReference,
    pub super_class: ClassReference,
    pub interfaces: Vec<ClassReference>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub source_file: Option<String>,
    pub inner_classes: Vec<InnerClassInfo>,
    pub enclosing_method: Option<EnclosingMethod>,
    pub source_debug_extension: Vec<u8>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub module: Option<Module>,
    pub module_packages: Vec<PackageReference>,
    pub module_main_class: Option<ClassReference>,
    pub nest_host: Option<ClassReference>,
    pub nest_members: Vec<ClassReference>,
    pub permitted_subclasses: Vec<ClassReference>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<String>,
    pub record: Vec<RecordComponent>,
}

impl ClassFile {
    pub fn to_class(self) -> Result<Class, ClassFileParsingError> {
        let this_class = self.constant_pool.get_class_ref(self.this_class)?;
        let super_class = self.constant_pool.get_class_ref(self.super_class)?;
        let interfaces = self
            .interfaces
            .into_iter()
            .map(|i| self.constant_pool.get_class_ref(i))
            .collect::<Result<Vec<_>, ClassFileParsingError>>()?;
        let fields = self
            .fields
            .into_iter()
            .map(|f| f.to_field(&self.constant_pool))
            .collect::<Result<Vec<Field>, ClassFileParsingError>>()?;
        let methods = self
            .methods
            .into_iter()
            .map(|m| m.to_method(&self.constant_pool))
            .collect::<Result<Vec<Method>, ClassFileParsingError>>()?;

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
        for attr in self.attributes.into_iter() {
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
            version: self.version,
            access_flags: self.access_flags,
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

#[derive(Debug, Copy, Clone)]
/// The version of a class file.
pub struct ClassFileVersion {
    /// The major version number.
    pub major: u16,
    /// the minor version number.
    pub minor: u16,
}

impl ClassFileVersion {
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

pub struct Method {
    pub access_flags: u16,
    pub name: String,
    pub descriptor: String,
    pub body: Option<MethodBody>,
    pub excaptions: Vec<ClassReference>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    pub annotation_default: Option<ElementValue>,
    pub parameters: Vec<MethodParameter>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<String>,
}

#[derive(Debug)]
pub struct MethodInfo {
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: AttributeList,
}

impl MethodInfo {
    pub(crate) fn to_method(
        self,
        constant_pool: &ConstantPool,
    ) -> Result<Method, ClassFileParsingError> {
        let access_flags = self.access_flags;
        let name = constant_pool.get_string(self.name_index)?;
        let descriptor = constant_pool.get_string(self.descriptor_index)?;

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
        for attr in self.attributes.into_iter() {
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

    fn parse_multiple<R>(
        reader: &mut R,
        methods_count: u16,
        constant_pool: &ConstantPool,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut methods = Vec::with_capacity(methods_count as usize);
        for _ in 0..methods_count {
            methods.push(Self::parse(reader, constant_pool)?);
        }
        Ok(methods)
    }

    fn parse<R>(reader: &mut R, constant_pool: &ConstantPool) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let access_flags = read_u16(reader)?;
        let name_index = read_u16(reader)?;
        let descriptor_index = read_u16(reader)?;
        let attributes = AttributeList::parse(reader, constant_pool)?;
        Ok(Self {
            access_flags,
            name_index,
            descriptor_index,
            attributes,
        })
    }
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

impl ClassFile {
    pub fn parse<R>(reader: &mut R) -> Result<ClassFile, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let magic = read_u32(reader)?;
        if magic != JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::MalformedClassFile);
        }
        let version = ClassFileVersion::parse(reader)?;
        let constant_pool = ConstantPool::parse(reader)?;
        let access_flags = read_u16(reader)?;
        let this_class = read_u16(reader)?;
        let super_class = read_u16(reader)?;
        let interfaces_count = read_u16(reader)?;
        let mut interfaces: Vec<u16> = Vec::with_capacity(interfaces_count as usize);
        for _ in 0..interfaces_count {
            interfaces.push(read_u16(reader)?);
        }
        let fields_count = read_u16(reader)?;
        let fields = FieldInfo::parse_multiple(reader, fields_count, &constant_pool)?;
        let methods_count = read_u16(reader)?;
        let methods = MethodInfo::parse_multiple(reader, methods_count, &constant_pool)?;
        let attributes = AttributeList::parse(reader, &constant_pool)?;
        let mut may_remain: [u8; 1] = [0];
        let remain = reader.read(&mut may_remain)?;
        if remain == 1 {
            return Err(ClassFileParsingError::UnexpectedData);
        }
        Ok(ClassFile {
            version,
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

#[derive(Debug, PartialEq, Eq)]
pub struct ClassReference {
    pub name: String,
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
