//! Discovering and loading classes.

use std::{collections::HashMap, fs::File, io::BufReader, sync::Mutex};

use thiserror::Error;
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
    Parse(#[from] super::parsing::errors::ClassFileParsingError),
    /// Error occurred while reading the class bytes or locating the class file.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    /// Other error occurred.
    #[error("Cause: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A class path that can be searched for classes.
pub trait ClassPath
where
    Self: std::fmt::Debug,
{
    /// Find a class by its binary name.
    ///
    /// # Errors
    /// See [`ClassLoadingError`].
    fn find_class(&self, binary_name: &str) -> Result<Class, ClassLoadingError>;
}

/// A class loader that can load classes from a list of class paths.
#[derive(Debug)]
pub struct ClassLoader {
    class_path: Vec<Box<dyn ClassPath>>,
}

impl ClassLoader {
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

    /// Create a new class loader with the given class paths.
    #[must_use]
    pub fn new(class_path: Vec<Box<dyn ClassPath>>) -> Self {
        Self { class_path }
    }

    /// Convert this class loader into a [`CachingClassLoader`].
    #[must_use]
    pub fn into_cached(self) -> CachingClassLoader {
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
pub struct JarClassPath {
    jar_file: std::path::PathBuf,
}

impl JarClassPath {
    /// Create a new JAR class path.
    pub fn new(jar_file: impl Into<std::path::PathBuf>) -> Self {
        Self {
            jar_file: jar_file.into(),
        }
    }
}

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
        Class::from_reader(&mut class_file).map_err(std::convert::Into::into)
    }
}

/// A class loader that caches loaded classes.
#[derive(Debug)]
pub struct CachingClassLoader {
    class_loader: ClassLoader,
    cache: Mutex<HashMap<String, Class>>,
}

impl CachingClassLoader {
    /// Loads a class from the class loader's cache, or loads it from the class loader if it is
    /// not.
    ///
    /// # Errors
    /// See [`ClassLoadingError`].
    pub fn load_class(&self, binary_name: impl AsRef<str>) -> Result<&Class, ClassLoadingError> {
        let mut cache_guard = match self.cache.lock() {
            Ok(it) => it,
            Err(poison_err) => poison_err.into_inner(),
        };
        let key_ref = binary_name.as_ref();
        let class = if let Some(class) = cache_guard.get(key_ref) {
            class
        } else {
            let class = self.class_loader.load_class(key_ref)?;
            cache_guard.insert(key_ref.to_owned(), class);
            cache_guard.get(key_ref).unwrap_or_else(|| unreachable!())
        };
        // SAFETY: The class reference is valid for the lifetime of the cache.
        Ok(unsafe { std::mem::transmute(class) })
    }
}
