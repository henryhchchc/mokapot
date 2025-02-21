//! APIs for static analysis.

use std::collections::{HashMap, HashSet};

use crate::{
    ir::{ClassHierarchy, InterfaceImplHierarchy},
    jvm::{Class, class_loader::ClassPath, references::ClassRef},
};

pub mod fixed_point;

/// A context for class resolution during analysis.
#[derive(Debug)]
pub struct ResolutionContext {
    /// The application classes.
    pub application_classes: HashMap<ClassRef, Class>,
    /// The library classes.
    pub library_classes: HashMap<ClassRef, Class>,
    /// The class hierarchy.
    pub class_hierarchy: ClassHierarchy,
    /// The interface implementations.
    pub interface_implementations: InterfaceImplHierarchy,
}

/// A trait that can provide an exhaustive list of [`ClassRef`].
pub trait ClassRefs {
    /// List all classes.
    fn class_refs(&self) -> HashSet<ClassRef>;
}

impl ResolutionContext {
    /// Create a new resolution context.
    #[must_use]
    pub fn new<P>(app_class_path: &[P], lib_class_path: &[P]) -> Self
    where
        P: ClassPath + ClassRefs,
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
#[derive(Debug, derive_more::Display)]
pub enum InitError {}

fn load_classes<P>(class_path: &[P]) -> HashMap<ClassRef, Class>
where
    P: ClassPath + ClassRefs,
{
    class_path
        .iter()
        .flat_map(|cp| {
            cp.class_refs()
                .into_iter()
                .map(|cr| {
                    cp.find_class(&cr.binary_name)
                        .expect("Class ref yielded by the class path must be found.")
                })
                .map(|it| (it.as_ref(), it))
        })
        .collect()
}
