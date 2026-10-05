use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[test]
fn loader_exhausts_class_paths_after_not_found() {
    struct Missing<'a>(&'a AtomicUsize);
    impl ClassPath for Missing<'_> {
        fn find_class(&self, _: &BinaryName) -> Result<Class, Error> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(Error::NotFound)
        }
    }
    let missing_calls = AtomicUsize::new(0);
    let second_calls = AtomicUsize::new(0);
    let loader = ClassLoader::new(vec![
        Box::new(Missing(&missing_calls)) as Box<dyn ClassPath>,
        Box::new(Missing(&second_calls)),
    ]);
    let name = "org/mokapot/test/MyClass".parse().unwrap();
    assert!(matches!(loader.load_class(&name), Err(Error::NotFound)));
    assert_eq!(missing_calls.load(Ordering::Relaxed), 1);
    assert_eq!(second_calls.load(Ordering::Relaxed), 1);
}

#[test]
fn loader_propagates_non_not_found_errors() {
    struct Failure;
    impl ClassPath for Failure {
        fn find_class(&self, _: &BinaryName) -> Result<Class, Error> {
            Err(Error::Other("failure".into()))
        }
    }
    let later_calls = AtomicUsize::new(0);
    let loader = ClassLoader::new(vec![
        Box::new(Failure) as Box<dyn ClassPath>,
        Box::new(MockClassPath {
            counter: &later_calls,
        }),
    ]);
    let error = loader
        .load_class(&"org/mokapot/test/MyClass".parse().unwrap())
        .unwrap_err();
    assert!(matches!(error, Error::Other(_)));
    assert_eq!(later_calls.load(Ordering::Relaxed), 0);
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

#[test]
fn class_path_is_object_safe() {
    let _: Box<dyn ClassPath> = Box::new(super::class_paths::NopClassPath);
}
