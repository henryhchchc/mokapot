//! Strings in JVM modified UTF-8 encoding.

use std::{
    borrow::{Borrow, Cow},
    fmt,
    ops::Deref,
    sync::Arc,
};

use derive_more::Display;
use simd_cesu8::mutf8;

/// A borrowed JVM modified UTF-8 string, which may contain unpaired surrogates.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct JavaStr {
    bytes: [u8],
}

impl JavaStr {
    /// Views bytes as a JVM modified UTF-8 string.
    ///
    /// # Errors
    /// Returns an error if the bytes are not valid JVM modified UTF-8.
    pub fn from_cesu8(bytes: &[u8]) -> Result<&Self, Cesu8Error> {
        validate_cesu8(bytes)?;
        // SAFETY: The bytes have been validated above.
        Ok(unsafe { Self::from_cesu8_unchecked(bytes) })
    }

    /// Views bytes as a JVM modified UTF-8 string without validation.
    ///
    /// # Safety
    /// The bytes must be valid JVM modified UTF-8. Unpaired surrogates are allowed.
    #[must_use]
    pub const unsafe fn from_cesu8_unchecked(bytes: &[u8]) -> &Self {
        // SAFETY: JavaStr is transparent over [u8], and the caller guarantees validity.
        unsafe { &*(std::ptr::from_ref(bytes) as *const Self) }
    }

    /// Returns the original JVM-encoded bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Decodes the string as UTF-8.
    ///
    /// # Errors
    /// Returns an error if the string contains unpaired surrogates.
    pub fn to_str(&self) -> Result<Cow<'_, str>, JavaStrDecodeError> {
        mutf8::decode_strict(&self.bytes).map_err(|_| JavaStrDecodeError)
    }

    /// Encodes Rust text as a JVM modified UTF-8 string.
    #[must_use]
    #[expect(clippy::should_implement_trait, reason = "Different return type")]
    pub fn from_str(s: &str) -> Cow<'_, JavaStr> {
        match mutf8::encode(s) {
            // SAFETY: `mutf8::encode` returns a borrowed slice if `s` is valid modified UTF-8.
            Cow::Borrowed(bytes) => Cow::Borrowed(unsafe { Self::from_cesu8_unchecked(bytes) }),
            Cow::Owned(bytes) => Cow::Owned(JavaString { bytes }),
        }
    }

    pub(crate) fn split_first_ascii(&self) -> Option<(u8, &Self)> {
        let (&first, rest) = self.bytes.split_first()?;
        if !first.is_ascii() {
            return None;
        }
        // SAFETY: Removing an ASCII byte preserves modified UTF-8 validity.
        Some((first, unsafe { Self::from_cesu8_unchecked(rest) }))
    }

    pub(crate) fn split_once_ascii(&self, separator: u8) -> Option<(&Self, &Self)> {
        assert!(separator.is_ascii());
        let index = self.bytes.iter().position(|&byte| byte == separator)?;
        let (before, after) = self.bytes.split_at(index);
        // SAFETY: ASCII separators cannot occur inside a multibyte sequence.
        Some(unsafe {
            (
                Self::from_cesu8_unchecked(before),
                Self::from_cesu8_unchecked(&after[1..]),
            )
        })
    }

    pub(crate) fn split_ascii(&self, separator: u8) -> impl DoubleEndedIterator<Item = &Self> {
        assert!(separator.is_ascii());
        self.bytes
            .split(move |&byte| byte == separator)
            .map(|bytes| {
                // SAFETY: ASCII separators cannot occur inside a multibyte sequence.
                unsafe { Self::from_cesu8_unchecked(bytes) }
            })
    }

    pub(crate) fn replace_ascii(&self, from: u8, to: u8) -> JavaString {
        assert!(from.is_ascii() && to.is_ascii() && from != 0 && to != 0);
        JavaString {
            bytes: self
                .bytes
                .iter()
                .map(|&byte| if byte == from { to } else { byte })
                .collect(),
        }
    }
}

