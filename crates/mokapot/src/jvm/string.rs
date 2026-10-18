//! Strings in JVM modified UTF-8 encoding.

use std::{
    borrow::{Borrow, Cow},
    fmt::{self, Write},
    ops::Deref,
};

mod cesu8;

/// A borrowed JVM modified UTF-8 string, including preserved malformed bytes.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct JStr {
    bytes: [u8],
}

impl JStr {
    /// Views JVM-encoded bytes without decoding or validating them.
    #[must_use]
    pub const fn from_bytes(bytes: &[u8]) -> &Self {
        // SAFETY: JStr is transparent over [u8] and accepts every byte sequence.
        unsafe { &*(std::ptr::from_ref(bytes) as *const Self) }
    }

    /// Returns the original JVM-encoded bytes.
    #[must_use]
    pub const fn to_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Decodes the string as UTF-8.
    ///
    /// # Errors
    /// Returns an error for undecodable bytes or unpaired surrogates.
    pub fn to_str(&self) -> Result<Cow<'_, str>, JStrDecodeError> {
        cesu8::decode(&self.bytes)
    }
}

impl ToOwned for JStr {
    type Owned = JString;

    fn to_owned(&self) -> Self::Owned {
        JString {
            bytes: self.bytes.to_vec(),
        }
    }
}

impl fmt::Debug for JStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(text) = cesu8::as_str(&self.bytes) {
            return fmt::Debug::fmt(text, f);
        }
        f.write_char('"')?;
        for ch in cesu8::chars(&self.bytes) {
            match ch {
                Ok('\'') => f.write_char('\'')?,
                Ok(ch) => write!(f, "{}", ch.escape_debug())?,
                Err(cesu8::DecodeError::InvalidEncoding(bytes)) => {
                    for byte in bytes {
                        write!(f, "{}", byte.escape_ascii())?;
                    }
                }
                Err(cesu8::DecodeError::UnpairedSurrogate(unit)) => {
                    write!(f, "\\u{{{unit:x}}}")?;
                }
            }
        }
        f.write_char('"')
    }
}

impl fmt::Display for JStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(text) = cesu8::as_str(&self.bytes) {
            return fmt::Display::fmt(text, f);
        }
        let chars = cesu8::chars(&self.bytes)
            .map(|ch| ch.unwrap_or(char::REPLACEMENT_CHARACTER))
            .take(f.precision().unwrap_or(usize::MAX));
        let padding = f
            .width()
            .map_or(0, |width| width.saturating_sub(chars.clone().count()));
        let before = match f.align().unwrap_or(fmt::Alignment::Left) {
            fmt::Alignment::Left => 0,
            fmt::Alignment::Right => padding,
            fmt::Alignment::Center => padding / 2,
        };
        let fill = f.fill();
        for _ in 0..before {
            f.write_char(fill)?;
        }
        for ch in chars {
            f.write_char(ch)?;
        }
        for _ in before..padding {
            f.write_char(fill)?;
        }
        Ok(())
    }
}

/// An error converting a Java string to UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Java string cannot be decoded as UTF-8")]
pub struct JStrDecodeError;

/// An owned JVM modified UTF-8 string, including preserved malformed bytes.
///
/// Rust string conversions encode text; byte conversions preserve their input.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct JString {
    bytes: Vec<u8>,
}

impl JString {
    /// Creates an empty Java string.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Borrows the string.
    #[must_use]
    pub const fn as_j_str(&self) -> &JStr {
        JStr::from_bytes(self.bytes.as_slice())
    }

    /// Returns the original JVM-encoded bytes, consuming the string.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Borrow<JStr> for JString {
    fn borrow(&self) -> &JStr {
        self.as_j_str()
    }
}

impl AsRef<JStr> for JString {
    fn as_ref(&self) -> &JStr {
        self.as_j_str()
    }
}

impl Deref for JString {
    type Target = JStr;

    fn deref(&self) -> &Self::Target {
        self.as_j_str()
    }
}

