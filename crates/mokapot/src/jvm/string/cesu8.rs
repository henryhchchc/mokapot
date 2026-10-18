use std::{borrow::Cow, iter::Peekable};

use super::JStrDecodeError;

pub(super) fn as_str(bytes: &[u8]) -> Result<&str, JStrDecodeError> {
    let text = str::from_utf8(bytes).map_err(|_| JStrDecodeError)?;
    if needs_encoding(text) {
        return Err(JStrDecodeError);
    }
    Ok(text)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Cow<'_, str>, JStrDecodeError> {
    if let Ok(text) = as_str(bytes) {
        return Ok(Cow::Borrowed(text));
    }
    let mut text = String::with_capacity(bytes.len());
    for ch in chars(bytes) {
        text.push(ch.map_err(|_| JStrDecodeError)?);
    }
    Ok(Cow::Owned(text))
}

pub(super) fn encode(text: &str) -> Cow<'_, [u8]> {
    if !needs_encoding(text) {
        return Cow::Borrowed(text.as_bytes());
    }
    let start = text
        .bytes()
        .position(|byte| byte == 0 || byte >= 0xf0)
        .expect("text requires JVM encoding");
    let mut bytes = Vec::with_capacity(text.len());
    bytes.extend_from_slice(&text.as_bytes()[..start]);
    for unit in text[start..].encode_utf16() {
        match unit {
            1..=0x7f => bytes.push((unit & 0x7f) as u8),
            0..=0x7ff => {
                bytes.push(0xc0 | ((unit >> 6) & 0x1f) as u8);
                bytes.push(0x80 | (unit & 0x3f) as u8);
            }
            _ => {
                bytes.push(0xe0 | (unit >> 12) as u8);
                bytes.push(0x80 | ((unit >> 6) & 0x3f) as u8);
                bytes.push(0x80 | (unit & 0x3f) as u8);
            }
        }
    }
    Cow::Owned(bytes)
}

fn needs_encoding(text: &str) -> bool {
    text.bytes().fold(0, |encoded, byte| {
        encoded | u8::from(byte == 0 || byte >= 0xf0)
    }) != 0
}

#[derive(Clone, Copy, Debug)]
pub(super) enum DecodeError<'a> {
    InvalidEncoding(&'a [u8]),
    UnpairedSurrogate(u16),
}

pub(super) fn chars(bytes: &[u8]) -> impl Iterator<Item = Result<char, DecodeError<'_>>> + Clone {
    Chars {
        units: Units { bytes }.peekable(),
    }
}

#[derive(Clone)]
struct Units<'a> {
    bytes: &'a [u8],
}

impl<'a> Iterator for Units<'a> {
    type Item = Result<u16, &'a [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        let &first = self.bytes.first()?;
        let (width, mut unit) = match first {
            1..=0x7f => (1, u16::from(first)),
            0xc0 | 0xc2..=0xdf => (2, u16::from(first & 0x1f)),
            0xe0..=0xef => (3, u16::from(first & 0x0f)),
            _ => {
                let (invalid, remaining) = self.bytes.split_at(1);
                self.bytes = remaining;
                return Some(Err(invalid));
            }
        };
        for index in 1..width {
            let valid = self.bytes.get(index).is_some_and(|&byte| {
                (0x80..=0xbf).contains(&byte)
                    && (index != 1
                        || match first {
                            0xc0 => byte == 0x80,
                            0xe0 => byte >= 0xa0,
                            _ => true,
                        })
            });
            if !valid {
                let (invalid, remaining) = self.bytes.split_at(index);
                self.bytes = remaining;
                return Some(Err(invalid));
            }
            unit = (unit << 6) | u16::from(self.bytes[index] & 0x3f);
        }
        self.bytes = &self.bytes[width..];
        Some(Ok(unit))
    }
}

#[derive(Clone)]
struct Chars<'a> {
    units: Peekable<Units<'a>>,
}

impl<'a> Iterator for Chars<'a> {
    type Item = Result<char, DecodeError<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let unit = match self.units.next()? {
            Ok(unit) => unit,
            Err(bytes) => return Some(Err(DecodeError::InvalidEncoding(bytes))),
        };
        let scalar = match unit {
            0xd800..=0xdbff => {
                let Some(&Ok(low @ 0xdc00..=0xdfff)) = self.units.peek() else {
                    return Some(Err(DecodeError::UnpairedSurrogate(unit)));
                };
                self.units.next();
                0x10000 + ((u32::from(unit) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
            }
            0xdc00..=0xdfff => return Some(Err(DecodeError::UnpairedSurrogate(unit))),
            _ => u32::from(unit),
        };
        Some(Ok(char::from_u32(scalar).expect("decoded Unicode scalar")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn encoding_boundaries_match_jvm_spec() {
        for (text, bytes) in [
            ("", b"".as_slice()),
            ("\0", &[0xc0, 0x80]),
            ("\u{1}\u{7f}", &[0x01, 0x7f]),
            ("\u{80}", &[0xc2, 0x80]),
            ("\u{7ff}", &[0xdf, 0xbf]),
            ("\u{800}", &[0xe0, 0xa0, 0x80]),
            ("\u{d7ff}", &[0xed, 0x9f, 0xbf]),
            ("\u{e000}", &[0xee, 0x80, 0x80]),
            ("\u{ffff}", &[0xef, 0xbf, 0xbf]),
            ("\u{10000}", &[0xed, 0xa0, 0x80, 0xed, 0xb0, 0x80]),
            ("\u{10ffff}", &[0xed, 0xaf, 0xbf, 0xed, 0xbf, 0xbf]),
        ] {
            assert_eq!(encode(text).as_ref(), bytes);
            assert_eq!(decode(bytes).as_deref(), Ok(text));
        }
    }

    #[test]
    fn rejects_malformed_encodings_and_unpaired_surrogates() {
        for bytes in [
            &[0][..],
            &[0xf0, 0x90, 0x80, 0x80],
            &[0xff],
            &[0x80],
            &[0xc0, 0x81],
            &[0xc1, 0xbf],
            &[0xe0, 0x80, 0x80],
            &[0xc0],
            &[0xc2],
            &[0xe0, 0xa0],
            &[0xc2, b'a'],
            &[0xe1, 0x80, b'a'],
            &[0xed, 0xa0, 0x80],
            &[0xed, 0xb0, 0x80],
            &[0xed, 0xb0, 0x80, 0xed, 0xa0, 0x80],
            &[0xed, 0xa0, 0x80, 0xed, 0xa0, 0x80],
        ] {
            assert_eq!(decode(bytes), Err(JStrDecodeError), "{bytes:x?}");
            let mixed = [b"prefix", bytes, &[0xc0, 0x80], b"suffix"].concat();
            assert_eq!(decode(&mixed), Err(JStrDecodeError), "{mixed:x?}");
        }
    }

    proptest! {
        #[test]
        fn text_round_trips(text: String) {
            let bytes = encode(&text);
            let decoded = decode(&bytes);
            prop_assert_eq!(decoded.as_deref(), Ok(text.as_str()));
        }

        #[test]
        fn accepted_bytes_are_canonical(bytes: Vec<u8>) {
            if let Ok(text) = decode(&bytes) {
                let encoded = encode(&text);
                prop_assert_eq!(encoded.as_ref(), bytes.as_slice());
            }
        }
    }
}