impl ToOwned for JavaStr {
    type Owned = JavaString;

    fn to_owned(&self) -> Self::Owned {
        JavaString {
            bytes: self.bytes.to_vec(),
        }
    }
}

impl fmt::Debug for JavaStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(text) = self.to_str() {
            return fmt::Debug::fmt(&*text, f);
        }
        write!(f, "\"{}\"", self.bytes.escape_ascii())
    }
}

impl fmt::Display for JavaStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&*mutf8::decode_lossy_strict(&self.bytes), f)
    }
}

/// An error converting a Java string to UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Java string cannot be decoded as UTF-8")]
pub struct JavaStrDecodeError;

/// An error validating JVM modified UTF-8 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid JVM modified UTF-8 at byte {valid_up_to}")]
pub struct Cesu8Error {
    valid_up_to: usize,
    error_len: Option<usize>,
}

impl Cesu8Error {
    /// Returns the length of the valid prefix.
    #[must_use]
    pub const fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    /// Returns the invalid sequence's length, or `None` for an incomplete sequence.
    #[must_use]
    pub const fn error_len(&self) -> Option<usize> {
        self.error_len
    }
}

/// An error constructing a Java string, retaining the original bytes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{error}")]
pub struct FromCesu8Error {
    bytes: Vec<u8>,
    #[source]
    error: Cesu8Error,
}

impl FromCesu8Error {
    /// Returns the original bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Returns the original bytes, consuming the error.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the validation error.
    #[must_use]
    pub const fn cesu8_error(&self) -> Cesu8Error {
        self.error
    }
}

/// An owned JVM modified UTF-8 string, which may contain unpaired surrogates.
///
/// Rust string conversions encode text; byte constructors validate their input.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display("{}", self.as_java_str())]
pub struct JavaString {
    bytes: Vec<u8>,
}

impl JavaString {
    /// Creates an empty Java string.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Constructs a string from JVM modified UTF-8 bytes without copying them.
    ///
    /// # Errors
    /// Returns the original bytes and a validation error if the encoding is invalid.
    pub fn from_cesu8(bytes: Vec<u8>) -> Result<Self, FromCesu8Error> {
        match validate_cesu8(&bytes) {
            Ok(()) => Ok(Self { bytes }),
            Err(error) => Err(FromCesu8Error { bytes, error }),
        }
    }

    /// Constructs a string from JVM modified UTF-8 bytes without validation.
    ///
    /// # Safety
    /// The bytes must be valid JVM modified UTF-8. Unpaired surrogates are allowed.
    #[must_use]
    pub const unsafe fn from_cesu8_unchecked(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Borrows the string.
    #[must_use]
    pub const fn as_java_str(&self) -> &JavaStr {
        // SAFETY: Every JavaString constructor preserves valid JVM modified UTF-8.
        unsafe { JavaStr::from_cesu8_unchecked(self.bytes.as_slice()) }
    }

    /// Converts this [`JavaString`] into [`Box<JavaStr>`].
    #[must_use]
    pub fn into_boxed_java_str(self) -> Box<JavaStr> {
        let bytes = self.bytes.into_boxed_slice();
        // SAFETY: JavaStr is transparent over [u8], and the bytes are valid.
        unsafe { Box::from_raw(Box::into_raw(bytes) as *mut JavaStr) }
    }

    /// Returns the original JVM-encoded bytes, consuming the string.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Appends a Java string.
    pub fn push_java_str(&mut self, value: &JavaStr) {
        self.bytes.extend_from_slice(value.as_bytes());
    }

    /// Encodes and appends Rust text.
    pub fn push_str(&mut self, value: &str) {
        self.bytes.extend_from_slice(&mutf8::encode(value));
    }
}

impl Borrow<JavaStr> for JavaString {
    fn borrow(&self) -> &JavaStr {
        self.as_java_str()
    }
}

impl Deref for JavaString {
    type Target = JavaStr;

