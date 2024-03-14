use std::io::{self};

use mokapot::{
    jvm::{
        class::{AccessFlags, Class},
        parsing::Error,
        references::ClassRef,
    },
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::ReturnType,
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

fn parse_my_class() -> Result<Class, Error> {
    let bytes = test_data_class!("", "org/mokapot/test/MyClass");
    Class::parse(bytes)
}

/// Parse classes in OpenJDK test data
/// Stolen from https://github.com/openjdk/jdk/tree/master/test/jdk/jdk/classfile/testdata
#[test]
fn parse_openjdk_test_data() {
    let test_data = [
        test_data_class!("openjdk", "testdata/Pattern1"),
        test_data_class!("openjdk", "testdata/Pattern2"),
        test_data_class!("openjdk", "testdata/Pattern3"),
        test_data_class!("openjdk", "testdata/Pattern4"),
        test_data_class!("openjdk", "testdata/Pattern5"),
        test_data_class!("openjdk", "testdata/Pattern6"),
        test_data_class!("openjdk", "testdata/Pattern7"),
        test_data_class!("openjdk", "testdata/Pattern8"),
        test_data_class!("openjdk", "testdata/Pattern9"),
        test_data_class!("openjdk", "testdata/Pattern10"),
        test_data_class!("openjdk", "testdata/Lvt"),
        test_data_class!("openjdk", "testdata/TypeAnnotationPattern"),
        test_data_class!("openjdk", "testdata/TypeAnnotationPattern$Foo"),
        test_data_class!("openjdk", "testdata/TypeAnnotationPattern$Bar"),
        test_data_class!("openjdk", "testdata/TypeAnnotationPattern$Middle"),
        test_data_class!("openjdk", "testdata/TypeAnnotationPattern$Middle$Inner"),
    ];
    for bytes in test_data {
        assert!(Class::parse(bytes).is_ok())
    }
}

#[test]
fn test_parse_file() {
    assert!(parse_my_class().is_ok());
}

#[test]
fn test_parse_version() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(65, my_class.version.major());
    assert_eq!(0, my_class.version.minor());
    assert!(!my_class.version.is_preview_enabled());
}

#[test]
fn test_access_flag() {
    let my_class = parse_my_class().unwrap();
    let expected = AccessFlags::PUBLIC | AccessFlags::SUPER;
    assert_eq!(expected, my_class.access_flags);
}

#[test]
fn test_class_name() {
    let my_class = parse_my_class().unwrap();
    assert_eq!("org/mokapot/test/MyClass", my_class.binary_name);
}

#[test]
fn test_super_class_name() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(
        Some(ClassRef::new("java/lang/Object")),
        my_class.super_class
    );
}

#[test]
fn test_interfaces() {
    let my_class = parse_my_class().unwrap();
    let mut interfaces = my_class.interfaces.into_iter();
    assert_eq!(
        Some(ClassRef::new("java/lang/Cloneable")),
        interfaces.next()
    );
}

#[test]
fn test_fields() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(2, my_class.fields.len());
    let test_field = &my_class.fields.iter().find(|f| f.name == "test").unwrap();
    assert_eq!(FieldType::Base(PrimitiveType::Long), test_field.field_type);
}

#[test]
fn test_methods() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(4, my_class.methods.len());
    let main_method = &my_class
        .methods
        .iter()
        .find(|f| f.name == "main")
        .expect("main method not found");
    assert_eq!(ReturnType::Void, main_method.descriptor.return_type);
    assert_eq!(
        FieldType::Object(ClassRef::new("java/lang/String")).into_array_type(),
        main_method.descriptor.parameters_types[0]
    );
}

#[test]
fn parse_complicated_class() {
    for bytes in [
        test_data_class!("", "org/mokapot/test/ComplicatedClass"),
        test_data_class!("", "org/mokapot/test/ComplicatedClass$InnerClass"),
        test_data_class!("", "org/mokapot/test/ComplicatedClass$1Test"),
    ] {
        Class::parse(bytes).unwrap();
    }
}

#[test]
fn parse_module_info() {
    let bytes = test_data_class!("", "module-info");
    let class = Class::parse(bytes).unwrap();
    assert_eq!("module-info", class.binary_name);
    let module = class.module.expect("The class is a module-info");
    assert_eq!(1, module.exports.len());
    assert_eq!(1, module.opens.len());
    assert_eq!(1, module.requires.len());
}

#[test]
fn not_a_class_file() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/MyClass.java"
    ));
    assert!(matches!(
        Class::parse(bytes.as_slice()),
        Err(Error::IO(e)) if e.kind() == io::ErrorKind::InvalidData
    ));
}
