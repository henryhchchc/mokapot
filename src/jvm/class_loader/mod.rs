//! Discovering and loading classes.

use std::{borrow::Borrow, collections::HashMap, mem::transmute, ops::Deref, sync::RwLock};

use super::Class;

/// An error that can occur while loading a class.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The class could not be found.
    #[error("Class not found")]
    NotFound,
    /// Error occurred while parsing the class bytes.
    #[error("Error parsing class bytes: {0}")]
    Malformed(#[from] super::parsing::errors::Error),
    /// Error occurred while reading the class bytes or locating the class file.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    /// Other error occurred.
    #[error("Cause: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A class path that can be searched for classes.
pub trait ClassPath {
    /// Find a class by its binary name.
    ///
    /// # Errors
    /// See [`Error`].
    fn find_class(&self, binary_name: &str) -> Result<Class, Error>;
}

impl<T> ClassPath for T
where
    T: Deref,
    <T as Deref>::Target: ClassPath,
{
    fn find_class(&self, binary_name: &str) -> Result<Class, Error> {
        self.deref().find_class(binary_name)
    }
}

/// A class loader that can load classes from a list of class paths.
#[derive(Debug)]
pub struct ClassLoader<P> {
    class_path: Vec<P>,
}

impl<P> ClassLoader<P> {
    /// Create a new class loader with the given class paths.
    ///
    /// # Errors
    /// See [`Error`].
    pub fn load_class(&self, binary_name: &str) -> Result<Class, Error>
    where
        P: ClassPath,
    {
        for class_path in &self.class_path {
            match class_path.find_class(binary_name) {
                Ok(class) => return Ok(class),
                Err(Error::NotFound) => continue,
                Err(err) => return Err(err),
            }
        }
        Err(Error::NotFound)
    }

    /// Create a new class loader with the given class paths.
    #[must_use]
    pub fn new<C: Into<Vec<P>>>(class_path: C) -> Self {
        let class_path = class_path.into();
        Self { class_path }
    }

    /// Convert this class loader into a [`CachingClassLoader`].
    #[must_use]
    #[deprecated(note = "Use [`CachingClassLoader::from`] instead")]
    pub fn into_cached(self) -> CachingClassLoader<P> {
        CachingClassLoader::from(self)
    }
}

pub mod class_paths;

/// A class loader that caches loaded classes.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct CachingClassLoader<P> {
    class_loader: ClassLoader<P>,
    cache: RwLock<HashMap<String, Box<Class>>>,
}

impl<P> CachingClassLoader<P> {
    /// Loads a class from the class loader's cache, or loads it from the class loader if it is
    /// not.
    ///
    /// # Errors
    /// See [`Error`].
    #[allow(
        clippy::missing_panics_doc,
        // TODO: Uncomment the following when lint reason is stabalized.
        //       See https://github.com/rust-lang/rust/issues/54503
        // reason = "The unwrap is garenteed to not panic."
    )]
    pub fn load_class(&self, binary_name: impl Borrow<str>) -> Result<&Class, Error>
    where
        P: ClassPath,
    {
        let cache = match self.cache.read() {
            Ok(it) => it,
            Err(poison_err) => {
                // The operaion on `self.cache` should not panic.
                // When the other thread holding the lock get panic, the panic should happen before
                // modifying the cache.
                // Therefore, it is safe to take the lock even if it is poisoned.
                poison_err.into_inner()
            }
        };
        let class_ref = if let Some(b) = cache.get(binary_name.borrow()) {
            // SAFETY: We never remove elements from the cache so the `Box` is not dropped until
            // `self.cache` gets dropped, which is when `self` gets dropped.
            // Therefore, it is ok to extend the lifetime of the reference to the lifetime of `self`.
            unsafe { transmute(b.as_ref()) }
        } else {
            drop(cache);
            let mut cache = match self.cache.write() {
                Ok(it) => it,
                Err(poison_err) => poison_err.into_inner(),
            };
            // It is possible that the class is loaded before we get the write lock.
            // Therefore, we need to check the cache again.
            let class_box = if let Some(b) = cache.get(binary_name.borrow()) {
                b
            } else {
                let class = self.class_loader.load_class(binary_name.borrow())?;
                let overridden = cache.insert(binary_name.borrow().to_owned(), Box::new(class));
                debug_assert!(overridden.is_none(), "Class is already in the cache");
                // The unwrap is safe since the class was just inserted into the cache.
                cache.get(binary_name.borrow()).unwrap()
            };
            // SAFETY: We never remove elements from the cache so the `Box` is not dropped until
            // `self.cache` gets dropped, which is when `self` gets dropped.
            // Therefore, it is ok to extend the lifetime of the reference to the lifetime of `self`.
            unsafe { transmute(class_box.as_ref()) }
        };
        Ok(class_ref)
    }
}

impl<P> From<ClassLoader<P>> for CachingClassLoader<P> {
    fn from(class_loader: ClassLoader<P>) -> Self {
        Self {
            class_loader,
            cache: RwLock::new(HashMap::new()),
        }
    }
}
