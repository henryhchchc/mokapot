//! Module names in class-file form.
#![doc = see_jvm_spec!(4, 2, 3)]

use std::str::FromStr;

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{JavaStr, JavaString},
};

/// A module name in class-file form, such as `java.base`.
///
/// ```
/// use mokapot::types::module_name::ModuleName;
///
/// let name: ModuleName = "java.base".parse().unwrap();
/// assert_eq!(name.as_java_str(), "java.base");
/// ```
#[doc = see_jvm_spec!(4, 2, 3)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleName(JavaString);

/// An invalid module name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Invalid module name: {_0}")]
pub struct InvalidModuleName(String);

impl ModuleName {
    /// Creates a module name in class-file form.
    ///
    /// # Errors
    /// Returns [`InvalidModuleName`] for control characters or invalid escapes.
    pub fn new(name: impl Into<JavaString>) -> Result<Self, InvalidModuleName> {
        let name = name.into();
        validate(name.as_java_str())?;
        Ok(Self(name))
    }

    /// Returns the module name in class-file form, preserving escapes.
    #[must_use]
    pub const fn as_java_str(&self) -> &JavaStr {
        self.0.as_java_str()
    }
}

impl AsRef<JavaStr> for ModuleName {
    fn as_ref(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl std::fmt::Display for ModuleName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_java_str(), f)
    }
}

impl FromStr for ModuleName {
    type Err = InvalidModuleName;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::new(JavaString::from(name.to_owned()))
    }
}

impl TryFrom<&JavaStr> for ModuleName {
    type Error = InvalidModuleName;

    fn try_from(name: &JavaStr) -> Result<Self, Self::Error> {
        Self::new(name.to_owned())
    }
}

impl PartialEq<&str> for ModuleName {
    fn eq(&self, other: &&str) -> bool {
        self.as_java_str() == *other
    }
}

impl PartialEq<ModuleName> for &str {
    fn eq(&self, other: &ModuleName) -> bool {
        other == self
    }
}

fn validate(name: &JavaStr) -> Result<(), InvalidModuleName> {
    let mut bytes = name.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            // C0 only occurs in validated modified UTF-8 as the start of NUL.
            0..=31 | 0xc0 | b':' | b'@' => return Err(InvalidModuleName(name.to_string())),
            b'\\' if !matches!(bytes.next(), Some(b'\\' | b':' | b'@')) => {
                return Err(InvalidModuleName(name.to_string()));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_module_name_encoding() {
        for name in ["java.base", r"a\:b", r"a\@b", r"a\\b", "日本語"] {
            assert!(name.parse::<ModuleName>().is_ok(), "{name:?}");
        }
        for name in ["a:b", "a@b", "a\\", r"a\x", "a\n", "a\0"] {
            assert!(name.parse::<ModuleName>().is_err(), "{name:?}");
        }
    }

    #[test]
    fn preserves_unpaired_surrogates() {
        let encoded = JavaStr::from_cesu8(&[b'm', 0xed, 0xa0, 0x80]).unwrap();
        let name = ModuleName::try_from(encoded).unwrap();
        assert_eq!(name.as_java_str(), encoded);
    }
}