impl From<Vec<u8>> for JString {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl From<String> for JString {
    fn from(value: String) -> Self {
        match cesu8::encode(&value) {
            Cow::Borrowed(_) => Self {
                bytes: value.into_bytes(),
            },
            Cow::Owned(bytes) => Self { bytes },
        }
    }
}

impl From<&str> for JString {
    fn from(value: &str) -> Self {
        Self {
            bytes: cesu8::encode(value).into_owned(),
        }
    }
}

impl From<&[u8]> for JString {
    fn from(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
        }
    }
}

impl From<&JStr> for JString {
    fn from(value: &JStr) -> Self {
        value.to_owned()
    }
}

impl fmt::Debug for JString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_j_str(), f)
    }
}

impl fmt::Display for JString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_j_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn miri_borrowed_and_owned_strings_preserve_bytes() {
        for bytes in [
            b"".as_slice(),
            b"text",
            &[0xc0, 0x80],
            &[0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80],
            &[0xed, 0xa0, 0x80],
            &[0xff],
        ] {
            let view = JStr::from_bytes(bytes);
            let owned = view.to_owned();
            let borrowed: &JStr = owned.borrow();
            assert_eq!(view.to_bytes(), bytes);
            assert_eq!(borrowed, view);
            assert_eq!(&*owned, view);
            assert_eq!(owned.into_bytes(), bytes);
        }
    }

    #[test]
    fn owned_keys_support_borrowed_lookup() {
        let values = HashMap::from([(JString::from("text"), 1), (JString::from(vec![0xff]), 2)]);
        assert_eq!(values.get(JStr::from_bytes(b"text")), Some(&1));
        assert_eq!(values.get(JStr::from_bytes(&[0xff])), Some(&2));
    }

    #[test]
    fn formatting_decodes_jvm_text_and_preserves_invalid_debug_bytes() {
        for (bytes, display, debug) in [
            (&[0xc0, 0x80][..], "\0", r#""\0""#),
            (&[0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80], "😀", r#""😀""#),
            (&[0xff], "�", r#""\xff""#),
            (&[0], "�", r#""\x00""#),
            (
                &[0xed, 0xa0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80],
                "�😀",
                r#""\u{d800}😀""#,
            ),
            (
                &[0xe1, 0x80, b'a', 0xc0, 0x80, 0xff],
                "�a\0�",
                r#""\xe1\x80a\0\xff""#,
            ),
        ] {
            let borrowed = JStr::from_bytes(bytes);
            let owned = borrowed.to_owned();
            assert_eq!(format!("{borrowed}"), display);
            assert_eq!(format!("{borrowed:?}"), debug);
            assert_eq!(format!("{owned}"), display);
            assert_eq!(format!("{owned:?}"), debug);
            assert_eq!(format!("{borrowed:8}"), format!("{display:8}"));
            assert_eq!(format!("{borrowed:*>8.2}"), format!("{display:*>8.2}"));
            assert_eq!(format!("{borrowed:*^7}"), format!("{display:*^7}"));
        }
    }

    #[test]
    fn utf8_conversion_preserves_jvm_encoding() {
        for (text, bytes) in [
            ("plain", b"plain".as_slice()),
            ("\0", &[0xc0, 0x80]),
            ("😀", &[0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80]),
        ] {
            let borrowed = JStr::from_bytes(bytes);
            let owned = JString::from(text);
            assert_eq!(borrowed.to_str().as_deref(), Ok(text));
            assert_eq!(owned.to_bytes(), bytes);
            assert_eq!(JString::from(text.to_owned()), owned);
            assert_eq!(owned.to_str().as_deref(), Ok(text));
        }
        for bytes in [&[0xed, 0xa0, 0x80][..], &[0xff]] {
            assert_eq!(JStr::from_bytes(bytes).to_str(), Err(JStrDecodeError));
        }
    }

    proptest::proptest! {
        #[test]
        fn text_formatting_matches_rust(text: String) {
            let encoded = JString::from(format!("\0{text}"));
            let expected = format!("\0{text}");
            proptest::prop_assert_eq!(format!("{encoded}"), expected.as_str());
            proptest::prop_assert_eq!(format!("{encoded:?}"), format!("{expected:?}"));
        }
    }
}