    fn deref(&self) -> &Self::Target {
        self.as_java_str()
    }
}

impl From<String> for JavaString {
    fn from(value: String) -> Self {
        match mutf8::encode(&value) {
            Cow::Borrowed(_) => Self {
                bytes: value.into_bytes(),
            },
            Cow::Owned(bytes) => Self { bytes },
        }
    }
}

impl From<&JavaStr> for Arc<JavaStr> {
    fn from(value: &JavaStr) -> Self {
        let bytes: Arc<[u8]> = Arc::from(value.as_bytes());
        // SAFETY: JavaStr is transparent over [u8], and the source guarantees
        // valid encoding. The allocation layout and reference counts are unchanged.
        unsafe { Self::from_raw(Arc::into_raw(bytes) as *const JavaStr) }
    }
}
impl PartialEq<str> for JavaStr {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == mutf8::encode(other).as_ref()
    }
}

impl PartialEq<&str> for JavaStr {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl PartialEq<str> for JavaString {
    fn eq(&self, other: &str) -> bool {
        self.as_java_str() == other
    }
}

impl PartialEq<&str> for JavaString {
    fn eq(&self, other: &&str) -> bool {
        self.as_java_str() == *other
    }
}

impl PartialEq<String> for JavaStr {
    fn eq(&self, other: &String) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<String> for JavaString {
    fn eq(&self, other: &String) -> bool {
        self.as_java_str() == other.as_str()
    }
}

impl fmt::Debug for JavaString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_java_str(), f)
    }
}

