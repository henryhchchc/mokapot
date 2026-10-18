use proptest::prelude::*;

use super::*;

fn assert_version_matches_spec(major: u16, minor: u16) {
    let valid = major == 45
        || ((46..=Version::LATEST.major()).contains(&major)
            && (minor == 0 || major >= 56 && minor == u16::MAX));
    let expected = if valid {
        Ok((major, minor, major >= 56 && minor == u16::MAX))
    } else {
        Err(crate::jvm::errors::ParseErrorKind::Malformed)
    };
    let actual = Version::from_versions(major, minor)
        .map(|version| {
            (
                version.major(),
                version.minor(),
                version.is_preview_enabled(),
            )
        })
        .map_err(|error| error.kind());
    assert_eq!(actual, expected, "version {major}.{minor}");
}

#[test]
fn class_bytes_reject_invalid_magic() {
    let reader = &[0, 0, 0, 0][..];
    let error = Class::from_bytes(reader).unwrap_err();
    assert_eq!(error.kind(), crate::jvm::errors::ParseErrorKind::Malformed);
}

proptest! {
    #[test]
    fn class_version_acceptance_matches_spec(
        (major, minor) in prop_oneof![
            (Just(45), any::<u16>()),
            (46..=Version::LATEST.major(), Just(0)),
            (56..=Version::LATEST.major(), Just(u16::MAX)),
            (any::<u16>(), any::<u16>()),
        ]
    ) {
        assert_version_matches_spec(major, minor);
    }
}

#[test]
fn class_version_boundaries_match_spec() {
    for major in [
        44,
        45,
        46,
        55,
        56,
        Version::LATEST.major(),
        Version::LATEST.major() + 1,
    ] {
        for minor in [0, 1, u16::MAX - 1, u16::MAX] {
            assert_version_matches_spec(major, minor);
        }
    }
}

#[test]
fn class_predicates_select_their_access_flags() {
    for (flag, abstract_class, interface) in [
        (AccessFlags::empty(), false, false),
        (AccessFlags::ABSTRACT, true, false),
        (AccessFlags::INTERFACE, false, true),
    ] {
        let class = Class {
            access_flags: AccessFlags::PUBLIC | flag,
            ..Default::default()
        };
        assert_eq!(class.is_abstract(), abstract_class);
        assert_eq!(class.is_interface(), interface);
    }
}

#[test]
fn miri_method_handle_reference_kinds_match_jvm_tags() {
    let owner = crate::types::reference_type::ReferenceType::Class("Example".parse().unwrap());
    let field = FieldRef {
        owner: owner.clone(),
        name: "value".into(),
        field_type: "I".parse().unwrap(),
    };
    let method = MethodRef {
        owner,
        name: "method".into(),
        descriptor: "()V".parse().unwrap(),
    };
    for (handle, expected) in [
        (MethodHandle::RefGetField(field.clone()), 1),
        (MethodHandle::RefGetStatic(field.clone()), 2),
        (MethodHandle::RefPutField(field.clone()), 3),
        (MethodHandle::RefPutStatic(field), 4),
        (MethodHandle::RefInvokeVirtual(method.clone()), 5),
        (MethodHandle::RefInvokeStatic(method.clone()), 6),
        (MethodHandle::RefInvokeSpecial(method.clone()), 7),
        (MethodHandle::RefNewInvokeSpecial(method.clone()), 8),
        (MethodHandle::RefInvokeInterface(method), 9),
    ] {
        assert_eq!(handle.reference_kind(), expected);
    }
}
