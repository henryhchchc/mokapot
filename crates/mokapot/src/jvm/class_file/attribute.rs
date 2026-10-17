use zerocopy::{
    FromBytes, Immutable, KnownLayout, Unaligned,
    byteorder::big_endian::{U16, U32},
};

mod extract;
pub(super) use extract::extract_attributes;

use crate::types::class_name::ClassName;
use crate::types::package_name::PackageName;

use super::{
    Decode, ParseError, ResolutionContext,
    annotation::raw as annotation_raw,
    class::raw as class_raw,
    code::raw as code_raw,
    code::{LocalVariableDescriptorEntry, LocalVariableSignatureEntry},
    decoder::{borrow_slice, decode_value, parse_repeated},
    method::MethodParameterInfo,
    module::raw as module_raw,
    resolve::{Resolve, decode_resolved, resolve_table},
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
        let (attribute, remaining) =
            AttributeInfo::ref_from_prefix_with_elems(bytes, attribute_length)
                .map_err(|_| ParseError::malform("Truncated class-file attribute"))?;
        *bytes = remaining;
        Ok(attribute)
    }
}

#[derive(Debug)]
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
    LocalVariableTable(Vec<LocalVariableDescriptorEntry>),
    LocalVariableTypeTable(Vec<LocalVariableSignatureEntry>),
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

impl Resolve for &AttributeInfo {
    type Output = Attribute;

