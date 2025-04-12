//! Implementations of [`ClassPath`].

use std::{collections::HashSet, fs::File, io::BufReader};

#[cfg(feature = "jar")]
use zip::{ZipArchive, result::ZipError};

use super::{ClassPath, Error};
use crate::{
    analysis::ClassRefs,
    jvm::{Class, references::ClassRef},
};
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
            let mut buf_read = BufReader::new(class_file);
            let class = Class::from_reader(&mut buf_read)?;
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

impl ClassRefs for DirectoryClassPath {
    fn class_refs(&self) -> HashSet<ClassRef> {
        walkdir::WalkDir::new(&self.directory)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|it| it.path().extension().is_some_and(|it| it == "class"))
            .map(|it| {
                let binary_name = it
                    .path()
                    .strip_prefix(&self.directory)
                    .expect("The directory should start with `self.directory`")
                    .with_extension("")
                    .to_str()
                    .expect("The path name is not valid UTF-8")
                    .to_owned();
                ClassRef { binary_name }
            })
            .collect()
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

#[cfg(feature = "jar")]
impl ClassRefs for JarClassPath {
    fn class_refs(&self) -> HashSet<ClassRef> {
        let Ok(jar_file) = File::open(&self.jar_file) else {
            return HashSet::default();
        };
        let jar_reader = BufReader::new(jar_file);
        let Ok(jar_archive) = ZipArchive::new(jar_reader) else {
            return HashSet::default();
        };
        jar_archive
            .file_names()
            .filter_map(|it| it.strip_suffix(".class"))
            .map(|binary_name| {
                let binary_name = binary_name.to_owned();
                ClassRef { binary_name }
            })
            .collect()
    }
}
