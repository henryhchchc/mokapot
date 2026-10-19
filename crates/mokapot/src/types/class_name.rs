//! Class and interface names in JVM internal form.
#![doc = see_jvm_spec!(4, 2, 1)]

use std::{str::FromStr, sync::Arc};

use super::{name_validation::validate_internal_name, package_name::PackageName};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
};

/// A class or interface name in JVM internal form, such as `java/lang/String`.
///
/// # Examples
///
/// ```
/// use mokapot::types::class_name::ClassName;
///
/// let name: ClassName = "java/util/Map$Entry".parse().unwrap();
/// assert_eq!(name.as_java_str(), "java/util/Map$Entry");
/// assert_eq!(name.jls_name(), "java.util.Map$Entry");
/// assert_eq!(name.unqualified_name(), "Map$Entry");
/// assert_eq!(name.package().unwrap().as_java_str(), "java/util");
/// ```
#[doc = see_jvm_spec!(4, 2, 1)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassName(Arc<JavaStr>);

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
    pub fn new(name: impl Into<JavaString>) -> Result<Self, InvalidClassName> {
        let name = name.into();
        validate_internal_name(name.as_java_str()).map_err(InvalidClassName)?;
        Ok(Self(Arc::from(name)))
    }

    /// Returns the name in JVM internal form.
    #[must_use]
    pub fn as_java_str(&self) -> &JavaStr {
        &self.0
    }

    /// Returns the dotted binary name used by the JLS.
    ///
    /// Preserves `$`, for example `java.util.Map$Entry`.
    #[must_use]
    pub fn jls_name(&self) -> JavaString {
        self.as_java_str().replace_ascii(b'/', b'.')
    }

    /// Returns the named package, or `None` for the unnamed package.
    #[must_use]
    pub fn package(&self) -> Option<PackageName> {
        PackageName::from_class_name(self)
    }

    /// Returns the unqualified name, preserving `$`, for example `Map$Entry`.
    #[must_use]
    pub fn unqualified_name(&self) -> &JavaStr {
        self.as_java_str()
            .split_ascii(b'/')
            .next_back()
            .unwrap_or(self.as_java_str())
    }
}

impl AsRef<JavaStr> for ClassName {
    fn as_ref(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl std::fmt::Display for ClassName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_java_str(), f)
    }
}

impl PartialEq<&str> for ClassName {
    fn eq(&self, other: &&str) -> bool {
        self.as_java_str() == *other
    }
}

impl PartialEq<ClassName> for &str {
    fn eq(&self, other: &ClassName) -> bool {
        other == self
    }
}

impl FromStr for ClassName {
    type Err = InvalidClassName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(JavaString::from(s))
    }
}

impl TryFrom<&JavaStr> for ClassName {
    type Error = InvalidClassName;

    fn try_from(name: &JavaStr) -> Result<Self, Self::Error> {
        validate_internal_name(name).map_err(InvalidClassName)?;
        Ok(Self(Arc::from(name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_class_names_with_unpaired_surrogates() {
        let name = JavaString::from_cesu8(vec![b'p', b'/', 0xed, 0xa0, 0x80]).unwrap();
        let name = ClassName::new(name).unwrap();
        assert_eq!(name.unqualified_name().as_bytes(), &[0xed, 0xa0, 0x80]);
        assert_eq!(
            name.jls_name().as_java_str().as_bytes(),
            &[b'p', b'.', 0xed, 0xa0, 0x80]
        );
        assert_eq!(name.package().unwrap().as_java_str().as_bytes(), b"p");
    }
}
