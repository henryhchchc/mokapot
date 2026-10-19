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
