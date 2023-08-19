use std::io::BufReader;

use crate::elements::{
    class::{Class, ClassAccessFlags},
    class_parser::{ClassFileParsingError, ClassParser},
    field::{FieldType, PrimitiveType},
    references::ClassReference,
};

/// Parses the class file `MyClass.class` from the `test_data` directory.
/// The source code ot the class files is as follows.
fn parse_my_class() -> Result<Class, ClassFileParsingError> {
    let bytes = include_bytes!("../test_data/MyClass.class");
    let mut reader = BufReader::new(&bytes[..]);
    ClassParser::from_reader(&mut reader).parse()
}

#[test]
fn test_parse_file() {
    print!("{:#?}", parse_my_class().unwrap());
}

#[test]
fn test_parse_version() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(64, my_class.version.major);
    assert_eq!(0, my_class.version.minor);
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
    assert_eq!(
        ClassReference {
            name: "org/pkg/MyClass".to_string()
        },
        my_class.this_class
    );
}

#[test]
fn test_super_class_name() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(
        ClassReference {
            name: "java/lang/Object".to_string()
        },
        my_class.super_class
    );
}

#[test]
fn test_interfaces() {
    let my_class = parse_my_class().unwrap();
    let mut interfaces = my_class.interfaces.into_iter();
    assert_eq!(
        Some(ClassReference {
            name: "java/lang/Cloneable".to_string()
        }),
        interfaces.next()
    );
}

#[test]
fn test_fields() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(2, my_class.fields.len());
    let test_field = &my_class
        .fields
        .iter()
        .filter(|f| f.name == "test")
        .next()
        .unwrap();
    assert_eq!(FieldType::Base(PrimitiveType::Long), test_field.field_type);
}

#[test]
fn test_methods() {
    let my_class = parse_my_class().unwrap();
    assert_eq!(4, my_class.methods.len());
    let main_method = &my_class
        .methods
        .iter()
        .filter(|f| f.name == "main")
        .next()
        .unwrap();
    assert_eq!("([Ljava/lang/String;)V", main_method.descriptor);
}
