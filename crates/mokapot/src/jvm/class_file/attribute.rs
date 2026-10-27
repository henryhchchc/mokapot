use zerocopy::{
    FromBytes, Immutable, KnownLayout, Unaligned,
    byteorder::big_endian::{U16, U32},
};

mod extract;
pub(super) mod raw;
mod resolve;
pub(super) use extract::extract_attributes;

use super::{
    Decode, ParseError, ResolutionContext,
    annotation::raw as annotation_raw,
    class::raw as class_raw,
    code::raw as code_raw,
    code::{LocalVariableDescriptorEntry, LocalVariableSignatureEntry},
    decoder::{borrow_view, decode_value, parse_repeated},
    module::raw as module_raw,
    resolve::{Resolve, decode_resolved},
};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        annotation::{AnnotationData, ElementValueData, TypeAnnotationData},
        attribute::AttributeData,
        class::{
            bootstrap::BootstrapMethodData,
            nesting::{EnclosingMethodData, InnerClassData},
            record_component::RecordComponentData,
        },
        code::{LineNumberTableEntry, MethodBodyData, StackMapFrameData},
        constant_pool::{ClassId, ConstantValueId, PackageId, Utf8Id},
        method::ParameterInfoData,
        module::ModuleData,
    },
};

/// Represent an attribute of a class file, method, field, or code.
#[doc = see_jvm_spec!(4, 7)]
#[derive(Debug, FromBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub(crate) struct AttributeInfo {
    attribute_name_index: U16,
    attribute_length: U32,
    info: [u8],
}

impl<'a> Decode<'a> for &'a AttributeInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        // Probe the header to bound the borrowed view to this attribute.
        let mut header = *bytes;
        let _: u16 = decode_value(&mut header)?;
        let attribute_length: u32 = decode_value(&mut header)?;
        let attribute_length = usize::try_from(attribute_length)
            .expect("32-bit size is not supported on the current platform");
        borrow_view::<AttributeInfo>(bytes, attribute_length)
    }
}

#[derive(Debug)]
pub(crate) enum Attribute {
    ConstantValue(ConstantValueId),
    Code(MethodBodyData),
    StackMapTable(Box<[StackMapFrameData]>),
    Exceptions(Box<[ClassId]>),
    SourceFile(Utf8Id),
    LineNumberTable(Box<[LineNumberTableEntry]>),
    InnerClasses(Box<[InnerClassData]>),
    Synthetic,
    Deprecated,
    EnclosingMethod(EnclosingMethodData),
    Signature(Utf8Id),
    SourceDebugExtension(Box<[u8]>),
    LocalVariableTable(Box<[LocalVariableDescriptorEntry]>),
    LocalVariableTypeTable(Box<[LocalVariableSignatureEntry]>),
    RuntimeVisibleAnnotations(Box<[AnnotationData]>),
    RuntimeInvisibleAnnotations(Box<[AnnotationData]>),
    RuntimeVisibleParameterAnnotations(Box<[Box<[AnnotationData]>]>),
    RuntimeInvisibleParameterAnnotations(Box<[Box<[AnnotationData]>]>),
    RuntimeVisibleTypeAnnotations(Box<[TypeAnnotationData]>),
    RuntimeInvisibleTypeAnnotations(Box<[TypeAnnotationData]>),
    AnnotationDefault(ElementValueData),
    BootstrapMethods(Box<[BootstrapMethodData]>),
    MethodParameters(Box<[ParameterInfoData]>),
    Module(ModuleData),
    ModulePackages(Box<[PackageId]>),
    ModuleMainClass(ClassId),
    NestHost(ClassId),
    NestMembers(Box<[ClassId]>),
    Record(Box<[RecordComponentData]>),
    PermittedSubclasses(Box<[ClassId]>),
    Unrecognized(AttributeData),
}

impl Attribute {
    pub const fn name(&self) -> &str {
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
            Self::Unrecognized(_) => "unrecognized",
        }
    }
}

impl Resolve for &AttributeInfo {
    type Output = Attribute;

