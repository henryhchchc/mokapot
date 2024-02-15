use std::io::BufReader;

use mokapot::{
    jvm::{
        class::{Class, ClassAccessFlags, ClassReference},
        method::ReturnType,
        parsing::Error,
    },
    types::field_type::{FieldType, PrimitiveType},
};

/// Parses the class file compiled from `MyClass.java` from the `test_data` directory.
/// The source code ot the class files is as follows.
fn parse_my_class() -> Result<Class, Error> {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/java_classes/org/pkg/MyClass.class"
    ));
    let reader = BufReader::new(&bytes[..]);
    Class::from_reader(reader)
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
    let expected = ClassAccessFlags::PUBLIC | ClassAccessFlags::SUPER;
    assert_eq!(expected, my_class.access_flags);
}

#[test]
fn test_class_name() {
    let my_class = parse_my_class().unwrap();
    assert_eq!("org/pkg/MyClass", my_class.binary_name);
}

#[test]
fn test_super_class_name() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(
        Some(ClassReference::new("java/lang/Object")),
        my_class.super_class
    );
}

#[test]
fn test_interfaces() {
    let my_class = parse_my_class().unwrap();
    let mut interfaces = my_class.interfaces.into_iter();
    assert_eq!(
        Some(ClassReference::new("java/lang/Cloneable")),
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
        FieldType::Object(ClassReference::new("java/lang/String")).make_array_type(),
        main_method.descriptor.parameters_types[0]
    );
}

fn parse_complicated_class() -> Class {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/java_classes/org/mokapot/test/ComplicatedClass.class"
    ));
    let reader = BufReader::new(&bytes[..]);
    Class::from_reader(reader).unwrap()
}

fn parse_complicated_inner_class() -> Class {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/java_classes/org/mokapot/test/ComplicatedClass$InnerClass.class"
    ));
    let reader = BufReader::new(&bytes[..]);
    Class::from_reader(reader).unwrap()
}

fn parse_complicated_in_method_class() -> Class {
    let bytes = include_bytes!(concat!(
        env!("OUT_DIR"),
        "/java_classes/org/mokapot/test/ComplicatedClass$1Test.class"
    ));
    let reader = BufReader::new(&bytes[..]);
    Class::from_reader(reader).unwrap()
}

#[test]
fn parse_complicated_class_works() {
    parse_complicated_class();
    parse_complicated_inner_class();
    parse_complicated_in_method_class();
}

#[test]
fn not_a_class_file() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/test_data/MyClass.java"
    ));
    assert!(matches!(
        Class::from_reader(bytes.as_slice()),
        Err(Error::NotAClassFile)
    ));
}
