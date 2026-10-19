use super::*;
use crate::jvm::class_file::{Decode, resolve::decode_resolved};
use crate::jvm::{
    Class,
    annotation::{Annotation, ElementValue, TypeAnnotation},
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
        RawEntry::Utf8(b"\xed\xa0\x80"),
    ]);
    ResolutionContext {
        constant_pool: ConstantPool::from_raw(pool).unwrap(),
        class_version: Version::LATEST,
    }
}

fn class_with_pool(mut pool: ConstantPool) -> Class {
    let name = pool.insert_class_name("Example".parse().unwrap());
    Class {
        name,
        constant_pool: pool,
        ..Class::default()
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
    let mut ctx = context();
    let mut remaining = bytes.as_slice();
    let data = decode_resolved::<raw::Annotation>(&mut remaining, &mut ctx).unwrap();
    let next = decode_resolved::<raw::Annotation>(&mut remaining, &mut ctx).unwrap();
    let class = class_with_pool(ctx.constant_pool);
    let annotation = Annotation::new(&class, &data);
    assert_eq!(annotation.annotation_type(), &"LExample;".parse().unwrap());
    assert_eq!(annotation.element_value_pairs().len(), 1);
    let ElementValue::Array(values) = annotation.get_value().unwrap() else {
        panic!("expected annotation array");
    };
    assert_eq!(values.len(), 5);
    assert_eq!(
        values.get(0),
        Some(ElementValue::Primitive(
            PrimitiveType::Int,
            ConstantValueView::Integer(42)
        ))
    );
    let ElementValue::String(string) = values.get(1).unwrap() else {
        panic!("expected string");
    };
    assert_eq!(string, "hello");
    let ElementValue::EnumConstant {
        enum_type,
        const_name,
    } = values.get(2).unwrap()
    else {
        panic!("expected enum constant");
    };
    assert_eq!(enum_type, &"LChoice;".parse().unwrap());
    assert_eq!(const_name, "YES");
    let ElementValue::Class { return_descriptor } = values.get(3).unwrap() else {
        panic!("expected class literal");
    };
    assert_eq!(return_descriptor, &"V".parse().unwrap());
    let ElementValue::AnnotationInterface(nested) = values.get(4).unwrap() else {
        panic!("expected nested annotation");
    };
    assert_eq!(nested.annotation_type(), annotation.annotation_type());
    assert!(std::ptr::eq(
        nested.annotation_type(),
        annotation.annotation_type()
    ));
    let ElementValue::Array(empty) = nested.get_value().unwrap() else {
        panic!("expected nested annotation array");
    };
    assert!(empty.is_empty());
    assert_eq!(empty.get(0), None);
    assert_eq!(
        Annotation::new(&class, &next).element_value_pairs().len(),
        0
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
    let mut ctx = context();
    let data = decode_resolved::<raw::TypeAnnotation>(&mut bytes.as_slice(), &mut ctx).unwrap();
    let class = class_with_pool(ctx.constant_pool);
    let annotation = TypeAnnotation::new(&class, &data);
    assert_eq!(annotation.annotation_type(), &"LExample;".parse().unwrap());
    assert_eq!(
        annotation.target_info(),
        &TargetInfo::LocalVar(
            VariableKind::Local,
            Box::new([LocalVariableId {
                range: 2.into()..5.into(),
                index: 4,
            }]),
        )
    );
    assert_eq!(annotation.target_path(), &[TypePathElement::Array]);
    let (name, value) = annotation.element_value_pairs().next().unwrap();
    assert_eq!(name, "value");
    assert_eq!(
        value,
        ElementValue::Primitive(PrimitiveType::Int, ConstantValueView::Integer(42))
    );
}

#[test]
fn preserves_unpaired_surrogates_in_annotation_names_and_strings() {
    let bytes = [0, 1, 0, 2, 0, 8, b's', 0, 8, 0, 2, b'e', 0, 5, 0, 8];
    let mut ctx = context();
    let data = decode_resolved::<raw::Annotation>(&mut bytes.as_slice(), &mut ctx).unwrap();
    let class = class_with_pool(ctx.constant_pool);
    let annotation = Annotation::new(&class, &data);
    let (name, ElementValue::String(string)) = annotation.element_value_pairs().next().unwrap()
    else {
        panic!("expected string element");
    };
    assert_eq!(name.as_bytes(), b"\xed\xa0\x80");
    assert!(std::ptr::eq(name, string));
    let ElementValue::EnumConstant { const_name, .. } = annotation.get_value().unwrap() else {
        panic!("expected enum constant");
    };
    assert!(std::ptr::eq(name, const_name));
    assert_eq!(
        annotation.get_element_value(name),
        Some(ElementValue::String(string))
    );
}

#[test]
fn annotation_equality_compares_values_across_pools() {
    let mut first = context();
    let mut second = context();
    let name = second.constant_pool.insert_utf8("value".into());
    let bytes = [0, 1, 0, 1, 0, 2, b'[', 0, 2, b's', 0, 4, b'I', 0, 3];
    let first_data = decode_resolved::<raw::Annotation>(&mut bytes.as_slice(), &mut first).unwrap();
    let mut second_data =
        decode_resolved::<raw::Annotation>(&mut bytes.as_slice(), &mut second).unwrap();
    second_data.element_value_pairs[0].0 = name;
    assert_ne!(first_data, second_data);
    let first_class = class_with_pool(first.constant_pool);
    let second_class = class_with_pool(second.constant_pool);
    assert_eq!(
        Annotation::new(&first_class, &first_data),
        Annotation::new(&second_class, &second_data)
    );
}

#[test]
fn rejects_unknown_value_tags_and_mismatched_primitive_constants() {
    let mut ctx = context();
    for mut bytes in [b"?".as_slice(), &[b'D', 0, 3]] {
        let error = decode_resolved::<raw::ElementValueInfo>(&mut bytes, &mut ctx).unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
