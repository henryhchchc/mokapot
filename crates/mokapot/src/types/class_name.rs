//! Class and interface names in JVM internal form.
#![doc = see_jvm_spec!(4, 2, 1)]

use std::{str::FromStr, sync::Arc};

use derive_more::{AsRef, Display};

use super::{name_validation::validate_internal_name, package_name::PackageName};
use crate::intrinsics::see_jvm_spec;

/// A class or interface name in JVM internal form, such as `java/lang/String`.
///
/// # Examples
///
/// ```
/// use mokapot::types::class_name::ClassName;
///
/// let name: ClassName = "java/util/Map$Entry".parse().unwrap();
/// assert_eq!(name.as_str(), "java/util/Map$Entry");
/// assert_eq!(name.jls_name(), "java.util.Map$Entry");
/// assert_eq!(name.unqualified_name(), "Map$Entry");
/// assert_eq!(name.package().unwrap().as_str(), "java/util");
/// ```
#[doc = see_jvm_spec!(4, 2, 1)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, AsRef, Display)]
#[as_ref(str)]
#[display("{_0}")]
pub struct ClassName(Arc<str>);

/// An invalid class or interface name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid class name: {_0}")]
pub struct InvalidClassName(String);

impl ClassName {
    /// Creates a class or interface name in JVM internal form.
    ///
    /// # Errors
    /// Returns [`InvalidClassName`] for empty components or `.`, `;`, or `[`.
    #[doc = see_jvm_spec!(4, 2, 1)]
    pub fn new(name: impl Into<Box<str>>) -> Result<Self, InvalidClassName> {
        let name: Box<str> = name.into();
        validate_internal_name(&name).map_err(InvalidClassName)?;
        Ok(Self(name.into()))
    }

    /// Returns the name in JVM internal form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the dotted binary name used by the JLS.
    ///
    /// Preserves `$`, for example `java.util.Map$Entry`.
    #[must_use]
    pub fn jls_name(&self) -> String {
        self.0.replace('/', ".")
    }

    /// Returns the named package, or `None` for the unnamed package.
    #[must_use]
    pub fn package(&self) -> Option<PackageName> {
        PackageName::from_class_name(self)
    }

    /// Returns the unqualified name, preserving `$`, for example `Map$Entry`.
    #[must_use]
    pub fn unqualified_name(&self) -> &str {
        self.0.rsplit_once('/').map_or(&self.0, |(_, name)| name)
    }
}

impl PartialEq<&str> for ClassName {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<ClassName> for &str {
    fn eq(&self, other: &ClassName) -> bool {
        *self == other.0.as_ref()
    }
}

impl FromStr for ClassName {
    type Err = InvalidClassName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_internal_name(s).map_err(InvalidClassName)?;
        Ok(Self(Arc::from(s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name_validation::tests::name_components;
    use proptest::prelude::*;

    impl Arbitrary for ClassName {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            name_components()
                .prop_map(|segments| {
                    let name = segments.join("/");
                    ClassName::new(name).unwrap()
                })
                .boxed()
        }
    }

    proptest! {
        #[test]
        fn name_parts_preserve_components(components in name_components()) {
            let name = ClassName::new(components.join("/")).unwrap();
            let expected_package = (components.len() > 1)
                .then(|| components[..components.len() - 1].join("/"));

            prop_assert_eq!(name.unqualified_name(), components.last().unwrap());
            prop_assert_eq!(name.package().map(|package| package.to_string()), expected_package);
            prop_assert_eq!(name.jls_name(), components.join("."));
            prop_assert_eq!(name.to_string().parse::<ClassName>(), Ok(name));
        }
    }

    #[test]
    fn nested_names_preserve_binary_spelling() {
        let name: ClassName = "java/util/Map$Entry".parse().unwrap();
        assert_eq!(name.unqualified_name(), "Map$Entry");
        assert_eq!(name.jls_name(), "java.util.Map$Entry");
        let package = name.package().unwrap();
        assert_eq!(package, "java/util");
        assert_eq!(package.jls_name(), "java.util");
        assert!(ClassName::new("Map$Entry").unwrap().package().is_none());
    }
}