    #[expect(
        clippy::too_many_lines,
        reason = "Dispatches JVM attributes with explicit wire counts"
    )]
    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let name = ctx.constant_pool.get_str(self.attribute_name_index.get())?;
        let mut info = &self.info;
        let bytes = &mut info;

        let result = match name {
            "ConstantValue" => {
                let index = decode_value(bytes)?;
                ctx.constant_pool
                    .get_constant_value(index)
                    .map(Attribute::ConstantValue)
            }
            "Code" => decode_resolved::<code_raw::Code>(bytes, ctx).map(Attribute::Code),
            "StackMapTable" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<code_raw::StackMapFrameInfo>(bytes, ctx)
                })
                .map(Attribute::StackMapTable)
            }
            "Exceptions" => {
                let count: u16 = decode_value(bytes)?;
                resolve_indices(bytes, usize::from(count), |index| {
                    ctx.constant_pool.get_class_name(index)
                })
                .map(Attribute::Exceptions)
            }
            "InnerClasses" => {
                let count: u16 = decode_value(bytes)?;
                resolve_table::<class_raw::InnerClass>(bytes, usize::from(count), ctx)
                    .map(Attribute::InnerClasses)
            }
            "EnclosingMethod" => decode_resolved::<class_raw::EnclosingMethod>(bytes, ctx)
                .map(Attribute::EnclosingMethod),
            "Synthetic" => Ok(Attribute::Synthetic),
            "Deprecated" => Ok(Attribute::Deprecated),
            "Signature" => {
                let index = decode_value(bytes)?;
                ctx.constant_pool
                    .get_str(index)
                    .map(str::to_owned)
                    .map(Attribute::Signature)
            }
            "SourceFile" => {
                let index = decode_value(bytes)?;
                ctx.constant_pool
                    .get_str(index)
                    .map(str::to_owned)
                    .map(Attribute::SourceFile)
            }
            "SourceDebugExtension" => return Ok(Attribute::SourceDebugExtension(info.into())),
            "LineNumberTable" => {
                let count: u16 = decode_value(bytes)?;
                Ok(Attribute::LineNumberTable(
                    borrow_slice::<code_raw::LineNumberTableEntry>(bytes, usize::from(count))?
                        .iter()
                        .copied()
                        .map(Into::into)
                        .collect(),
                ))
            }
            "LocalVariableTable" => {
                let count: u16 = decode_value(bytes)?;
                resolve_table::<code_raw::LocalVariableInfo>(bytes, usize::from(count), ctx)
                    .map(Attribute::LocalVariableTable)
            }
            "LocalVariableTypeTable" => {
                let count: u16 = decode_value(bytes)?;
                resolve_table::<code_raw::LocalVariableTypeInfo>(bytes, usize::from(count), ctx)
                    .map(Attribute::LocalVariableTypeTable)
            }
            "RuntimeVisibleAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeVisibleAnnotations)
            }
            "RuntimeInvisibleAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeInvisibleAnnotations)
            }
            "RuntimeVisibleParameterAnnotations" => {
                let count: u8 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    let count: u16 = decode_value(bytes)?;
                    parse_repeated(bytes, count, |bytes| {
                        decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                    })
                })
                .map(Attribute::RuntimeVisibleParameterAnnotations)
            }
            "RuntimeInvisibleParameterAnnotations" => {
                let count: u8 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    let count: u16 = decode_value(bytes)?;
                    parse_repeated(bytes, count, |bytes| {
                        decode_resolved::<annotation_raw::Annotation>(bytes, ctx)
                    })
                })
                .map(Attribute::RuntimeInvisibleParameterAnnotations)
            }
            "RuntimeVisibleTypeAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::TypeAnnotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeVisibleTypeAnnotations)
            }
            "RuntimeInvisibleTypeAnnotations" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<annotation_raw::TypeAnnotation>(bytes, ctx)
                })
                .map(Attribute::RuntimeInvisibleTypeAnnotations)
            }
            "AnnotationDefault" => decode_resolved::<annotation_raw::ElementValueInfo>(bytes, ctx)
                .map(Attribute::AnnotationDefault),
            "BootstrapMethods" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<&class_raw::BootstrapMethod>(bytes, ctx)
                })
                .map(Attribute::BootstrapMethods)
            }
            "MethodParameters" => {
                let count: u8 = decode_value(bytes)?;
                resolve_table::<MethodParameterInfo>(bytes, usize::from(count), ctx)
                    .map(Attribute::MethodParameters)
            }
            "Module" => {
                decode_resolved::<module_raw::ModuleInfo>(bytes, ctx).map(Attribute::Module)
            }
            "ModulePackages" => {
                let count: u16 = decode_value(bytes)?;
                resolve_indices(bytes, usize::from(count), |index| {
                    ctx.constant_pool.get_package_name(index)
                })
                .map(Attribute::ModulePackages)
            }
            "ModuleMainClass" => {
                let index = decode_value(bytes)?;
                ctx.constant_pool
                    .get_class_name(index)
                    .map(Attribute::ModuleMainClass)
            }
            "NestHost" => {
                let index = decode_value(bytes)?;
                ctx.constant_pool
                    .get_class_name(index)
                    .map(Attribute::NestHost)
            }
            "NestMembers" => {
                let count: u16 = decode_value(bytes)?;
                resolve_indices(bytes, usize::from(count), |index| {
                    ctx.constant_pool.get_class_name(index)
                })
                .map(Attribute::NestMembers)
            }
            "Record" => {
                let count: u16 = decode_value(bytes)?;
                parse_repeated(bytes, count, |bytes| {
                    decode_resolved::<class_raw::RecordComponentInfo>(bytes, ctx)
                })
                .map(Attribute::Record)
            }
            "PermittedSubclasses" => {
                let count: u16 = decode_value(bytes)?;
                resolve_indices(bytes, usize::from(count), |index| {
                    ctx.constant_pool.get_class_name(index)
                })
                .map(Attribute::PermittedSubclasses)
            }
            name => return Ok(Attribute::Unrecognized(name.to_owned(), info.into())),
        }?;
        if !bytes.is_empty() {
            return Err(ParseError::malform(
                "Extra data at the end of the attribute",
            ));
        }
        Ok(result)
    }
}

