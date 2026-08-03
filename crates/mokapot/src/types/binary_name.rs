//! JVM binary name type.
//!
//! A binary name is the internal form of a class or interface name,
//! using `/` as the separator (e.g., `java/lang/String`, `java/util/Map$Entry`).
//! It is distinct from a *descriptor* (`Ljava/lang/String;`) and from a
//! *qualified name* (`java.lang.String`).
//!
#![doc = see_jvm_spec!(4, 2, 1)]
//!
//! # Validation
//!
//! `BinaryName` enforces that the string:
//! - is non-empty,
//! - contains no empty segments (no `//` or trailing `/`),
//! - does not start with `[` (an array descriptor indicator), and
//! - does not contain `;` (a descriptor terminator).
//!
//! These checks catch the common mistake of using a descriptor where a binary name
//! is expected, or vice versa.

use std::{fmt, ops::Deref, str::FromStr};

use crate::intrinsics::see_jvm_spec;

/// A validated JVM binary name.
///
/// # Examples
///
/// ```
/// use mokapot::types::binary_name::BinaryName;
///
/// let name = "java/lang/String".parse::<BinaryName>().unwrap();
/// assert_eq!(name.as_str(), "java/lang/String");
/// assert_eq!(name.to_qualified_name(), "java.lang.String");
/// ```
#[doc = see_jvm_spec!(4, 2, 1)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinaryName(Box<str>);

/// An error indicating that a string is not a valid JVM binary name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid binary name: {_0}")]
pub struct InvalidBinaryName(String);

impl BinaryName {
    /// Creates a new `BinaryName` from a string, validating it.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidBinaryName`] if the string does not satisfy the binary name invariants.
    #[doc = see_jvm_spec!(4, 2, 1)]
    pub fn new(name: impl Into<Box<str>>) -> Result<Self, InvalidBinaryName> {
        let name: Box<str> = name.into();
        validate(&name)?;
        Ok(Self(name))
    }

    /// Returns the qualified name (`.`-separated form).
    ///
    /// # Examples
    ///
    /// ```
    /// use mokapot::types::binary_name::BinaryName;
    ///
    /// let name = "java/lang/String".parse::<BinaryName>().unwrap();
    /// assert_eq!(name.to_qualified_name(), "java.lang.String");
    /// ```
    #[must_use]
    pub fn to_qualified_name(&self) -> String {
        self.0.replace('/', ".")
    }

    /// Returns the package portion of the binary name, if any.
    ///
    /// Returns `None` for classes in the default package.
    ///
    /// # Examples
    ///
    /// ```
    /// use mokapot::types::binary_name::BinaryName;
    ///
    /// let name = "java/lang/String".parse::<BinaryName>().unwrap();
    /// assert_eq!(name.package().map(|p| p.to_string()), Some("java/lang".to_string()));
    ///
    /// let name = "MyClass".parse::<BinaryName>().unwrap();
    /// assert_eq!(name.package(), None);
    /// ```
    #[must_use]
    pub fn package(&self) -> Option<Self> {
        self.0.rsplit_once('/').map(|(pkg, _)| Self(pkg.into()))
    }

    /// Returns the simple name of the class (the last segment).
    ///
    /// # Examples
    ///
    /// ```
    /// use mokapot::types::binary_name::BinaryName;
    ///
    /// let name = "java/lang/String".parse::<BinaryName>().unwrap();
    /// assert_eq!(name.simple_name(), "String");
    /// ```
    #[must_use]
    pub fn simple_name(&self) -> &str {
        self.0.rsplit_once('/').map_or(&self.0, |(_, name)| name)
    }
}

impl Deref for BinaryName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&str> for BinaryName {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl PartialEq<BinaryName> for &str {
    fn eq(&self, other: &BinaryName) -> bool {
        *self == other.0.as_ref()
    }
}

impl AsRef<str> for BinaryName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BinaryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for BinaryName {
    type Err = InvalidBinaryName;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Validate a binary name string.
fn validate(name: &str) -> Result<(), InvalidBinaryName> {
    if name.is_empty() {
        return Err(InvalidBinaryName("binary name must not be empty".into()));
    }
    if name.starts_with('[') {
        return Err(InvalidBinaryName(format!(
            "'{name}' starts with '['; binary names are class names, not array descriptors"
        )));
    }
    if name.contains(';') {
        return Err(InvalidBinaryName(format!(
            "'{name}' contains ';'; binary names are class names, not descriptors"
        )));
    }
    // Check for empty segments: leading/trailing/double slash
    if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
        return Err(InvalidBinaryName(format!(
            "'{name}' contains empty segments (leading, trailing, or double '/')"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_binary_names() {
        assert!("java/lang/String".parse::<BinaryName>().is_ok());
        assert!("java/util/Map$Entry".parse::<BinaryName>().is_ok());
        assert!("MyClass".parse::<BinaryName>().is_ok());
        assert!("org/mokapot/test/MyClass".parse::<BinaryName>().is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!("".parse::<BinaryName>().is_err());
    }

    #[test]
    fn rejects_array_descriptor() {
        assert!("[Ljava/lang/String;".parse::<BinaryName>().is_err());
        assert!("[I".parse::<BinaryName>().is_err());
    }

    #[test]
    fn rejects_class_descriptor() {
        assert!("Ljava/lang/String;".parse::<BinaryName>().is_err());
    }

    #[test]
    fn rejects_empty_segments() {
        assert!("/Foo".parse::<BinaryName>().is_err());
        assert!("Foo/".parse::<BinaryName>().is_err());
        assert!("Foo//Bar".parse::<BinaryName>().is_err());
    }

    #[test]
    fn qualified_name() {
        let name = "java/lang/String".parse::<BinaryName>().unwrap();
        assert_eq!(name.to_qualified_name(), "java.lang.String");
    }

    #[test]
    fn simple_name() {
        let name = "java/lang/String".parse::<BinaryName>().unwrap();
        assert_eq!(name.simple_name(), "String");

        let name = "MyClass".parse::<BinaryName>().unwrap();
        assert_eq!(name.simple_name(), "MyClass");
    }

    #[test]
    fn package() {
        let name = "java/lang/String".parse::<BinaryName>().unwrap();
        assert_eq!(
            name.package().map(|p| p.to_string()),
            Some("java/lang".to_string())
        );

        let name = "MyClass".parse::<BinaryName>().unwrap();
        assert_eq!(name.package(), None);
    }
}
