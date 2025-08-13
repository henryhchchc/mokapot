//! APIs for static analysis.

use std::collections::{HashMap, HashSet};

use crate::jvm::{Class, class_loader::ClassPath, references::ClassRef};

pub mod fixed_point;
pub mod type_hierarchy;

/// A context for class resolution during analysis.
#[derive(Debug)]
#[instability::unstable(feature = "project-analyses")]
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
#[instability::unstable(feature = "project-analyses")]
pub trait ClassRefs {
    /// List all classes.
    fn class_refs(&self) -> HashSet<ClassRef>;
}

impl ResolutionContext {
    /// Create a new resolution context.
    #[must_use]
    pub fn new<ACP, LCP>(app_class_path: ACP, lib_class_path: LCP) -> Self
    where
        ACP: IntoIterator<Item: ClassPath + ClassRefs>,
        LCP: IntoIterator<Item: ClassPath + ClassRefs>,
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
#[instability::unstable(feature = "project-analyses")]
pub enum InitError {}

fn load_classes<CP>(class_path: CP) -> HashMap<ClassRef, Class>
where
    CP: IntoIterator<Item: ClassPath + ClassRefs>,
{
    class_path
        .into_iter()
        .flat_map(|cp| {
            cp.class_refs()
                .into_iter()
                .map(move |cr| {
                    cp.find_class(&cr.binary_name)
                        .expect("Class ref yielded by the class path must be found.")
                })
                .map(|it| (it.make_ref(), it))
        })
        .collect()
}

/// A class hierarchy based on super class relationships.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct ClassHierarchy {
    inheritance: HashMap<ClassRef, HashSet<ClassRef>>,
    super_classes: HashMap<ClassRef, ClassRef>,
}

/// A class hierarchy based on interface implementations.
#[derive(Debug, Clone)]
#[instability::unstable(feature = "project-analyses")]
pub struct InterfaceImplHierarchy {
    implementations: HashMap<ClassRef, HashSet<ClassRef>>,
    implementers: HashMap<ClassRef, HashSet<ClassRef>>,
}
