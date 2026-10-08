//! Type hierarchy analysis components.

#[cfg_attr(not(feature = "unstable-project-analyses"), expect(unused_imports))]
use std::collections::{HashMap, HashSet};

#[cfg(feature = "petgraph")]
#[cfg_attr(not(feature = "unstable-project-analyses"), expect(unused_imports))]
use petgraph::visit::{Control, DfsEvent, Reversed, depth_first_search};

#[cfg_attr(not(feature = "unstable-project-analyses"), expect(unused_imports))]
use super::{ClassHierarchy, InterfaceImplHierarchy};
#[cfg_attr(not(feature = "unstable-project-analyses"), expect(unused_imports))]
use crate::{jvm::Class, types::class_name::ClassName};

#[cfg(feature = "petgraph")]
mod petgraph_impl;

#[instability::unstable(feature = "project-analyses")]
impl ClassHierarchy {
    /// Creates a new [`ClassHierarchy`] from a list of classes.
    #[must_use]
    pub fn from_classes<'a, I>(classes: I) -> Self
    where
        I: IntoIterator<Item = &'a Class>,
    {
        let mut inheritance: HashMap<ClassName, HashSet<ClassName>> = HashMap::new();
        let mut super_classes: HashMap<ClassName, ClassName> = HashMap::new();
        for class in classes {
            if let Some(ref super_class) = class.super_class {
                inheritance
                    .entry(super_class.clone())
                    .or_default()
                    .insert(class.name.clone());
                super_classes.insert(class.name.clone(), super_class.clone());
            }
        }
        Self {
            inheritance,
            super_classes,
        }
    }

    /// Returns the set of super classes of the given class.
    #[must_use]
    pub fn super_classes(&self, class_name: &ClassName) -> HashSet<ClassName> {
        let mut super_classes = HashSet::new();
        let mut current = class_name;
        while let Some(super_class) = self.super_classes.get(current) {
            super_classes.insert(super_class.clone());
            current = super_class;
        }
        super_classes
    }

    /// Returns the set of subclasses of the given class.
    #[must_use]
    #[cfg(feature = "petgraph")]
    pub fn subclasses(&self, class_name: &ClassName) -> HashSet<ClassName> {
        let mut subclasses = HashSet::new();
        depth_first_search(self, [class_name], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                subclasses.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        subclasses.remove(class_name);
        subclasses.into_iter().cloned().collect()
    }
}

#[instability::unstable(feature = "project-analyses")]
impl InterfaceImplHierarchy {
    /// Creates a new [`InterfaceImplHierarchy`] from a list of classes.
    #[must_use]
    pub fn from_classes<'a, I>(classes: I) -> Self
    where
        I: IntoIterator<Item = &'a Class>,
    {
        let mut implementations: HashMap<ClassName, HashSet<ClassName>> = HashMap::new();
        let mut implementers: HashMap<ClassName, HashSet<ClassName>> = HashMap::new();
        for class in classes {
            for interface in &class.interfaces {
                implementations
                    .entry(class.name.clone())
                    .or_default()
                    .insert(interface.clone());
                implementers
                    .entry(interface.clone())
                    .or_default()
                    .insert(class.name.clone());
            }
        }
        Self {
            implementations,
            implementers,
        }
    }

    /// Returns the set of interfaces implemented by the given class.
    #[must_use]
    #[cfg(feature = "petgraph")]
    pub fn implemented_interfaces(&self, class_name: &ClassName) -> HashSet<ClassName> {
        let mut interfaces = HashSet::new();
        depth_first_search(self, [class_name], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                interfaces.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        interfaces.remove(class_name);
        interfaces.into_iter().cloned().collect()
    }

    /// Returns the set of classes that implement the given interface.
    #[must_use]
    #[cfg(feature = "petgraph")]
    pub fn implementers(&self, interface_name: &ClassName) -> HashSet<ClassName> {
        let mut implementers = HashSet::new();
        let rev_impl_graph = Reversed(self);
        depth_first_search(&rev_impl_graph, [interface_name], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                implementers.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        implementers.remove(interface_name);
        implementers.into_iter().cloned().collect()
    }
}