fn validate_cesu8(bytes: &[u8]) -> Result<(), Cesu8Error> {
    let mut valid_up_to = 0;
    while let Some(&first) = bytes.get(valid_up_to) {
        let error = |error_len| Cesu8Error {
            valid_up_to,
            error_len,
        };
        let width = match first {
            1..=0x7f => 1,
            0xc0 | 0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => return Err(error(Some(1))),
        };
        for index in 1..width {
            let Some(&byte) = bytes.get(valid_up_to + index) else {
                return Err(error(None));
            };
            let valid = (0x80..=0xbf).contains(&byte)
                && (index != 1
                    || match first {
                        0xc0 => byte == 0x80,
                        0xe0 => byte >= 0xa0,
                        _ => true,
                    });
            if !valid {
                return Err(error(Some(index)));
            }
        }
        valid_up_to += width;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_STRINGS: [&[u8]; 12] = [
        b"",
        b"text",
        b"/x/",
        b"a/\xed\xa0\x80/b",
        b"\xed\xa0\x80/\xc0\x80/",
        &[0xc0, 0x80],
        &[0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xed, 0xbf, 0xbf],
        &[0xed, 0xb0, 0x80, 0xed, 0xa0, 0x80],
        &[0xc2, 0x80, 0xdf, 0xbf],
        &[0xe0, 0xa0, 0x80, 0xed, 0x9f, 0xbf, 0xef, 0xbf, 0xbf],
    ];

    #[test]
    fn miri_split_first_ascii_preserves_encoding() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            if let Some((first, rest)) = view.split_first_ascii() {
                assert!(first.is_ascii());
                assert_eq!(bytes, [&[first], rest.as_bytes()].concat());
                assert!(JavaStr::from_cesu8(rest.as_bytes()).is_ok());
            } else {
                assert!(bytes.first().is_none_or(|byte| !byte.is_ascii()));
            }
        }
    }

    #[test]
    fn miri_split_once_ascii_preserves_encoding() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            if let Some((before, after)) = view.split_once_ascii(b'/') {
                assert_eq!(bytes, [before.as_bytes(), b"/", after.as_bytes()].concat());
                assert!(JavaStr::from_cesu8(before.as_bytes()).is_ok());
                assert!(JavaStr::from_cesu8(after.as_bytes()).is_ok());
            } else {
                assert!(!bytes.contains(&b'/'));
            }
        }
    }

    #[test]
    fn miri_split_ascii_preserves_encoding() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            for part in view.split_ascii(b'/') {
                assert!(JavaStr::from_cesu8(part.as_bytes()).is_ok());
            }
        }
    }

    #[test]
    fn replace_ascii_preserves_encoding() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            let replaced = view.replace_ascii(b'/', b'.');
            assert!(JavaStr::from_cesu8(replaced.as_bytes()).is_ok());
        }
    }

    #[test]
    fn miri_owned_strings_preserve_bytes() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            let owned = view.to_owned();
            let borrowed: &JavaStr = owned.borrow();
            assert_eq!(view.as_bytes(), bytes);
            assert_eq!(borrowed, view);
            assert_eq!(&*owned, view);
            assert_eq!(owned.into_bytes(), bytes);
        }
    }

    #[test]
    fn miri_owned_constructor_retains_buffer() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            let input = bytes.to_vec();
            let pointer = input.as_ptr();
            let checked = JavaString::from_cesu8(input).unwrap();
            assert_eq!(checked.as_bytes().as_ptr(), pointer);
            assert_eq!(checked.as_java_str(), view);
        }
    }

    #[test]
    fn miri_boxed_strings_preserve_bytes() {
        for bytes in VALID_STRINGS {
            let owned = JavaString::from_cesu8(bytes.to_vec()).unwrap();
            let boxed = owned.into_boxed_java_str();
            assert_eq!(boxed.as_bytes(), bytes);
        }
    }

    #[test]
    fn miri_shared_strings_preserve_bytes() {
        for bytes in VALID_STRINGS {
            let view = JavaStr::from_cesu8(bytes).unwrap();
            let shared = Arc::<JavaStr>::from(view);
            assert_eq!(shared.as_bytes(), bytes);
        }
    }

    #[test]
    fn debug_escapes_unpaired_surrogate_bytes() {
        let text = JavaStr::from_cesu8(&[0xed, 0xa0, 0x80]).unwrap();
        assert_eq!(format!("{text:?}"), r#""\xed\xa0\x80""#);
    }

    #[test]
    fn invalid_encoding_reports_location_and_retains_owned_bytes() {
        for (invalid, error_len) in [
            (&[0][..], Some(1)),
            (&[0x80], Some(1)),
            (&[0xc0, 0x81], Some(1)),
            (&[0xc1, 0xbf], Some(1)),
            (&[0xc2, b'a'], Some(1)),
            (&[0xe0, 0x80, 0x80], Some(1)),
            (&[0xe1, 0x80, b'a'], Some(2)),
            (&[0xf0, 0x90, 0x80, 0x80], Some(1)),
            (&[0xff], Some(1)),
            (&[0xc0], None),
            (&[0xc2], None),
            (&[0xe0, 0xa0], None),
        ] {
            let bytes = [b"prefix", invalid].concat();
            let error = JavaStr::from_cesu8(&bytes).unwrap_err();
            assert_eq!(error.valid_up_to(), 6);
            assert_eq!(error.error_len(), error_len);
            let expected = bytes.clone();
            let pointer = bytes.as_ptr();
            let owned_error = JavaString::from_cesu8(bytes).unwrap_err();
            assert_eq!(owned_error.cesu8_error(), error);
            assert_eq!(owned_error.as_bytes(), expected);
            assert_eq!(owned_error.as_bytes().as_ptr(), pointer);
            assert_eq!(owned_error.into_bytes(), expected);
        }
    }

    proptest::proptest! {
        #[test]
        fn validator_accepts_encoded_text(text: String) {
            let encoded = JavaString::from(text);
            proptest::prop_assert!(JavaStr::from_cesu8(encoded.as_bytes()).is_ok());
        }
    }
}