fn resolve_indices<T>(
    bytes: &mut &[u8],
    count: usize,
    mut resolve: impl FnMut(u16) -> Result<T, ParseError>,
) -> Result<Vec<T>, ParseError> {
    use zerocopy::byteorder::big_endian::U16;
    borrow_slice::<U16>(bytes, count)?
        .iter()
        .map(|index| resolve(index.get()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::{
        class::Version,
        class_file::constant_pool::{RawConstantPool, RawEntry},
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
        assert_eq!(&empty.info, []);
        assert_eq!(remaining, []);
    }

    #[test]
    fn retains_opaque_attribute_payloads() {
        for name in ["SourceDebugExtension", "VendorData"] {
            let mut raw_pool = RawConstantPool::new();
            let index = raw_pool.put_entry(RawEntry::Utf8(name.as_bytes())).unwrap();
            let ctx = ResolutionContext {
                constant_pool: crate::jvm::class_file::constant_pool::ConstantPool::from_raw(
                    raw_pool,
                )
                .unwrap(),
                class_version: Version::LATEST,
                current_class_name: "Test".parse().unwrap(),
            };
            let payload = [0xaa, 0xbb];
            let bytes = attribute_bytes(index, &payload);
            let raw = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();
            let attribute = raw.resolve(&ctx).unwrap();
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
        let ctx = ResolutionContext {
            constant_pool: crate::jvm::class_file::constant_pool::ConstantPool::from_raw(raw_pool)
                .unwrap(),
            class_version: Version::LATEST,
            current_class_name: "Test".parse().unwrap(),
        };
        let bytes = attribute_bytes(attribute_name_index, &[0]);
        let attribute_info = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();

        let error = attribute_info.resolve(&ctx).unwrap_err();

        assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
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
        let constant_pool =
            crate::jvm::class_file::constant_pool::ConstantPool::from_raw(raw_pool).unwrap();
        let ctx = ResolutionContext {
            constant_pool,
            class_version: Version::LATEST,
            current_class_name: "module-info".parse().unwrap(),
        };
        let bytes = attribute_bytes(attribute_name_index, &[0, 2, 0, 3, 0, 5]);
        let attribute_info = <&AttributeInfo>::decode(&mut bytes.as_slice()).unwrap();

        let attribute = attribute_info.resolve(&ctx).unwrap();

        let Attribute::ModulePackages(packages) = attribute else {
            panic!("expected ModulePackages attribute");
        };
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].as_str(), "example/api");
        assert_eq!(packages[1].as_str(), "example/internal");
    }

    #[test]
    fn miri_method_parameters_resolve_borrowed_records() {
        use crate::jvm::{errors::ParseErrorKind, method::ParameterAccessFlags};
        let mut pool = RawConstantPool::new();
        let attribute_name_index = pool.put_entry(RawEntry::Utf8(b"MethodParameters")).unwrap();
        pool.put_entry(RawEntry::Utf8(b"value")).unwrap();
        let ctx = ResolutionContext {
            constant_pool: super::super::constant_pool::ConstantPool::from_raw(pool).unwrap(),
            class_version: Version::LATEST,
            current_class_name: "Test".parse().unwrap(),
        };
        let bytes = [0xff, 1, 0, 2, 0, 0x10];
        let resolve = |info| {
            let bytes = attribute_bytes(attribute_name_index, info);
            <&AttributeInfo>::decode(&mut bytes.as_slice())?.resolve(&ctx)
        };
        let Attribute::MethodParameters(parameters) = resolve(&bytes[1..]).unwrap() else {
            panic!("expected method parameters");
        };
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name.as_deref(), Some("value"));
        assert_eq!(parameters[0].access_flags, ParameterAccessFlags::FINAL);
        assert!(
            matches!(resolve(&[0]).unwrap(), Attribute::MethodParameters(parameters) if parameters.is_empty())
        );
        assert_eq!(
            resolve(&bytes[1..bytes.len() - 1]).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }
}
