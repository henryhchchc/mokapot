//! Type hierarchy analysis components.
use std::collections::{HashMap, HashSet};

#[cfg(feature = "petgraph")]
use petgraph::visit::{Control, DfsEvent, Reversed, depth_first_search};

use super::{ClassHierarchy, InterfaceImplHierarchy};
use crate::jvm::{Class, references::ClassRef};

#[cfg(feature = "petgraph")]
mod petgraph_impl;

impl ClassHierarchy {
    /// Creates a new [`ClassHierarchy`] from a list of classes.
    #[must_use]
    pub fn from_classes<'a, I>(classes: I) -> Self
    where
        I: IntoIterator<Item = &'a Class>,
    {
        let mut inheritance: HashMap<ClassRef, HashSet<ClassRef>> = HashMap::new();
        let mut super_classes: HashMap<ClassRef, ClassRef> = HashMap::new();
        for class in classes {
            if let Some(ref super_class) = class.super_class {
                inheritance
                    .entry(super_class.clone())
                    .or_default()
                    .insert(class.make_ref());
                super_classes.insert(class.make_ref(), super_class.clone());
            }
        }
        Self {
            inheritance,
            super_classes,
        }
    }

    /// Returns the set of super classes of the given class.
    #[must_use]
    pub fn super_classes(&self, class: &ClassRef) -> HashSet<ClassRef> {
        let mut super_classes = HashSet::new();
        let mut current = class;
        while let Some(super_class) = self.super_classes.get(current) {
            super_classes.insert(super_class.clone());
            current = super_class;
        }
        super_classes
    }

    /// Returns the set of subclasses of the given class.
    #[must_use]
    #[cfg(feature = "petgraph")]
    pub fn subclasses(&self, class: &ClassRef) -> HashSet<ClassRef> {
        let mut subclasses = HashSet::new();
        depth_first_search(self, [class], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                subclasses.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        subclasses.remove(class);
        subclasses.into_iter().cloned().collect()
    }
}

impl InterfaceImplHierarchy {
    /// Creates a new [`InterfaceImplHierarchy`] from a list of classes.
    #[must_use]
    pub fn from_classes<'a, I>(classes: I) -> Self
    where
        I: IntoIterator<Item = &'a Class>,
    {
        let mut implementations: HashMap<ClassRef, HashSet<ClassRef>> = HashMap::new();
        let mut implementers: HashMap<ClassRef, HashSet<ClassRef>> = HashMap::new();
        for class in classes {
            for interface in &class.interfaces {
                implementations
                    .entry(class.make_ref())
                    .or_default()
                    .insert(interface.clone());
                implementers
                    .entry(interface.clone())
                    .or_default()
                    .insert(class.make_ref());
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
    pub fn implemented_interfaces(&self, class: &ClassRef) -> HashSet<ClassRef> {
        let mut interfaces = HashSet::new();
        depth_first_search(self, [class], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                interfaces.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        interfaces.remove(class);
        interfaces.into_iter().cloned().collect()
    }

    /// Returns the set of classes that implement the given interface.
    #[must_use]
    #[cfg(feature = "petgraph")]
    pub fn implementers(&self, interface: &ClassRef) -> HashSet<ClassRef> {
        let mut implementers = HashSet::new();
        let rev_impl_graph = Reversed(self);
        depth_first_search(&rev_impl_graph, [interface], |event| {
            if let DfsEvent::TreeEdge(_, i) = event {
                implementers.insert(i);
            }
            if let DfsEvent::BackEdge(_, _) = event {
                return Control::<()>::Prune;
            }
            Control::<()>::Continue
        });
        implementers.remove(interface);
        implementers.into_iter().cloned().collect()
    }
}
