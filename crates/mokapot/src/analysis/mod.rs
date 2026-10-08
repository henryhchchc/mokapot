//! APIs for static analysis.

use crate::types::class_name::ClassName;

use std::collections::{HashMap, HashSet};

use derive_more::Display;

use crate::jvm::Class;
#[cfg_attr(not(feature = "unstable-project-analyses"), expect(unused_imports))]
use crate::jvm::class_loader::ClassPath;

pub mod fixed_point;
pub mod type_hierarchy;

/// A context for class resolution during analysis.
#[derive(Debug)]
#[instability::unstable(feature = "project-analyses")]
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

/// A trait that can provide an exhaustive list of [`ClassName`].
#[instability::unstable(feature = "project-analyses")]
pub trait ClassNames {
    /// List all classes.
    fn class_names(&self) -> HashSet<ClassName>;
}

#[instability::unstable(feature = "project-analyses")]
impl ResolutionContext {
    /// Create a new resolution context.
    #[must_use]
    pub fn new<ACP, LCP>(app_class_path: ACP, lib_class_path: LCP) -> Self
    where
        ACP: IntoIterator<Item: ClassPath + ClassNames>,
        LCP: IntoIterator<Item: ClassPath + ClassNames>,
    {
        let application_classes = load_classes(app_class_path);
        let library_classes = load_classes(lib_class_path);
        let all_classes = application_classes.values().chain(library_classes.values());
        let class_hierarchy = ClassHierarchy::from_classes(all_classes.clone());
        let interface_implementations = InterfaceImplHierarchy::from_classes(all_classes);
        Self {
            application_classes,
            library_classes,
            class_hierarchy,
            interface_implementations,
        }
    }
}

/// An error that occurs during initialization of a [`ResolutionContext`].
#[derive(Debug, Display)]
#[instability::unstable(feature = "project-analyses")]
pub enum InitError {}

#[cfg(feature = "unstable-project-analyses")]
fn load_classes<CP>(class_path: CP) -> HashMap<ClassName, Class>
where
    CP: IntoIterator<Item: ClassPath + ClassNames>,
{
    class_path
        .into_iter()
        .flat_map(|cp| {
            cp.class_names()
                .into_iter()
                .map(move |class_name| {
                    cp.find_class(&class_name)
                        .expect("A class name yielded by the class path must be found.")
                })
                .map(|it| (it.name.clone(), it))
        })
        .collect()
}

/// A class hierarchy based on super class relationships.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct ClassHierarchy {
    #[cfg_attr(
        all(not(feature = "petgraph"), feature = "unstable-project-analyses"),
        expect(dead_code)
    )]
    inheritance: HashMap<ClassName, HashSet<ClassName>>,
    super_classes: HashMap<ClassName, ClassName>,
}

/// A class hierarchy based on interface implementations.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct InterfaceImplHierarchy {
    #[cfg_attr(
        all(not(feature = "petgraph"), feature = "unstable-project-analyses"),
        expect(dead_code)
    )]
    implementations: HashMap<ClassName, HashSet<ClassName>>,
    #[cfg_attr(
        all(not(feature = "petgraph"), feature = "unstable-project-analyses"),
        expect(dead_code)
    )]
    implementers: HashMap<ClassName, HashSet<ClassName>>,
}
