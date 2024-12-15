#![cfg(integration_test)]

use std::io::{self};

use mokapot::{
    jvm::{
        class::{self, AccessFlags, RecordComponent},
        parsing::Error,
        references::ClassRef,
        Class,
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::{MethodDescriptor, ReturnType},
    },
};

#[macro_export]
macro_rules! test_data_class {
    ($folder:literal, $class_name:literal) => {
        include_bytes!(concat!(
            env!("OUT_DIR"),
            "/",
            $folder,
            "/java_classes/",
            $class_name,
            ".class"
        ))
        .as_slice()
    };
}

#[test]
fn test_parse_my_class() {
    let bytes = test_data_class!("mokapot", "org/mokapot/test/MyClass");
    let my_class = Class::from_reader(bytes).expect("Failed to parse class");

    assert_eq!(class::MAX_MAJOR_VERSION, my_class.version.major());
    assert_eq!(0, my_class.version.minor());
    assert!(!my_class.version.is_preview_enabled());
    assert_eq!(
        AccessFlags::PUBLIC | AccessFlags::SUPER,
        my_class.access_flags
    );
    assert_eq!("org/mokapot/test/MyClass", my_class.binary_name);
    assert_eq!(
        Some(ClassRef::new("java/lang/Object")),
        my_class.super_class
    );
    assert_eq!(
        Some(&ClassRef::new("java/io/Closeable")),
        my_class.interfaces.first()
    );
    assert_eq!(2, my_class.fields.len());
    assert!(my_class
        .get_field("test", FieldType::Base(PrimitiveType::Long))
        .is_some());

    let main_method = &my_class
        .get_method(
            "main",
            "([Ljava/lang/String;)V"
                .parse::<MethodDescriptor>()
                .unwrap(),
        )
        .expect("Cannot find main method");
    assert_eq!(ReturnType::Void, main_method.descriptor.return_type);
    assert_eq!(
        FieldType::Object(ClassRef::new("java/lang/String")).into_array_type(),
        main_method.descriptor.parameters_types[0]
    );
}

#[test]
fn parse_anno() {
    for bytes in [
        test_data_class!("mokapot", "org/mokapot/test/Anno"),
        test_data_class!("mokapot", "org/mokapot/test/Anno$Middle"),
    ] {
        Class::from_reader(bytes).unwrap();
    }
}

#[test]
fn parse_complicated_class() {
    for bytes in [
        test_data_class!("mokapot", "org/mokapot/test/ComplicatedClass"),
        test_data_class!("mokapot", "org/mokapot/test/ComplicatedClass$InnerClass"),
        test_data_class!("mokapot", "org/mokapot/test/ComplicatedClass$1Test"),
    ] {
        Class::from_reader(bytes).unwrap();
    }
}

#[test]
fn parse_module_info() {
    let bytes = test_data_class!("mokapot", "module-info");
    let class = Class::from_reader(bytes).expect("Fail to parse module-info");
    assert_eq!("module-info", class.binary_name);
    let module = class.module.expect("The class is a module-info");
    assert_eq!(1, module.exports.len());
    assert_eq!(1, module.opens.len());
    assert_eq!(1, module.requires.len());
}

#[test]
fn parse_record() {
    let bytes = test_data_class!("mokapot", "org/mokapot/test/RecordTest");
    let class = Class::from_reader(bytes).unwrap();
    assert_eq!("org/mokapot/test/RecordTest", class.binary_name);
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
        Some(RecordComponent { name, component_type: FieldType::Object(ClassRef { binary_name }), .. })
        if name == "description" && binary_name == "java/lang/String"
    ));
    assert!(rec_iter.next().is_none());
}

#[test]
fn not_a_class_file() {
    let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(matches!(
        Class::from_reader(bytes.as_slice()),
        Err(Error::IO(e)) if e.kind() == io::ErrorKind::InvalidData
    ));
}
