use std::sync::{
    atomic::{AtomicUsize, Ordering::Relaxed},
    Arc,
};

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

#[derive(Debug)]
struct TestClassPath {
    inner: DirectoryClassPath,
    counter: Arc<AtomicUsize>,
}

impl TestClassPath {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        Self {
            inner: create_test_dir_class_path(),
            counter,
        }
    }
}

impl ClassPath for TestClassPath {
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError> {
        self.counter.fetch_add(1, Relaxed);
        self.inner.find_class(binary_name)
    }
}

#[test]
fn caching_class_loader_load_once() {
    let counter = Arc::default();
    let test_cp = TestClassPath::new(Arc::clone(&counter));
    let class_loader = ClassLoader::new([test_cp]).into_cached();
    for _ in 0..10 {
        let class = class_loader.load_class("org/pkg/MyClass").unwrap();
        assert_eq!(class.binary_name, "org/pkg/MyClass");
    }
    assert_eq!(counter.load(Relaxed), 1);
}
