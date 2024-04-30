#![cfg(integration_test)]

use std::{path::PathBuf, sync::Mutex};

use mokapot::jvm::{
    class_loader::{
        class_paths::{DirectoryClassPath, JarClassPath},
        CachingClassLoader, ClassPath, Error,
    },
    Class, ClassLoader,
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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

fn create_test_dir_class_path() -> DirectoryClassPath {
    DirectoryClassPath::new(concat!(env!("OUT_DIR"), "/mokapot/java_classes"))
}

#[test]
fn load_class() {
    let dir_cp = create_test_dir_class_path();
    let class_loader = ClassLoader::new([dir_cp]);
    let class = class_loader.load_class("org/mokapot/test/MyClass").unwrap();
    assert_eq!(class.binary_name, "org/mokapot/test/MyClass");
}

#[test]
fn load_absent_class() {
    let dir_cp = create_test_dir_class_path();
    let class_loader = ClassLoader::new([dir_cp]);
    let class = class_loader.load_class("org/pkg/MyAbsentClass");
    assert!(matches!(class, Err(Error::NotFound)));
}

struct MockClassPath<'a> {
    counter: &'a Mutex<usize>,
}

impl<'a> MockClassPath<'a> {
    fn new(counter: &'a Mutex<usize>) -> Self {
        Self { counter }
    }
}

impl ClassPath for MockClassPath<'_> {
    fn find_class(&self, _binary_name: &str) -> Result<Class, Error> {
        let mut lock = self
            .counter
            .lock()
            .expect("The counter should not be poisoned");
        *lock += 1;
        let reader = test_data_class!("mokapot", "org/mokapot/test/MyClass");
        Class::from_reader(reader).map_err(Into::into)
    }
}

#[test]
fn caching_class_loader_load_once() {
    let counter = Mutex::new(0);
    let test_cp = MockClassPath::new(&counter);
    let class_loader = CachingClassLoader::from(ClassLoader::new([test_cp]));
    (0..100).into_par_iter().for_each(|_| {
        let class = class_loader.load_class("org/mokapot/test/MyClass").unwrap();
        assert_eq!(class.binary_name, "org/mokapot/test/MyClass");
    });
    let load_count = counter.lock().expect("The counter should not be poisoned");
    assert_eq!(*load_count, 1);
}

#[test]
fn jar_class_path() {
    let Ok(java_home) = std::env::var("JAVA_HOME") else {
        return;
    };
    let jar_path = PathBuf::from(java_home).join("lib").join("jrt-fs.jar");
    let jar_cp = JarClassPath::new(jar_path);
    let class_loader = ClassLoader::new([jar_cp]);

    assert!(class_loader
        .load_class("jdk/internal/jimage/ImageReader")
        .is_ok());
}

#[test]
fn jar_class_path_not_found() {
    let Ok(java_home) = std::env::var("JAVA_HOME") else {
        return;
    };
    let jar_path = PathBuf::from(java_home).join("lib").join("jrt-fs.jar");
    let jar_cp = JarClassPath::new(jar_path);
    let class_loader = ClassLoader::new([jar_cp]);

    assert!(matches!(
        class_loader.load_class("jdk/internal/jimage/ImageReader3"),
        Err(Error::NotFound)
    ));
}

#[test]
fn jar_class_path_not_jar() {
    let jar_path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let jar_cp = JarClassPath::new(jar_path);
    let class_loader = ClassLoader::new([jar_cp]);

    assert!(matches!(
        class_loader.load_class("jdk/internal/jimage/ImageReader"),
        Err(Error::Other(_)),
    ));
}

fn _class_path_object_safety(_b: Box<dyn ClassPath>) {
    // For compilation checking only.
}
