//! Shared validation for names in JVM internal form.

use crate::jvm::JavaStr;

pub(super) fn validate_internal_name(name: &JavaStr) -> Result<(), String> {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.split(|byte| *byte == b'/').any(<[u8]>::is_empty) {
        return Err(format!("'{name}' cannot contain empty components"));
    }
    if let Some(&character) = bytes
        .iter()
        .find(|&&byte| matches!(byte, b'.' | b';' | b'['))
    {
        return Err(format!(
            "'{name}' cannot contain '{}'",
            char::from(character)
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::validate_internal_name;
    use crate::jvm::{JavaStr, JavaString};

    #[test]
    fn rejects_invalid_internal_names() {
        for name in [
            "",
            "/java/lang",
            "java/lang/",
            "java//lang",
            "java.lang",
            "java;lang",
            "java/[lang",
        ] {
            let name = JavaString::from(name.to_owned());
            assert!(
                validate_internal_name(name.as_java_str()).is_err(),
                "accepted {name:?}"
            );
        }
    }

    #[test]
    fn accepts_jvm_names_beyond_source_identifiers() {
        for name in ["123/a name", "module-info", "class", "$", "日本語/🦀"] {
            let name = JavaString::from(name.to_owned());
            assert!(
                validate_internal_name(name.as_java_str()).is_ok(),
                "rejected {name:?}"
            );
        }
        let unpaired = JavaStr::from_cesu8(&[b'x', 0xed, 0xa0, 0x80]).unwrap();
        assert!(validate_internal_name(unpaired).is_ok());
    }
}
