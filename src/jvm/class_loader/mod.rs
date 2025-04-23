//! Discovering and loading classes.

use std::{borrow::Borrow, ops::Deref};

use super::{Class, ClassLoader};
use crate::utils::Cache;

/// An error that can occur while loading a class.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The class could not be found.
    #[error("Class not found")]
    NotFound,
    /// Error occurred while parsing the class bytes.
    #[error("Error parsing class bytes: {0}")]
    Malformed(#[from] super::bytecode::errors::ParseError),
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
                Err(Error::NotFound) => {}
                Err(err) => return Err(err),
            }
        }
        Err(Error::NotFound)
    }

    /// Create a new class loader with the given class paths.
    #[must_use]
    pub fn new<C: IntoIterator<Item = P>>(class_path: C) -> Self {
        let class_path = class_path.into_iter().collect();
        Self { class_path }
    }

    /// Convert this class loader into a [`CachingClassLoader`].
    #[must_use]
    #[deprecated(note = "Use `CachingClassLoader::from` instead")]
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
    cache: Cache<String, Class>,
}

impl<P> CachingClassLoader<P> {
    /// Loads a class from the class loader's cache, or loads it from the class loader if it is
    /// not.
    ///
    /// # Errors
    /// See [`Error`].
    pub fn load_class<N>(&self, binary_name: &N) -> Result<&Class, Error>
    where
        P: ClassPath,
        N: ?Sized + Borrow<str>,
    {
        self.cache
            .get_or_try_put(binary_name.borrow(), |it| self.class_loader.load_class(it))
    }
}

impl<P> From<ClassLoader<P>> for CachingClassLoader<P> {
    fn from(class_loader: ClassLoader<P>) -> Self {
        Self {
            class_loader,
            cache: Cache::new(),
        }
    }
}
