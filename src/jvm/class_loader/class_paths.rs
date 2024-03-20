//! Implementations of [`ClassPath`].

use std::{fs::File, io::BufReader};

#[cfg(feature = "jar")]
use zip::{result::ZipError, ZipArchive};

use crate::jvm::class::Class;

use super::{ClassPath, Error};
/// A class path that searches for classes in a directory.
#[derive(Debug)]
pub struct DirectoryClassPath {
    directory: std::path::PathBuf,
}

impl ClassPath for DirectoryClassPath {
    fn find_class(&self, binary_name: &str) -> Result<Class, Error> {
        let class_file_path = self.directory.join(binary_name).with_extension("class");
        if class_file_path.exists() {
            let class_file = File::open(class_file_path)?;
            let buf_read = BufReader::new(class_file);
            let class = Class::from_reader(buf_read)?;
            Ok(class)
        } else {
            Err(Error::NotFound)
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
    fn find_class(&self, binary_name: &str) -> Result<Class, Error> {
        let jar_file = File::open(&self.jar_file)?;
        let jar_reader = BufReader::new(jar_file);
        let mut jar_archive = ZipArchive::new(jar_reader).map_err(|e| match e {
            ZipError::Io(io_err) => Error::IO(io_err),
            e => Error::Other(Box::new(e)),
        })?;
        let mut class_file = jar_archive
            .by_name(&format!("{binary_name}.class"))
            .map_err(|e| match e {
                ZipError::FileNotFound => Error::NotFound,
                ZipError::Io(io_err) => Error::IO(io_err),
                e => Error::Other(Box::new(e)),
            })?;
        Class::from_reader(&mut class_file).map_err(Into::into)
    }
}
