use crate::types::class_name::ClassName;
use crate::types::package_name::PackageName;
use zerocopy::byteorder::big_endian::{U16, U32};

use itertools::Itertools;

use super::{
    FromBytecode, ParseError, ParsingContext,
    class_element::ClassElement,
    code::{LocalVariableDescAttr, LocalVariableTypeAttr},
    decoder::{BytecodeSlice, read_bytes, read_value},
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
pub(crate) struct AttributeInfo<'a> {
    attribute_name_index: u16,
    info: &'a [u8],
}

impl<'a> FromBytecode<'a> for AttributeInfo<'a> {
    fn from_bytes(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let attribute_name_index = read_value::<U16>(bytes)?.get();
        let attribute_length: u32 = read_value::<U32>(bytes)?.get();
        let attribute_length = usize::try_from(attribute_length)
            .expect("32-bit size is not supported on the current platform");
        let info = read_bytes(bytes, attribute_length)?;
        Ok(AttributeInfo {
            attribute_name_index,
            info,
        })
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
    (@count u8; $bytes:expr) => { read_value::<u8>(&mut *$bytes)? };
    (@count u16; $bytes:expr) => { read_value::<U16>(&mut *$bytes)?.get() };
    ($bytes:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {{
        let raw = $bytes.decode_value()?;
        ClassElement::from_raw(raw, $ctx)$( .map(Self::$attr) )?
    }};
    ($len_type:ident; $bytes:expr_2021, || $with:expr_2021 $(=> $attr:ident )?) => {{
        let count = parse!(@count $len_type; $bytes);
        (0..count).map(|_| $with).try_collect()$( .map(Self::$attr) )?
    }};
    ($len_type:ident; $bytes:expr_2021, $ctx:expr_2021 $(=> $attr:ident )?) => {
        parse![$len_type; $bytes, || parse!($bytes, $ctx)] $( .map(Self::$attr) )?
    };
}

impl<'a> ClassElement<'a> for Attribute {
    type Raw = AttributeInfo<'a>;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let AttributeInfo {
            attribute_name_index,
            mut info,
        } = raw;
        let name = ctx.constant_pool.get_str(attribute_name_index)?;
        let bytes = &mut info;

        let result = match name {
            "ConstantValue" => {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool
                    .get_constant_value(idx)
                    .map(Self::ConstantValue)
            }
            "Code" => parse!(bytes, ctx => Code),
            "StackMapTable" => parse![u16; bytes, ctx => StackMapTable],
            "Exceptions" => parse![u16; bytes, || {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool.get_class_name(idx)
            } => Exceptions],
            "InnerClasses" => parse![u16; bytes, ctx => InnerClasses],
            "EnclosingMethod" => parse!(bytes, ctx).map(Self::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => parse_string(bytes, ctx).map(Self::Signature),
            "SourceFile" => parse_string(bytes, ctx).map(Self::SourceFile),
            "SourceDebugExtension" => {
                return Ok(Self::SourceDebugExtension(info.into()));
            }
            "LineNumberTable" => parse![u16; bytes, ctx => LineNumberTable],
            "LocalVariableTable" => parse![u16; bytes, ctx => LocalVariableTable],
            "LocalVariableTypeTable" => parse![u16; bytes, ctx => LocalVariableTypeTable],
            "RuntimeVisibleAnnotations" => parse![u16; bytes, ctx => RuntimeVisibleAnnotations],
            "RuntimeInvisibleAnnotations" => {
                parse![u16; bytes, ctx => RuntimeInvisibleAnnotations]
            }
            "RuntimeVisibleParameterAnnotations" => parse![u8; bytes, || parse![u16; bytes, ctx]]
                .map(Self::RuntimeVisibleParameterAnnotations),
            "RuntimeInvisibleParameterAnnotations" => {
                parse![u8; bytes, || parse![u16; bytes, ctx] => RuntimeInvisibleParameterAnnotations]
            }
            "RuntimeVisibleTypeAnnotations" => {
                parse![u16; bytes, ctx => RuntimeVisibleTypeAnnotations]
            }
            "RuntimeInvisibleTypeAnnotations" => {
                parse![u16; bytes, ctx => RuntimeInvisibleTypeAnnotations]
            }
            "AnnotationDefault" => parse!(bytes, ctx => AnnotationDefault),
            "BootstrapMethods" => parse![u16; bytes, ctx => BootstrapMethods],
            "MethodParameters" => parse![u8; bytes, ctx => MethodParameters],
            "Module" => parse!(bytes, ctx => Module),
            "ModulePackages" => parse![u16; bytes, || {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool.get_package_name(idx)
            } => ModulePackages],
            "ModuleMainClass" => {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool
                    .get_class_name(idx)
                    .map(Self::ModuleMainClass)
            }
            "NestHost" => {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool.get_class_name(idx).map(Self::NestHost)
            }
            "NestMembers" => parse![u16; bytes, || {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool.get_class_name(idx)
            }]
            .map(Self::NestMembers),
            "Record" => parse![u16; bytes, ctx => Record],
            "PermittedSubclasses" => parse![u16; bytes, || {
                let idx = read_value::<U16>(&mut *bytes)?.get();
                ctx.constant_pool.get_class_name(idx)
            } => PermittedSubclasses],
            name => return Ok(Attribute::Unrecognized(name.to_owned(), info.into())),
        }?;
        if bytes.is_empty() {
            Ok(result)
        } else {
            Err(ParseError::malform(
                "Extra data at the end of the attribute",
            ))
        }
    }
}

