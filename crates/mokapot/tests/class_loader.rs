//! Tests for loading classes from compiled Java fixtures.

#![cfg(java_fixture_tests)]
#![allow(missing_docs)]

use mokapot::jvm::{
    ClassLoader,
    class_loader::{
        Error,
        class_paths::{DirectoryClassPath, JarClassPath},
    },
};

mod support;

#[test]
fn load_class() {
    let loader = ClassLoader::new([DirectoryClassPath::new(support::classes_dir())]);
    let name = "org/mokapot/test/MyClass".parse().unwrap();
    let class = loader.load_class(&name).unwrap();
    assert_eq!(class.binary_name, "org/mokapot/test/MyClass");
}

#[test]
fn jar_class_path() {
    let loader = ClassLoader::new([JarClassPath::new(support::jar_path())]);
    let name = "org/mokapot/test/MyClass".parse().unwrap();
    let class = loader.load_class(&name).unwrap();
    assert_eq!(class.binary_name, "org/mokapot/test/MyClass");
}

#[test]
fn jar_class_path_not_found() {
    let loader = ClassLoader::new([JarClassPath::new(support::jar_path())]);
    assert!(matches!(
        loader.load_class(&"org/mokapot/test/Missing".parse().unwrap()),
        Err(Error::NotFound)
    ));
}
