//! Tests for parsing compiled Java fixtures.

#![cfg(java_fixture_tests)]
#![allow(missing_docs, clippy::ignore_without_reason)]

use mokapot::types::class_name::ClassName;

use mokapot::{
    jvm::{
        Class,
        class::{self, AccessFlags, RecordComponent},
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::{MethodDescriptor, ReturnType},
    },
};

mod support;

fn test_data_class(name: &str) -> Vec<u8> {
    support::class_bytes(name)
}

#[test]
fn test_parse_my_class() {
    let bytes = test_data_class("org/mokapot/test/MyClass");
    let my_class = Class::from_bytes(&bytes).expect("Failed to parse class");

    assert_eq!(class::Version::LATEST, my_class.version);
    assert_eq!(0, my_class.version.minor());
    assert!(!my_class.version.is_preview_enabled());
    assert_eq!(
        AccessFlags::PUBLIC | AccessFlags::SUPER,
        my_class.access_flags
    );
    assert_eq!("org/mokapot/test/MyClass", my_class.name);
    let object: ClassName = "java/lang/Object".parse().unwrap();
    assert_eq!(Some(object), my_class.super_class);
    let closeable: ClassName = "java/io/Closeable".parse().unwrap();
    assert_eq!(Some(&closeable), my_class.interfaces.first());
    assert_eq!(2, my_class.fields.len());
    assert!(
        my_class
            .get_field("test", FieldType::Base(PrimitiveType::Long))
            .is_some()
    );

    let main_method = my_class
        .get_method_str(
            "main",
            "([Ljava/lang/String;)V"
                .parse::<MethodDescriptor>()
                .unwrap(),
        )
        .expect("Cannot find main method");
    assert_eq!(ReturnType::Void, main_method.descriptor().return_type);
    assert_eq!(
        FieldType::from("java/lang/String".parse::<ClassName>().unwrap()).into_array_type(),
        main_method.descriptor().parameters_types[0]
    );
}

#[test]
fn test_anno() {
    let bytes = test_data_class("org/mokapot/test/Anno");
    let class = Class::from_bytes(&bytes).unwrap();
    let field_type: FieldType = "[[Ljava/lang/String;".parse().unwrap();
    let field = class.get_field("fa", field_type).unwrap();
    assert_eq!(field.annotations.runtime_visible.len(), 1);
    assert_eq!(field.type_annotations.runtime_invisible.len(), 1);

    let bytes = test_data_class("org/mokapot/test/Anno$Middle");
    let class = Class::from_bytes(&bytes).unwrap();
    assert_eq!(class.type_annotations.runtime_visible.len(), 1);
    assert!(!class.inner_classes.is_empty());
}

#[test]
fn test_complicated_class() {
    for bytes in [
        test_data_class("org/mokapot/test/ComplicatedClass"),
        test_data_class("org/mokapot/test/ComplicatedClass$InnerClass"),
        test_data_class("org/mokapot/test/ComplicatedClass$1Test"),
    ] {
        let class = Class::from_bytes(&bytes).unwrap();
        assert!(!class.inner_classes.is_empty());
    }
}

#[test]
fn parse_module_info() {
    let bytes = test_data_class("module-info");
    let class = Class::from_bytes(&bytes).expect("Fail to parse module-info");
    assert_eq!("module-info", class.name);
    let module = class.module.as_ref().expect("The class is a module-info");
    assert_eq!(1, module.exports.len());
    assert_eq!(1, module.opens.len());
    assert_eq!(1, module.requires.len());
    assert_eq!(module.exports[0].package, "org/mokapot/test");
    assert_eq!(module.opens[0].package, "org/mokapot/test");
}

#[test]
fn parse_record() {
    let bytes = test_data_class("org/mokapot/test/RecordTest");
    let class = Class::from_bytes(&bytes).unwrap();
    assert_eq!("org/mokapot/test/RecordTest", class.name);
    let Some(components) = class.record else {
        panic!("Record components not found.");
    };
    let mut rec_iter = components.into_iter();
    assert!(matches!(
        rec_iter.next(),
        Some(RecordComponent { name, component_type, .. })
        if name == "x" && component_type == PrimitiveType::Int.into()
    ));
    assert!(matches!(
        rec_iter.next(),
        Some(RecordComponent { name, component_type, .. })
        if name == "y" && component_type == PrimitiveType::Int.into()
    ));
    assert!(matches!(
        rec_iter.next(),
        Some(RecordComponent { name, component_type, .. })
        if name == "z" && component_type == PrimitiveType::Double.into()
    ));
    assert!(matches!(
        rec_iter.next(),
        Some(RecordComponent { name, component_type: FieldType::Object(class_name), .. })
        if name == "description" && class_name.as_java_str() == "java/lang/String"
    ));
    assert!(rec_iter.next().is_none());
}
