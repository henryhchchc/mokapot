use std::{io::BufReader};

use crate::{elements::{class_file::ClassFile}, access_flags};


/// Parses the class file `MyClass.class` from the `test_data` directory.
/// The source code ot the class files is as follows.
fn parse_my_class() -> Result<ClassFile, crate::elements::class_file::ClassFileParsingError> {
    let bytes = include_bytes!("../test_data/MyClass.class");
    let mut reader = BufReader::new(&bytes[..]);
    ClassFile::parse(&mut reader)
}

#[test]
fn test_parse_file() {
    parse_my_class().unwrap();
}

#[test]
fn test_parse_version() {
    let my_class = parse_my_class().unwrap().to_class().unwrap();
    assert_eq!(64, my_class.version.major);
    assert_eq!(0, my_class.version.minor);
    assert!(!my_class.version.is_preview_enabled());
}

#[test]
fn test_access_flag() {
    let my_class = parse_my_class().unwrap().to_class().unwrap();
    let expected = access_flags::ACC_PUBLIC | access_flags::ACC_SUPER;
    assert_eq!(expected, my_class.access_flags);
}

#[test]
fn test_class_name() {
    let my_class = parse_my_class().unwrap().to_class().unwrap();
    assert_eq!("org/pkg/MyClass", my_class.binary_name);
}


#[test]
fn test_super_class_name() {
    let my_class = parse_my_class().unwrap().to_class().unwrap();
    assert_eq!("java/lang/Object", my_class.super_class_binary_name);
}



#[test]
fn test_interfaces() {
    let my_class = parse_my_class().unwrap().to_class().unwrap();
    let mut interfaces = my_class.interface_binary_names.into_iter();
    assert_eq!(Some("java/lang/Cloneable".to_string()), interfaces.next());
}