#[inline]
fn parse_string(bytes: &mut &[u8], ctx: &ParsingContext) -> Result<String, ParseError> {
    let str_idx = read_value::<U16>(bytes)?.get();
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
    fn code_payload_and_nested_tables_borrow_original_input() {
        let payload = [
            0, 1, 0, 1, // max stack and locals
            0, 0, 0, 1, 0xb1, // one return instruction
            0, 1, // one exception handler
            0, 0, 0, 1, 0, 0, 0, 0, // handler entry
            0, 1, // one nested attribute
            0, 2, 0, 0, 0, 2, 0xaa, 0xbb,
        ];
        let mut bytes = vec![0xff, 0, 1];
        bytes.extend(u32::try_from(payload.len()).unwrap().to_be_bytes());
        bytes.extend(payload);
        let mut remaining = &bytes[1..];
        let attribute = AttributeInfo::from_bytes(&mut remaining).unwrap();
        assert_eq!(remaining, []);
        let original_payload = &bytes[7..];
        assert!(std::ptr::eq(
            attribute.info.as_ptr(),
            original_payload.as_ptr()
        ));

        let mut remaining = attribute.info;
        let code = super::super::raw_attributes::Code::from_bytes(&mut remaining).unwrap();
        assert_eq!(remaining, []);
        assert_eq!(code.code, [0xb1]);
        assert!(std::ptr::eq(
            code.code.as_ptr(),
            original_payload[8..].as_ptr()
        ));
        assert!(std::ptr::eq(
            code.exception_table.as_ptr().cast::<u8>(),
            original_payload[11..].as_ptr()
        ));
        assert_eq!(code.exception_table[0].end_pc.get(), 1);
        assert_eq!(code.attributes[0].info, [0xaa, 0xbb]);
        assert!(std::ptr::eq(
            code.attributes[0].info.as_ptr(),
            original_payload[27..].as_ptr()
        ));
    }

    #[test]
    fn retained_payloads_own_their_bytes_after_conversion() {
        for name in ["SourceDebugExtension", "VendorData"] {
            let mut raw_pool = RawConstantPool::new();
            let index = raw_pool.put_entry(RawEntry::Utf8(name.as_bytes())).unwrap();
            let ctx = ParsingContext {
                constant_pool: crate::jvm::class::ConstantPool::from_raw(raw_pool).unwrap(),
                class_version: Version::LATEST,
                current_class_name: "Test".parse().unwrap(),
            };
            let payload = vec![0xaa, 0xbb];
            let raw = AttributeInfo {
                attribute_name_index: index,
                info: &payload,
            };
            let attribute = Attribute::from_raw(raw, &ctx).unwrap();
            drop(payload);
            match attribute {
                Attribute::SourceDebugExtension(bytes) => assert_eq!(&*bytes, [0xaa, 0xbb]),
                Attribute::Unrecognized(name, bytes) => {
                    assert_eq!(name, "VendorData");
                    assert_eq!(&*bytes, [0xaa, 0xbb]);
                }
                _ => panic!("expected a retained byte payload"),
            }
        }
    }

    #[test]
    fn rejects_trailing_bytes_in_known_attribute() {
        let mut raw_pool = RawConstantPool::new();
        let attribute_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"Synthetic".as_slice()))
            .unwrap();
        let ctx = ParsingContext {
            constant_pool: crate::jvm::class::ConstantPool::from_raw(raw_pool).unwrap(),
            class_version: Version::LATEST,
            current_class_name: "Test".parse().unwrap(),
        };
        let attribute_info = AttributeInfo {
            attribute_name_index,
            info: &[0],
        };

        let error = Attribute::from_raw(attribute_info, &ctx).unwrap_err();

        assert_eq!(
            error.kind(),
            crate::jvm::bytecode::ParseErrorKind::Malformed
        );
    }

    #[test]
    fn parses_module_packages_with_package_constant_pool_entries() {
        let mut raw_pool = RawConstantPool::new();
        let attribute_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"ModulePackages".as_slice()))
            .unwrap();
        let package_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"example/api".as_slice()))
            .unwrap();
        raw_pool
            .put_entry(RawEntry::Package {
                name_index: package_name_index.into(),
            })
            .unwrap();
        let other_name_index = raw_pool
            .put_entry(RawEntry::Utf8(b"example/internal".as_slice()))
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
        let attribute_info = AttributeInfo {
            attribute_name_index,
            info: &[0, 2, 0, 3, 0, 5],
        };

        let attribute = Attribute::from_raw(attribute_info, &ctx).unwrap();

        let Attribute::ModulePackages(packages) = attribute else {
            panic!("expected ModulePackages attribute");
        };
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].as_str(), "example/api");
        assert_eq!(packages[1].as_str(), "example/internal");
    }
}
