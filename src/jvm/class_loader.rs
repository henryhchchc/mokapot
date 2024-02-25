//! Discovering and loading classes.

use std::{collections::HashMap, fs::File, io::BufReader, ops::Deref, sync::Mutex};

use thiserror::Error;

#[cfg(feature = "jar")]
use zip::{result::ZipError, ZipArchive};

use super::class::Class;

/// An error that can occur while loading a class.
#[derive(Error, Debug)]
pub enum ClassLoadingError {
    /// The class could not be found.
    #[error("Class not found: {0}")]
    NotFound(String),
    /// Error occurred while parsing the class bytes.
    #[error("Error parsing class bytes: {0}")]
    Parse(#[from] super::parsing::errors::Error),
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
    /// See [`ClassLoadingError`].
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError>;
}

impl<T> ClassPath for T
where
    T: Deref,
    <T as Deref>::Target: ClassPath,
{
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError> {
        self.deref().find_class(binary_name)
    }
}

/// A class loader that can load classes from a list of class paths.
#[derive(Debug)]
pub struct ClassLoader<P> {
    class_path: Vec<P>,
}

impl<P: ClassPath> ClassLoader<P> {
    /// Create a new class loader with the given class paths.
    ///
    /// # Errors
    /// See [`ClassLoadingError`].
    pub fn load_class(&self, binary_name: impl AsRef<str>) -> Result<Class, ClassLoadingError> {
        for class_path in &self.class_path {
            match class_path.find_class(binary_name.as_ref()) {
                Ok(class) => return Ok(class),
                Err(ClassLoadingError::NotFound(_)) => continue,
                Err(err) => return Err(err),
            }
        }
        Err(ClassLoadingError::NotFound(binary_name.as_ref().to_owned()))
    }
}

impl<P> ClassLoader<P> {
    /// Create a new class loader with the given class paths.
    #[must_use]
    pub fn new(class_path: impl Into<Vec<P>>) -> Self {
        let class_path = class_path.into();
        Self { class_path }
    }

    /// Convert this class loader into a [`CachingClassLoader`].
    #[must_use]
    pub fn into_cached(self) -> CachingClassLoader<P> {
        CachingClassLoader {
            class_loader: self,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

/// A class path that searches for classes in a directory.
#[derive(Debug)]
pub struct DirectoryClassPath {
    directory: std::path::PathBuf,
}

impl ClassPath for DirectoryClassPath {
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError> {
        let class_file_path = self.directory.join(binary_name).with_extension("class");
        if class_file_path.exists() {
            let class_file = File::open(class_file_path)?;
            let mut buf_read = BufReader::new(class_file);
            let class = Class::parse(&mut buf_read)?;
            Ok(class)
        } else {
            Err(ClassLoadingError::NotFound(binary_name.to_owned()))
        }
    }
}

impl DirectoryClassPath {
    /// Create a new directory class path.
    pub fn new(directory: impl Into<std::path::PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
}

/// A class path that searches for classes in a JAR file.
#[derive(Debug)]
#[cfg(feature = "jar")]
pub struct JarClassPath {
    jar_file: std::path::PathBuf,
}

#[cfg(feature = "jar")]
impl JarClassPath {
    /// Create a new JAR class path.
    pub fn new(jar_file: impl Into<std::path::PathBuf>) -> Self {
        Self {
            jar_file: jar_file.into(),
        }
    }
}

#[cfg(feature = "jar")]
impl ClassPath for JarClassPath {
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError> {
        let jar_file = File::open(&self.jar_file)?;
        let jar_reader = BufReader::new(jar_file);
        let mut jar_archive = ZipArchive::new(jar_reader).map_err(|e| match e {
            ZipError::Io(io_err) => ClassLoadingError::IO(io_err),
            e => ClassLoadingError::Other(Box::new(e)),
        })?;
        let mut class_file = match jar_archive.by_name(&format!("{binary_name}.class")) {
            Ok(it) => it,
            Err(ZipError::FileNotFound) => {
                Err(ClassLoadingError::NotFound(binary_name.to_owned()))?
            }
            Err(ZipError::Io(io_err)) => Err(ClassLoadingError::IO(io_err))?,
            Err(e) => Err(ClassLoadingError::Other(Box::new(e)))?,
        };
        Class::from_reader(&mut class_file).map_err(Into::into)
    }
}

/// A class loader that caches loaded classes.
#[derive(Debug)]
pub struct CachingClassLoader<P> {
    class_loader: ClassLoader<P>,
    cache: Mutex<HashMap<String, Class>>,
}

impl<P: ClassPath> CachingClassLoader<P> {
    /// Loads a class from the class loader's cache, or loads it from the class loader if it is
    /// not.
    ///
    /// # Errors
    /// See [`ClassLoadingError`].
    #[allow(
        clippy::missing_panics_doc,
        // TODO: Uncomment the following when lint reason is stabalized.
        //       See https://github.com/rust-lang/rust/issues/54503
        // reason = "The unwrap is garenteed to not panic."
    )]
    pub fn load_class(&self, binary_name: impl AsRef<str>) -> Result<&Class, ClassLoadingError> {
        let mut cache = match self.cache.lock() {
            Ok(it) => it,
            Err(poison_err) => {
                // The operaion on `self.cache` should not panic.
                // When the other thread holding the lock get panic, the panic should happen before
                // modifying the cache.
                // Therefore, it is safe to take the lock even if it is poisoned.
                poison_err.into_inner()
            }
        };
        let key_ref = binary_name.as_ref();
        let class = if let Some(class) = cache.get(key_ref) {
            class
        } else {
            let class = self.class_loader.load_class(key_ref)?;
            let overridden = cache.insert(key_ref.to_owned(), class);
            debug_assert!(overridden.is_none(), "Class is already in the cache");
            // The unwrap is safe since the class was just inserted into the cache.
            cache.get(key_ref).unwrap()
        };
        // SAFETY: The class reference is valid for the lifetime of the cache.
        Ok(unsafe { std::mem::transmute(class) })
    }
}