    #[expect(
        clippy::too_many_lines,
        reason = "Dispatches JVM attributes with explicit wire counts"
    )]
    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let name_id = ctx.constant_pool.utf8_id(self.attribute_name_index.get())?;
        let mut info = &self.info;
        let bytes = &mut info;

        let result = match ctx.constant_pool.utf8(name_id).as_bytes() {
            b"ConstantValue" => {
                decode_resolved::<raw::ConstantValue>(bytes, ctx).map(Attribute::ConstantValue)
            }
            b"Code" => decode_resolved::<code_raw::Code>(bytes, ctx).map(Attribute::Code),
            b"StackMapTable" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<code_raw::StackMapFrameInfo>(bytes, ctx)
                })
                .map(Attribute::StackMapTable)
            }
            b"Exceptions" => {
                decode_resolved::<&raw::Exceptions>(bytes, ctx).map(Attribute::Exceptions)
            }
            b"InnerClasses" => {
                decode_resolved::<&raw::InnerClasses>(bytes, ctx).map(Attribute::InnerClasses)
            }
            b"EnclosingMethod" => decode_resolved::<class_raw::EnclosingMethod>(bytes, ctx)
                .map(Attribute::EnclosingMethod),
            b"Synthetic" => Ok(Attribute::Synthetic),
            b"Deprecated" => Ok(Attribute::Deprecated),
            b"Signature" => decode_resolved::<raw::Signature>(bytes, ctx).map(Attribute::Signature),
            b"SourceFile" => {
                decode_resolved::<raw::SourceFile>(bytes, ctx).map(Attribute::SourceFile)
            }
            b"SourceDebugExtension" => {
                return Ok(Attribute::SourceDebugExtension(info.into()));
            }
            b"LineNumberTable" => {
                decode_resolved::<&raw::LineNumberTable>(bytes, ctx).map(Attribute::LineNumberTable)
            }
            b"LocalVariableTable" => decode_resolved::<&raw::LocalVariableTable>(bytes, ctx)
                .map(Attribute::LocalVariableTable),
            b"LocalVariableTypeTable" => {
                decode_resolved::<&raw::LocalVariableTypeTable>(bytes, ctx)
                    .map(Attribute::LocalVariableTypeTable)
            }
            b"RuntimeVisibleAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeVisibleAnnotations)
            }
            b"RuntimeInvisibleAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeInvisibleAnnotations)
            }
            b"RuntimeVisibleParameterAnnotations" => {
                let count: u8 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    let count: u16 = decode_value(bytes)?;
                    parse_repeated(bytes, count, |bytes| {
                        decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                    })
                })
                .map(Attribute::RuntimeVisibleParameterAnnotations)
            }
            b"RuntimeInvisibleParameterAnnotations" => {
                let count: u8 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    let count: u16 = decode_value(bytes)?;
                    parse_repeated(bytes, count, |bytes| {
                        decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                    })
                })
                .map(Attribute::RuntimeInvisibleParameterAnnotations)
            }
            b"RuntimeVisibleTypeAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::TypeAnnotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeVisibleTypeAnnotations)
            }
            b"RuntimeInvisibleTypeAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::TypeAnnotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeInvisibleTypeAnnotations)
            }
            b"AnnotationDefault" => decode_resolved::<annotation_raw::ElementValueInfo>(bytes, ctx)
                .map(Attribute::AnnotationDefault),
            b"BootstrapMethods" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<&class_raw::BootstrapMethod>(bytes, ctx)
                })
                .map(Attribute::BootstrapMethods)
            }
            b"MethodParameters" => decode_resolved::<&raw::MethodParameters>(bytes, ctx)
                .map(Attribute::MethodParameters),
            b"Module" => {
                decode_resolved::<module_raw::ModuleInfo>(bytes, ctx).map(Attribute::Module)
            }
            b"ModulePackages" => {
                decode_resolved::<&raw::ModulePackages>(bytes, ctx).map(Attribute::ModulePackages)
            }
            b"ModuleMainClass" => {
                decode_resolved::<raw::ModuleMainClass>(bytes, ctx).map(Attribute::ModuleMainClass)
            }
            b"NestHost" => decode_resolved::<raw::NestHost>(bytes, ctx).map(Attribute::NestHost),
            b"NestMembers" => {
                decode_resolved::<&raw::NestMembers>(bytes, ctx).map(Attribute::NestMembers)
            }
            b"Record" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<class_raw::RecordComponentInfo>(bytes, ctx)
                })
                .map(Attribute::Record)
            }
            b"PermittedSubclasses" => decode_resolved::<&raw::PermittedSubclasses>(bytes, ctx)
                .map(Attribute::PermittedSubclasses),
            _ => {
                return Ok(Attribute::Unrecognized(AttributeData {
                    name: name_id,
                    bytes: self.info.into(),
                }));
            }
        }?;
        if !bytes.is_empty() {
            return Err(ParseError::malform(
                "Extra data at the end of the attribute",
            ));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{
        class::Version,
        class_file::constant_pool::{RawConstantPool, RawEntry, resolve},
        errors::ParseErrorKind,
    };

    fn attribute_bytes(name_index: u16, info: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(name_index.to_be_bytes());
        bytes.extend(u32::try_from(info.len()).unwrap().to_be_bytes());
        bytes.extend(info);
        bytes
    }

    #[test]
    fn decodes_consecutive_attributes_using_declared_lengths() {
        let bytes = [0x12, 0x34, 0, 0, 0, 2, 0xaa, 0xbb, 0, 1, 0, 0, 0, 0];
        let mut remaining = bytes.as_slice();
        let attribute = <&AttributeInfo>::decode(&mut remaining).unwrap();
        assert_eq!(attribute.attribute_name_index.get(), 0x1234);
        assert_eq!(&attribute.info, [0xaa, 0xbb]);

        let empty = <&AttributeInfo>::decode(&mut remaining).unwrap();
        assert_eq!(empty.attribute_name_index.get(), 1);
        assert_eq!(&empty.info, b"");
        assert_eq!(remaining, b"");
    }

    #[test]
    fn retains_opaque_attribute_payloads() {
        for name in ["SourceDebugExtension", "VendorData"] {
            let raw_pool = RawConstantPool::from_entries([RawEntry::Utf8(name.as_bytes())]);
            let index = 1;
            let mut ctx = ResolutionContext {
                constant_pool: resolve(raw_pool).unwrap(),
                class_version: Version::LATEST,
            };
            let payload = [0xaa, 0xbb];
            let bytes = attribute_bytes(index, &payload);
            let raw = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();
            let attribute = raw.resolve(&mut ctx).unwrap();
            match attribute {
                Attribute::SourceDebugExtension(bytes) => assert_eq!(&*bytes, [0xaa, 0xbb]),
                Attribute::Unrecognized(data) => {
                    assert_eq!(ctx.constant_pool.utf8(data.name), "VendorData");
                    assert_eq!(&*data.bytes, [0xaa, 0xbb]);
                }
                _ => panic!("expected a retained byte payload"),
            }
        }
    }

    #[test]
    fn rejects_trailing_bytes_in_known_attributes() {
        for name in [
            "Synthetic",
            "Exceptions",
            "InnerClasses",
            "LineNumberTable",
            "LocalVariableTable",
            "LocalVariableTypeTable",
            "MethodParameters",
            "ModulePackages",
            "NestMembers",
            "PermittedSubclasses",
        ] {
            let pool = RawConstantPool::from_entries([RawEntry::Utf8(name.as_bytes())]);
            let index = 1;
            let mut ctx = ResolutionContext {
                constant_pool: resolve(pool).unwrap(),
                class_version: Version::LATEST,
            };
            let count_width = match name {
                "Synthetic" => 0,
                "MethodParameters" => 1,
                _ => 2,
            };
            let mut payload = vec![0; count_width];
            payload.push(0xff);
            let bytes = attribute_bytes(index, &payload);
            let raw = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();
            let error = raw.resolve(&mut ctx).unwrap_err();
            assert_eq!(error.kind(), ParseErrorKind::Malformed);
        }
    }

    #[test]
    fn parses_module_packages_with_package_constant_pool_entries() {
        let raw_pool = RawConstantPool::from_entries([
            RawEntry::Utf8(b"ModulePackages"),
            RawEntry::Utf8(b"example/api"),
            RawEntry::Package {
                name_index: 2.into(),
            },
            RawEntry::Utf8(b"example/internal"),
            RawEntry::Package {
                name_index: 4.into(),
            },
        ]);
        let attribute_name_index = 1;
        let constant_pool = resolve(raw_pool).unwrap();
        let mut ctx = ResolutionContext {
            constant_pool,
            class_version: Version::LATEST,
        };
        let bytes = attribute_bytes(attribute_name_index, &[0, 2, 0, 3, 0, 5]);
        let attribute_info = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();

        let attribute = attribute_info.resolve(&mut ctx).unwrap();

        let Attribute::ModulePackages(packages) = attribute else {
            panic!("expected ModulePackages attribute");
        };
        assert_eq!(packages.len(), 2);
        assert_eq!(
            ctx.constant_pool.package_name(packages[0]).as_java_str(),
            "example/api"
        );
        assert_eq!(
            ctx.constant_pool.package_name(packages[1]).as_java_str(),
            "example/internal"
        );
    }

    #[test]
    fn method_parameters_resolve_borrowed_records() {
        use crate::jvm::method::ParameterAccessFlags;
        let pool = RawConstantPool::from_entries([
            RawEntry::Utf8(b"MethodParameters"),
            RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
        ]);
        let attribute_name_index = 1;
        let mut ctx = ResolutionContext {
            constant_pool: resolve(pool).unwrap(),
            class_version: Version::LATEST,
        };
        let bytes = [0xff, 1, 0, 2, 0, 0x10];
        let mut resolve = |info| {
            let bytes = attribute_bytes(attribute_name_index, info);
            <&AttributeInfo>::decode(&mut bytes.as_slice())?.resolve(&mut ctx)
        };
        let Attribute::MethodParameters(parameters) = resolve(&bytes[1..]).unwrap() else {
            panic!("expected method parameters");
        };
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].access_flags, ParameterAccessFlags::FINAL);
        assert!(
            matches!(resolve(&[0]).unwrap(), Attribute::MethodParameters(parameters) if parameters.is_empty())
        );
        assert_eq!(
            resolve(&bytes[1..bytes.len() - 1]).unwrap_err().kind(),
            ParseErrorKind::Truncated {
                requested: 5,
                available: 4
            }
        );
        let name = ctx.constant_pool.utf8(parameters[0].name.unwrap());
        assert_eq!(name.as_bytes(), &[0xed, 0xa0, 0x80]);
    }
}
