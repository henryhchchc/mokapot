//! APIs for static analysis.

use crate::types::class_name::ClassName;

use std::collections::{HashMap, HashSet};

use crate::jvm::Class;
use crate::jvm::class_loader::{ClassNames, ClassPath, Error as ClassPathError};

pub mod fixed_point;
pub mod type_hierarchy;

/// A context for class resolution during analysis.
#[derive(Debug)]
pub struct ResolutionContext {
    /// The application classes.
    pub application_classes: HashMap<ClassName, Class>,
    /// The library classes.
    pub library_classes: HashMap<ClassName, Class>,
    /// The class hierarchy.
    pub class_hierarchy: ClassHierarchy,
    /// The interface implementations.
    pub interface_implementations: InterfaceImplHierarchy,
}

impl ResolutionContext {
    /// Create a new resolution context.
    ///
    /// # Errors
    /// Returns an error if a class path cannot be enumerated or a class cannot be loaded.
    pub fn new<ACP, LCP>(app_class_path: ACP, lib_class_path: LCP) -> Result<Self, InitError>
    where
        ACP: IntoIterator<Item: ClassPath + ClassNames>,
        LCP: IntoIterator<Item: ClassPath + ClassNames>,
    {
        let application_classes = load_classes(app_class_path)?;
        let library_classes = load_classes(lib_class_path)?;
        let all_classes = application_classes.values().chain(library_classes.values());
        let class_hierarchy = ClassHierarchy::from_classes(all_classes.clone());
        let interface_implementations = InterfaceImplHierarchy::from_classes(all_classes);
        Ok(Self {
            application_classes,
            library_classes,
            class_hierarchy,
            interface_implementations,
        })
    }
}

/// An error that occurs during initialization of a [`ResolutionContext`].
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// A class path could not be enumerated.
    #[error("failed to enumerate class path: {0}")]
    Discovery(#[source] ClassPathError),
    /// A discovered class could not be loaded.
    #[error("failed to load class {name}: {source}")]
    Load {
        /// The discovered class name.
        name: ClassName,
        /// The underlying loading error.
        #[source]
        source: ClassPathError,
    },
}

fn load_classes<CP>(class_path: CP) -> Result<HashMap<ClassName, Class>, InitError>
where
    CP: IntoIterator<Item: ClassPath + ClassNames>,
{
    let mut classes = HashMap::new();
    for cp in class_path {
        for name in cp.class_names().map_err(InitError::Discovery)? {
            let class = cp
                .find_class(&name)
                .map_err(|source| InitError::Load { name, source })?;
            classes.insert(class.name().clone(), class);
        }
    }
    Ok(classes)
}

/// A class hierarchy based on super class relationships.
#[derive(Debug, Clone)]
pub struct ClassHierarchy {
    #[cfg_attr(
        not(feature = "petgraph"),
        expect(dead_code, reason = "used by petgraph traversal")
    )]
    inheritance: HashMap<ClassName, HashSet<ClassName>>,
    super_classes: HashMap<ClassName, ClassName>,
}

/// A class hierarchy based on interface implementations.
#[derive(Debug, Clone)]
pub struct InterfaceImplHierarchy {
    #[cfg_attr(
        not(feature = "petgraph"),
        expect(dead_code, reason = "used by petgraph traversal")
    )]
    implementations: HashMap<ClassName, HashSet<ClassName>>,
    #[cfg_attr(
        not(feature = "petgraph"),
        expect(dead_code, reason = "used by petgraph traversal")
    )]
    implementers: HashMap<ClassName, HashSet<ClassName>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::class_loader::class_paths::{DirectoryClassPath, NopClassPath};

    struct MissingClass;

    impl ClassNames for MissingClass {
        fn class_names(&self) -> Result<HashSet<ClassName>, ClassPathError> {
            Ok(["example/Missing".parse().unwrap()].into())
        }
    }

    impl ClassPath for MissingClass {
        fn find_class(&self, _: &ClassName) -> Result<Class, ClassPathError> {
            Err(ClassPathError::NotFound)
        }
    }

    #[test]
    fn context_reports_class_loading_failure() {
        let result = ResolutionContext::new([MissingClass], NopClassPath::EMPTY);
        assert!(
            matches!(result, Err(InitError::Load { name, source: ClassPathError::NotFound }) if name == "example/Missing")
        );
    }

    #[test]
    fn context_reports_discovery_failure() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml/missing");
        let result = ResolutionContext::new([DirectoryClassPath::new(path)], NopClassPath::EMPTY);
        assert!(matches!(result, Err(InitError::Discovery(_))));
    }
}
