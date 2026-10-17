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
fn class_is_abstract() {
    let class = Class {
        access_flags: AccessFlags::PUBLIC | AccessFlags::ABSTRACT,
        ..Default::default()
    };
    assert!(class.is_abstract());

    let class = Class {
        access_flags: AccessFlags::PUBLIC,
        ..Default::default()
    };
    assert!(!class.is_abstract());
}

#[test]
fn class_is_interface() {
    let class = Class {
        access_flags: AccessFlags::PUBLIC | AccessFlags::INTERFACE,
        ..Default::default()
    };
    assert!(class.is_interface());

    let class = Class {
        access_flags: AccessFlags::PUBLIC,
        ..Default::default()
    };
    assert!(!class.is_interface());
}
