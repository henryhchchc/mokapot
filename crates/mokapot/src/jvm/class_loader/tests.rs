use std::sync::atomic::{AtomicUsize, Ordering};

use super::{CachingClassLoader, ClassLoader, ClassPath, Error, class_paths::DirectoryClassPath};
use crate::{jvm::Class, types::binary_name::BinaryName};

#[test]
fn load_absent_class() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let loader = ClassLoader::new([DirectoryClassPath::new(path)]);
    let result = loader.load_class(&"org/pkg/MyAbsentClass".parse().unwrap());
    assert!(matches!(result, Err(Error::NotFound)));
}

struct MockClassPath<'a> {
    counter: &'a AtomicUsize,
}

impl ClassPath for MockClassPath<'_> {
    fn find_class(&self, name: &BinaryName) -> Result<Class, Error> {
        self.counter.fetch_add(1, Ordering::Relaxed);
        Ok(Class {
            binary_name: name.clone(),
            ..Default::default()
        })
    }
}

#[test]
fn caching_class_loader_load_once() {
    let counter = AtomicUsize::new(0);
    let loader = CachingClassLoader::from(ClassLoader::new([MockClassPath { counter: &counter }]));
    let name: BinaryName = "org/mokapot/test/MyClass".parse().unwrap();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let loader = &loader;
                let name = &name;
                scope.spawn(move || {
                    for _ in 0..25 {
                        assert_eq!(loader.load_class(name).unwrap().binary_name, *name);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    assert_eq!(1, counter.load(Ordering::Relaxed));
}

#[cfg(feature = "jar")]
#[test]
fn jar_class_path_not_jar() {
    use super::class_paths::JarClassPath;

    let jar = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let loader = ClassLoader::new([JarClassPath::new(jar)]);
    assert!(matches!(
        loader.load_class(&"org/mokapot/test/MyClass".parse().unwrap()),
        Err(Error::Other(_))
    ));
}

#[test]
fn class_path_is_object_safe() {
    let _: Box<dyn ClassPath> = Box::new(super::class_paths::NopClassPath);
}
