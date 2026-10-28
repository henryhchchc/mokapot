//! Implementations of [`ClassPath`].

use std::{
    collections::HashSet,
    io::{self, ErrorKind},
    path::PathBuf,
};
#[cfg(feature = "jar")]
use std::{
    fs::File,
    io::{BufReader, Read},
};

#[cfg(feature = "jar")]
use zip::{ZipArchive, result::ZipError};

use super::{ClassNames, ClassPath, Error};
use crate::{jvm::Class, types::class_name::ClassName};

/// A class path that does nothing.
///
/// This is used as a type place holder when an empry collection of class paths is required.
/// See [`NopClassPath::EMPTY`].
#[derive(Debug)]
pub struct NopClassPath;

impl NopClassPath {
    /// An empty class path.
    pub const EMPTY: [NopClassPath; 0] = [];
}

impl ClassPath for NopClassPath {
    fn find_class(&self, _class_name: &ClassName) -> Result<Class, Error> {
        Err(Error::NotFound)
    }
}

impl ClassNames for NopClassPath {
    fn class_names(&self) -> Result<HashSet<ClassName>, Error> {
        Ok(HashSet::new())
    }
}

/// A class path that searches for classes in a directory.
#[derive(Debug)]
pub struct DirectoryClassPath {
    directory: PathBuf,
}

impl ClassPath for DirectoryClassPath {
    fn find_class(&self, class_name: &ClassName) -> Result<Class, Error> {
        let class_name = class_name
            .as_java_str()
            .to_str()
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error))?;
        let class_file_path = self.directory.join(&*class_name).with_extension("class");
        if class_file_path.exists() {
            let bytes = std::fs::read(class_file_path)?;
            let class = Class::from_bytes(&bytes)?;
            Ok(class)
        } else {
            Err(Error::NotFound)
        }
    }
}

impl DirectoryClassPath {
    /// Create a new directory class path.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
}

impl ClassNames for DirectoryClassPath {
    fn class_names(&self) -> Result<HashSet<ClassName>, Error> {
        let mut names = HashSet::new();
        for entry in walkdir::WalkDir::new(&self.directory) {
            let entry = entry.map_err(|error| Error::Other(Box::new(error)))?;
            if entry.path().extension().is_none_or(|ext| ext != "class") {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(&self.directory)
                .map_err(|error| Error::Other(Box::new(error)))?
                .with_extension("");
            let name = relative.to_str().ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidData, "class path is not valid UTF-8")
            })?;
            if let Ok(name) = name.parse() {
                names.insert(name);
            }
        }
        Ok(names)
    }
}

/// A class path that searches for classes in a JAR file.
#[derive(Debug)]
#[cfg(feature = "jar")]
pub struct JarClassPath {
    jar_file: PathBuf,
}

#[cfg(feature = "jar")]
impl JarClassPath {
    /// Create a new JAR class path.
    pub fn new(jar_file: impl Into<PathBuf>) -> Self {
        Self {
            jar_file: jar_file.into(),
        }
    }
}

#[cfg(feature = "jar")]
impl ClassPath for JarClassPath {
    fn find_class(&self, class_name: &ClassName) -> Result<Class, Error> {
        let class_name = class_name
            .as_java_str()
            .to_str()
            .map_err(|error| io::Error::new(ErrorKind::InvalidInput, error))?;
        let jar_file = File::open(&self.jar_file)?;
        let jar_reader = BufReader::new(jar_file);
        let mut jar_archive = ZipArchive::new(jar_reader).map_err(|e| match e {
            ZipError::Io(io_err) => Error::IO(io_err),
            e => Error::Other(Box::new(e)),
        })?;
        let mut class_file = jar_archive
            .by_name(&format!("{class_name}.class"))
            .map_err(|e| match e {
                ZipError::FileNotFound => Error::NotFound,
                ZipError::Io(io_err) => Error::IO(io_err),
                e => Error::Other(Box::new(e)),
            })?;
        let mut bytes = Vec::new();
        class_file.read_to_end(&mut bytes)?;
        Class::from_bytes(&bytes).map_err(Into::into)
    }
}

#[cfg(feature = "jar")]
impl ClassNames for JarClassPath {
    fn class_names(&self) -> Result<HashSet<ClassName>, Error> {
        let jar_file = File::open(&self.jar_file)?;
        let jar_reader = BufReader::new(jar_file);
        let jar_archive = ZipArchive::new(jar_reader).map_err(|error| match error {
            ZipError::Io(error) => Error::IO(error),
            error => Error::Other(Box::new(error)),
        })?;
        let mut names = HashSet::new();
        for name in jar_archive.file_names() {
            let name = name.map_err(|error| Error::Other(Box::new(error)))?;
            if let Some(name) = name.strip_suffix(".class")
                && let Ok(name) = name.parse()
            {
                names.insert(name);
            }
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_class_path_reports_absent_class() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let result =
            DirectoryClassPath::new(path).find_class(&"org/pkg/MyAbsentClass".parse().unwrap());
        assert!(matches!(result, Err(Error::NotFound)));
    }

    #[cfg(feature = "jar")]
    #[test]
    fn jar_class_path_rejects_non_jar_file() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let result =
            JarClassPath::new(path).find_class(&"org/mokapot/test/MyClass".parse().unwrap());
        assert!(matches!(result, Err(Error::Other(_))));
    }
}
