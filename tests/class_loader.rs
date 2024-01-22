use mokapot::jvm::class_loader::{ClassLoader, ClassLoadingError, ClassPath, DirectoryClassPath};

fn create_dir_class_loader() -> ClassLoader {
    let class_path: Vec<Box<dyn ClassPath>> = vec![Box::new(DirectoryClassPath::new(concat!(
        env!("OUT_DIR"),
        "/java_classes"
    )))];
    ClassLoader::new(class_path)
}

#[test]
fn load_class() {
    let class_loader = create_dir_class_loader();
    let class = class_loader.load_class("org/pkg/MyClass").unwrap();
    assert_eq!(class.binary_name, "org/pkg/MyClass");
}

#[test]
fn load_absent_class() {
    let class_loader = create_dir_class_loader();
    let class = class_loader.load_class("org/pkg/MyAbsentClass");
    assert!(matches!(class, Err(ClassLoadingError::NotFound(_))));
}
