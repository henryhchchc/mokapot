//! Named packages in JVM internal form.
#![doc = see_jvm_spec!(4, 2, 3)]

use std::str::FromStr;

use super::{class_name::ClassName, name_validation::validate_internal_name};
use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
};

/// A named package in JVM internal form, such as `java/lang`.
///
/// ```
/// use mokapot::types::package_name::PackageName;
///
/// let package: PackageName = "java/lang".parse().unwrap();
/// assert_eq!(package.as_java_str(), "java/lang");
/// assert_eq!(package.jls_name(), "java.lang");
/// ```
#[doc = see_jvm_spec!(4, 2, 3)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageName(JavaString);

/// An invalid package name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid package name: {_0}")]
pub struct InvalidPackageName(String);

impl PackageName {
    pub(super) fn from_class_name(name: &ClassName) -> Option<Self> {
        let bytes = name.as_java_str().as_bytes();
        let end = bytes.iter().rposition(|byte| *byte == b'/')?;
        let package = JavaString::from_cesu8(bytes[..end].to_vec())
            .expect("a class name prefix is valid modified UTF-8");
        Some(Self(package))
    }

    /// Creates a named package in JVM internal form.
    ///
    /// # Errors
    /// Returns [`InvalidPackageName`] for empty components or `.`, `;`, or `[`.
    pub fn new(name: impl Into<JavaString>) -> Result<Self, InvalidPackageName> {
        let name = name.into();
        validate_internal_name(name.as_java_str()).map_err(InvalidPackageName)?;
        Ok(Self(name))
    }

    /// Returns the package name in JVM internal form.
    #[must_use]
    pub const fn as_java_str(&self) -> &JavaStr {
        self.0.as_java_str()
    }

    /// Returns the dotted package name used by the JLS.
    #[must_use]
    pub fn jls_name(&self) -> JavaString {
        self.as_java_str().replace_ascii(b'/', b'.')
    }
}

impl AsRef<JavaStr> for PackageName {
    fn as_ref(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_java_str(), f)
    }
}

impl FromStr for PackageName {
    type Err = InvalidPackageName;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::new(JavaString::from(name))
    }
}

impl TryFrom<&JavaStr> for PackageName {
    type Error = InvalidPackageName;

    fn try_from(name: &JavaStr) -> Result<Self, Self::Error> {
        Self::new(name.to_owned())
    }
}

impl PartialEq<&str> for PackageName {
    fn eq(&self, other: &&str) -> bool {
        self.as_java_str() == *other
    }
}

impl PartialEq<PackageName> for &str {
    fn eq(&self, other: &PackageName) -> bool {
        other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_preserves_modified_utf8() {
        let bytes = [b'p', b'/', 0xed, 0xa0, 0x80];
        let encoded = JavaStr::from_cesu8(&bytes).unwrap();
        let package = PackageName::try_from(encoded).unwrap();
        assert_eq!(package.as_java_str().as_bytes(), bytes);
    }
}
