//! APIs for static analysis.

use std::collections::HashSet;

use crate::jvm::references::ClassRef;

pub mod fixed_point;

/// A context for class resolution during analysis.
#[derive(Debug)]
pub struct ResolutionContext<P> {
    class_path: Vec<(P, ResolutionLevel)>,
}

/// The resolution level of a class path.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ResolutionLevel {
    /// The class path contains application classes.
    Application,
    /// The class path contains library classes.
    Library,
}

/// A trait that can provide an exhaustive list of [`ClassRef`].
pub trait ClassRefs {
    /// List all classes.
    fn class_refs(&self) -> HashSet<ClassRef>;
}

impl<P> ResolutionContext<P> {
    /// Create a new resolution context.
    #[must_use]
    pub fn new(class_path: Vec<(P, ResolutionLevel)>) -> Self {
        Self { class_path }
    }

    /// Get the available classes as [`ClassRef`] at the given [`ResolutionLevel`].
    #[must_use]
    pub fn class_refs(&self, level: ResolutionLevel) -> HashSet<ClassRef>
    where
        P: ClassRefs,
    {
        self.class_path
            .iter()
            .filter(|(_, l)| l == &level)
            .flat_map(|(cp, _)| cp.class_refs().into_iter())
            .collect()
    }
}
