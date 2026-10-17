use crate::types::class_name::ClassName;
use crate::types::package_name::PackageName;
use zerocopy::{
    FromBytes,
    byteorder::big_endian::{U16, U32},
};

use std::io::{self, Read};

use itertools::Itertools;

use super::{
    FromBytecode, ParseError, ParsingContext,
    class_element::ClassElement,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    reader::{BytecodeReader, read_bytes},
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Annotation, ConstantValue, Module, TypeAnnotation,
        annotation::ElementValue,
        class::{BootstrapMethod, EnclosingMethod, InnerClassInfo, RecordComponent},
        code::{LineNumberTableEntry, MethodBody, StackMapFrame},
        method::ParameterInfo,
    },
};

/// Represent an attribute of a class file, method, field, or code.
#[doc = see_jvm_spec!(4, 7)]
#[derive(Debug)]
pub(crate) struct AttributeInfo {
    attribute_name_index: u16,
    info: Box<[u8]>,
}

impl AttributeInfo {
    const fn from_raw_parts(attribute_name_index: u16, info: Box<[u8]>) -> Self {
        Self {
            attribute_name_index,
            info,
        }
    }
}

impl FromBytecode for AttributeInfo {
    fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let attribute_name_index = U16::read_from_io(&mut reader)?.get();
        let attribute_length: u32 = U32::read_from_io(&mut reader)?.get();
        let attribute_length = usize::try_from(attribute_length)
            .expect("32-bit size is not supported on the current platform");
        let info = read_bytes(&mut reader, attribute_length)?;
        Ok(Self::from_raw_parts(attribute_name_index, info))
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum Attribute {
    ConstantValue(ConstantValue),
    Code(MethodBody),
    StackMapTable(Vec<StackMapFrame>),
    Exceptions(Vec<ClassName>),
    SourceFile(String),
    LineNumberTable(Vec<LineNumberTableEntry>),
    InnerClasses(Vec<InnerClassInfo>),
    Synthetic,
    Deprecated,
    EnclosingMethod(EnclosingMethod),
    Signature(String),
    SourceDebugExtension(Box<[u8]>),
    LocalVariableTable(Vec<LocalVariableDescAttr>),
    LocalVariableTypeTable(Vec<LocalVariableTypeAttr>),
    RuntimeVisibleAnnotations(Vec<Annotation>),
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    AnnotationDefault(ElementValue),
    BootstrapMethods(Vec<BootstrapMethod>),
    MethodParameters(Vec<ParameterInfo>),
    Module(Module),
    ModulePackages(Vec<PackageName>),
    ModuleMainClass(ClassName),
    NestHost(ClassName),
    NestMembers(Vec<ClassName>),
    Record(Vec<RecordComponent>),
    PermittedSubclasses(Vec<ClassName>),
    Unrecognized(String, Box<[u8]>),
}

impl Attribute {
    pub fn name(&self) -> &str {
        match self {
            Self::ConstantValue(_) => "ConstantValue",
            Self::Code(_) => "Code",
            Self::StackMapTable(_) => "StackMapTable",
            Self::Exceptions(_) => "Exceptions",
            Self::SourceFile(_) => "SourceFile",
            Self::LineNumberTable(_) => "LineNumberTable",
            Self::InnerClasses(_) => "InnerClasses",
            Self::Synthetic => "Synthetic",
            Self::Deprecated => "Deprecated",
            Self::EnclosingMethod(_) => "EnclosingMethod",
            Self::Signature(_) => "Signature",
            Self::SourceDebugExtension(_) => "SourceDebugExtension",
            Self::LocalVariableTable(_) => "LocalVariableTable",
            Self::LocalVariableTypeTable(_) => "LocalVariableTypeTable",
            Self::RuntimeVisibleAnnotations(_) => "RuntimeVisibleAnnotations",
            Self::RuntimeInvisibleAnnotations(_) => "RuntimeInvisibleAnnotations",
            Self::RuntimeVisibleParameterAnnotations(_) => "RuntimeVisibleParameterAnnotations",
            Self::RuntimeInvisibleParameterAnnotations(_) => "RuntimeInvisibleParameterAnnotations",
            Self::RuntimeVisibleTypeAnnotations(_) => "RuntimeVisibleTypeAnnotations",
            Self::RuntimeInvisibleTypeAnnotations(_) => "RuntimeInvisibleTypeAnnotations",
            Self::AnnotationDefault(_) => "AnnotationDefault",
            Self::BootstrapMethods(_) => "BootstrapMethods",
            Self::MethodParameters(_) => "MethodParameters",
            Self::Module(_) => "Module",
            Self::ModulePackages(_) => "ModulePackages",
            Self::ModuleMainClass(_) => "ModuleMainClass",
            Self::NestHost(_) => "NestHost",
            Self::NestMembers(_) => "NestMembers",
            Self::Record(_) => "Record",
            Self::PermittedSubclasses(_) => "PermittedSubclasses",
            Self::Unrecognized(name, _) => name,
        }
    }
}

macro_rules! parse {
    (@count u8; $reader:expr) => { u8::read_from_io(&mut *$reader)? };
    (@count u16; $reader:expr) => { U16::read_from_io(&mut *$reader)?.get() };
    ($reader:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {{
        let raw = $reader.decode_value()?;
        ClassElement::from_raw(raw, $ctx)$( .map(Self::$attr) )?
    }};
    ($len_type:ident; $reader:expr_2021, || $with:expr_2021 $(=> $attr:ident )?) => {{
        let count = parse!(@count $len_type; $reader);
        (0..count).map(|_| $with).try_collect()$( .map(Self::$attr) )?
    }};
    ($len_type:ident; $reader:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {
        parse![$len_type; $reader, || parse!($reader, $ctx)] $( .map(Self::$attr) )?
    };
}

impl ClassElement for Attribute {
    type Raw = AttributeInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let AttributeInfo {
            attribute_name_index,
            info,
        } = raw;
        let name = ctx.constant_pool.get_str(attribute_name_index)?;
        let reader = &mut info.as_ref();

        let result = match name {
            "ConstantValue" => {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool
                    .get_constant_value(idx)
                    .map(Self::ConstantValue)
            }
            "Code" => parse!(reader, ctx => Code),
            "StackMapTable" => parse![u16; reader, ctx => StackMapTable],
            "Exceptions" => parse![u16; reader, || {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool.get_class_name(idx)
            } => Exceptions],
            "InnerClasses" => parse![u16; reader, ctx => InnerClasses],
            "EnclosingMethod" => parse!(reader, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => parse_string(reader.by_ref(), ctx).map(Self::Signature),
            "SourceFile" => parse_string(reader.by_ref(), ctx).map(Self::SourceFile),
            "SourceDebugExtension" => {
                return Ok(Self::SourceDebugExtension(info));
            }
            "LineNumberTable" => parse![u16; reader, ctx => LineNumberTable],
            "LocalVariableTable" => parse![u16; reader, ctx => LocalVariableTable],
            "LocalVariableTypeTable" => parse![u16; reader, ctx => LocalVariableTypeTable],
            "RuntimeVisibleAnnotations" => parse![u16; reader, ctx => RuntimeVisibleAnnotations],
            "RuntimeInvisibleAnnotations" => {
                parse![u16; reader, ctx => RuntimeInvisibleAnnotations]
            }
            "RuntimeVisibleParameterAnnotations" => parse![u8; reader, || parse![u16; reader, ctx]]
                .map(Self::RuntimeVisibleParameterAnnotations),
            "RuntimeInvisibleParameterAnnotations" => {
                parse![u8; reader, || parse![u16; reader, ctx] => RuntimeInvisibleParameterAnnotations]
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse![u16; reader, ctx => RuntimeVisibleTypeAnnotations]
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse![u16; reader, ctx => RuntimeInvisibleTypeAnnotations]
            }
            "AnnotationDefault" => parse!(reader, ctx => AnnotationDefault),
            "BootstrapMethods" => parse![u16; reader, ctx => BootstrapMethods],
            "MethodParameters" => parse![u8; reader, ctx => MethodParameters],
            "Module" => parse!(reader, ctx => Module),
            "ModulePackages" => parse![u16; reader, || {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool.get_package_name(idx)
            } => ModulePackages],
            "ModuleMainClass" => {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool
                    .get_class_name(idx)
                    .map(Self::ModuleMainClass)
            }
            "NestHost" => {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool.get_class_name(idx).map(Self::NestHost)
            }
            "NestMembers" => parse![u16; reader, || {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool.get_class_name(idx)
            }]
            .map(Self::NestMembers),
            "Record" => parse![u16; reader, ctx => Record],
            "PermittedSubclasses" => parse![u16; reader, || {
                let idx = U16::read_from_io(&mut *reader)?.get();
                ctx.constant_pool.get_class_name(idx)
            } => PermittedSubclasses],
            name => return Ok(Attribute::Unrecognized(name.to_owned(), info)),
        }?;
        if reader.is_empty() {
            Ok(result)
        } else {
            Err(ParseError::from(io::Error::new(
                io::ErrorKind::InvalidData,
                "Extra data at the end of the attribute",
            )))
        }
    }
}

#[inline]
fn parse_string<R: Read>(reader: R, ctx: &ParsingContext) -> Result<String, ParseError> {
    let str_idx = U16::read_from_io(reader)?.get();
    ctx.constant_pool.get_str(str_idx).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{
        bytecode::constant_pool::{RawConstantPool, RawEntry},
        class::Version,
    };

    #[test]
    fn parses_module_packages_with_package_constant_pool_entries() {
        let mut raw_pool = RawConstantPool::new();
        let attribute_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"ModulePackages".as_slice().into()))
            .unwrap();
        let package_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"example/api".as_slice().into()))
            .unwrap();
        raw_pool
            .put_entry(RawEntry::Package {
                name_index: package_name_index.into(),
            })
            .unwrap();
        let other_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"example/internal".as_slice().into()))
            .unwrap();
        raw_pool
            .put_entry(RawEntry::Package {
                name_index: other_name_index.into(),
            })
            .unwrap();
        let constant_pool = crate::jvm::class::ConstantPool::from_raw(raw_pool).unwrap();
        let ctx = ParsingContext {
            constant_pool,
            class_version: Version::LATEST,
            current_class_name: "module-info".parse().unwrap(),
        };
        let attribute_info =
            AttributeInfo::from_raw_parts(attribute_name_index, [0, 2, 0, 3, 0, 5].into());

        let attribute = Attribute::from_raw(attribute_info, &ctx).unwrap();

        let Attribute::ModulePackages(packages) = attribute else {
            panic!("expected ModulePackages attribute");
        };
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].as_str(), "example/api");
        assert_eq!(packages[1].as_str(), "example/internal");
    }
}
