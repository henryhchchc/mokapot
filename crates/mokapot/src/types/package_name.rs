//! Named packages in JVM internal form.
#![doc = see_jvm_spec!(4, 2, 3)]

use std::str::FromStr;

use derive_more::{AsRef, Display};

use super::{class_name::ClassName, name_validation::validate_internal_name};
use crate::intrinsics::see_jvm_spec;

/// A named package in JVM internal form, such as `java/lang`.
///
/// ```
/// use mokapot::types::package_name::PackageName;
///
/// let package: PackageName = "java/lang".parse().unwrap();
/// assert_eq!(package.as_str(), "java/lang");
/// assert_eq!(package.jls_name(), "java.lang");
/// ```
#[doc = see_jvm_spec!(4, 2, 3)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Display)]
#[as_ref(str)]
#[display("{_0}")]
pub struct PackageName(Box<str>);

/// An invalid package name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid package name: {_0}")]
pub struct InvalidPackageName(String);

impl PackageName {
    pub(super) fn from_class_name(name: &ClassName) -> Option<Self> {
        // Class name validation also validates its package prefix.
        name.as_str()
            .rsplit_once('/')
            .map(|(package, _)| Self(package.into()))
    }

    /// Creates a named package in JVM internal form.
    ///
    /// # Errors
    /// Returns [`InvalidPackageName`] for empty components or `.`, `;`, or `[`.
    pub fn new(name: impl Into<Box<str>>) -> Result<Self, InvalidPackageName> {
        let name = name.into();
        validate_internal_name(&name).map_err(InvalidPackageName)?;
        Ok(Self(name))
    }

    /// Returns the package name in JVM internal form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the dotted package name used by the JLS.
    #[must_use]
    pub fn jls_name(&self) -> String {
        self.0.replace('/', ".")
    }
}

impl FromStr for PackageName {
    type Err = InvalidPackageName;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::new(name)
    }
}

impl PartialEq<&str> for PackageName {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<PackageName> for &str {
    fn eq(&self, other: &PackageName) -> bool {
        *self == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::types::name_validation::tests::name_components;

    impl Arbitrary for PackageName {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            name_components()
                .prop_map(|components| PackageName::new(components.join("/")).unwrap())
                .boxed()
        }
    }

    proptest! {
        #[test]
        fn parsing_round_trip(name in any::<PackageName>()) {
            prop_assert_eq!(name.to_string().parse::<PackageName>(), Ok(name));
        }

        #[test]
        fn jls_name_preserves_components(components in name_components()) {
            let name = PackageName::new(components.join("/")).unwrap();
            prop_assert_eq!(name.jls_name(), components.join("."));
        }
    }
}
