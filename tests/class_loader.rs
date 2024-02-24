use std::cell::Cell;

use mokapot::jvm::{
    class::Class,
    class_loader::{ClassLoader, ClassLoadingError, ClassPath, DirectoryClassPath},
};

fn create_test_dir_class_path() -> DirectoryClassPath {
    DirectoryClassPath::new(concat!(env!("OUT_DIR"), "/java_classes"))
}

#[test]
fn load_class() {
    let dir_cp = create_test_dir_class_path();
    let class_loader = ClassLoader::new([dir_cp]);
    let class = class_loader.load_class("org/pkg/MyClass").unwrap();
    assert_eq!(class.binary_name, "org/pkg/MyClass");
}

#[test]
fn load_absent_class() {
    let dir_cp = create_test_dir_class_path();
    let class_loader = ClassLoader::new([dir_cp]);
    let class = class_loader.load_class("org/pkg/MyAbsentClass");
    assert!(matches!(class, Err(ClassLoadingError::NotFound(_))));
}

struct MockClassPath<'a> {
    counter: &'a Cell<usize>,
}

impl<'a> MockClassPath<'a> {
    fn new(counter: &'a Cell<usize>) -> Self {
        Self { counter }
    }
}

impl ClassPath for MockClassPath<'_> {
    fn find_class(&self, _binary_name: &str) -> Result<Class, ClassLoadingError> {
        self.counter.set(self.counter.get() + 1);
        let reader = include_bytes!(concat!(
            env!("OUT_DIR"),
            "/java_classes/org/pkg/MyClass.class"
        ));
        Class::from_reader(reader.as_slice()).map_err(Into::into)
    }
}

#[test]
fn caching_class_loader_load_once() {
    let counter = Cell::new(0);
    let test_cp = MockClassPath::new(&counter);
    let class_loader = ClassLoader::new([test_cp]).into_cached();
    for _ in 0..10 {
        let class = class_loader.load_class("org/pkg/MyClass").unwrap();
        assert_eq!(class.binary_name, "org/pkg/MyClass");
    }
    assert_eq!(counter.get(), 1);
}
