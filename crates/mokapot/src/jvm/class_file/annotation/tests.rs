use super::*;
use crate::jvm::class_file::{Decode, resolve::decode_resolved};
use crate::jvm::{
    JavaString,
    class::Version,
    class_file::constant_pool::{ConstantPool, RawConstantPool, RawEntry},
    errors::ParseErrorKind,
};

fn context() -> ResolutionContext {
    let pool = RawConstantPool::from_entries([
        RawEntry::Utf8(b"LExample;"),
        RawEntry::Utf8(b"value"),
        RawEntry::Integer(42.into()),
        RawEntry::Utf8(b"hello"),
        RawEntry::Utf8(b"LChoice;"),
        RawEntry::Utf8(b"YES"),
        RawEntry::Utf8(b"V"),
    ]);
    ResolutionContext {
        constant_pool: ConstantPool::from_raw(pool).unwrap(),
        class_version: Version::LATEST,
        current_class_name: "Example".parse().unwrap(),
    }
}

#[test]
fn raw_annotations_preserve_constant_pool_indices() {
    let bytes = [0x12, 0x34, 0, 1, 0xab, 0xcd, b'c', 0, 5];
    let annotation = raw::Annotation::decode(&mut bytes.as_slice()).unwrap();
    assert_eq!(annotation.type_index, 0x1234);
    assert!(matches!(
        annotation.element_value_pairs.as_ref(),
        [(0xabcd, raw::ElementValueInfo::ClassInfo(5))]
    ));
}

#[test]
fn decodes_nested_annotation_values_and_continues_to_the_next_annotation() {
    let bytes = [
        0, 1, 0, 1, 0, 2, // annotation with one element named value
        b'[', 0, 5, // array of five values
        b'I', 0, 3, // integer
        b's', 0, 4, // string
        b'e', 0, 5, 0, 6, // enum
        b'c', 0, 7, // void class literal
        b'@', 0, 1, 0, 1, 0, 2, b'[', 0, 0, // nested annotation with an empty array
        0, 1, 0, 0, // next annotation
    ];
    let ctx = context();
    let mut remaining = bytes.as_slice();
    let annotation = decode_resolved::<raw::Annotation>(&mut remaining, &ctx).unwrap();
    let nested = Annotation {
        annotation_type: "LExample;".parse().unwrap(),
        element_value_pairs: Box::new([("value".to_owned(), ElementValue::Array(Box::new([])))]),
    };
    let expected = Annotation {
        annotation_type: "LExample;".parse().unwrap(),
        element_value_pairs: Box::new([(
            "value".to_owned(),
            ElementValue::Array(Box::new([
                ElementValue::Primitive(PrimitiveType::Int, ConstantValue::Integer(42)),
                ElementValue::String(JavaString::from("hello")),
                ElementValue::EnumConstant {
                    enum_type: "LChoice;".parse().unwrap(),
                    const_name: "YES".to_owned(),
                },
                ElementValue::Class {
                    return_descriptor: "V".parse().unwrap(),
                },
                ElementValue::AnnotationInterface(nested),
            ])),
        )]),
    };
    assert_eq!(annotation, expected);
    assert_eq!(
        decode_resolved::<raw::Annotation>(&mut remaining, &ctx).unwrap(),
        Annotation {
            annotation_type: "LExample;".parse().unwrap(),
            element_value_pairs: Box::new([]),
        }
    );
    assert_eq!(remaining, b"");
}

#[test]
fn decodes_type_annotation_target_path_and_values() {
    let bytes = [
        0x40, 0, 1, 0, 2, 0, 3, 0, 4, // local variable target
        1, 0, 0, // array type path
        0, 1, 0, 1, 0, 2, b'I', 0, 3, // annotation and element value
    ];
    let ctx = context();
    let annotation = decode_resolved::<raw::TypeAnnotation>(&mut bytes.as_slice(), &ctx).unwrap();
    assert_eq!(
        annotation,
        TypeAnnotation {
            annotation_type: "LExample;".parse().unwrap(),
            target_info: TargetInfo::LocalVar(
                VariableKind::Local,
                Box::new([LocalVariableId {
                    range: 2.into()..5.into(),
                    index: 4,
                }]),
            ),
            target_path: Box::new([TypePathElement::Array]),
            element_value_pairs: Box::new([(
                "value".to_owned(),
                ElementValue::Primitive(PrimitiveType::Int, ConstantValue::Integer(42)),
            )]),
        }
    );
}

#[test]
fn rejects_unknown_value_tags_and_mismatched_primitive_constants() {
    let ctx = context();
    for mut bytes in [b"?".as_slice(), &[b'D', 0, 3]] {
        let error = decode_resolved::<raw::ElementValueInfo>(&mut bytes, &ctx).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
