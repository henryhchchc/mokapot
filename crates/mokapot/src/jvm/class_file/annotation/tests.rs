use super::*;
use crate::jvm::{
    JavaString,
    class::Version,
    class_file::constant_pool::{ConstantPool, RawConstantPool, RawEntry},
    errors::ParseErrorKind,
};

fn context() -> ResolutionContext {
    let mut pool = RawConstantPool::new();
    for entry in [
        RawEntry::Utf8(b"LExample;"),
        RawEntry::Utf8(b"value"),
        RawEntry::Integer(42.into()),
        RawEntry::Utf8(b"hello"),
        RawEntry::Utf8(b"LChoice;"),
        RawEntry::Utf8(b"YES"),
        RawEntry::Utf8(b"V"),
    ] {
        pool.put_entry(entry).unwrap();
    }
    ResolutionContext {
        constant_pool: ConstantPool::from_raw(pool).unwrap(),
        class_version: Version::LATEST,
        current_class_name: "Example".parse().unwrap(),
    }
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
    let annotation = decode_annotation(&mut remaining, &ctx).unwrap();
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
                ElementValue::String(JavaString::Utf8("hello".to_owned())),
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
        decode_annotation(&mut remaining, &ctx).unwrap(),
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
    let annotation = decode_type_annotation(&mut bytes.as_slice(), &ctx).unwrap();
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
        let error = decode_element_value(&mut bytes, &ctx).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
