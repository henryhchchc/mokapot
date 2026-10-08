//! Shared validation for names in JVM internal form.

pub(super) fn validate_internal_name(name: &str) -> Result<(), String> {
    if name.split('/').any(str::is_empty) {
        return Err(format!("'{name}' cannot contain empty components"));
    }
    if let Some(character) = name
        .chars()
        .find(|character| matches!(character, '.' | ';' | '['))
    {
        return Err(format!("'{name}' cannot contain '{character}'"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use proptest::prelude::*;

    use super::validate_internal_name;

    pub fn name_components() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop::collection::vec(
                any::<char>().prop_filter("An unqualified name character", |character| {
                    !matches!(character, '.' | ';' | '[' | '/')
                }),
                1..16,
            )
            .prop_map(|characters| characters.into_iter().collect()),
            1..5,
        )
    }

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
            "[I",
            "[Ljava/lang/String;",
            "Ljava/lang/String;",
        ] {
            assert!(validate_internal_name(name).is_err(), "accepted {name:?}");
        }
    }

    #[test]
    fn accepts_jvm_names_beyond_source_identifiers() {
        for name in [
            "java/lang/String",
            "java/util/Map$Entry",
            "123/name",
            "a name/another name",
            "module-info",
            "package-info",
            "class",
            "$",
            "日本語/名前",
            "emoji/🦀",
        ] {
            assert!(validate_internal_name(name).is_ok(), "rejected {name:?}");
        }
    }

    proptest! {
        #[test]
        fn rejects_empty_components(components in name_components(), position in any::<usize>()) {
            let mut components = components;
            let position = position % (components.len() + 1);
            components.insert(position, String::new());
            prop_assert!(validate_internal_name(&components.join("/")).is_err());
        }
    }
}
