use proptest::prelude::*;

use super::*;

proptest! {
    #[test]
    fn jdk_1_1(minor in any::<u16>()) {
        let class_version = Version::from_versions(45, minor).unwrap();
        assert_eq!(45, class_version.major());
        assert_eq!(minor, class_version.minor());
    }

    #[test]
    fn jdk_1_x(major in 46u16..56) {
        let class_version = Version::from_versions(major, 0).unwrap();
        assert_eq!(major, class_version.major());
        assert!(!class_version.is_preview_enabled());
    }

    #[test]
    fn jdk_1_x_invalid(major in 46u16..56, minor in 1u16..) {
        assert!(Version::from_versions(major, minor).is_err());
    }

    #[test]
    fn newer_class_versions(
        major in 56..=MAX_MAJOR_VERSION,
        minor in prop_oneof![Just(0u16), Just(u16::MAX)],
    ) {
        let class_version = Version::from_versions(major, minor).unwrap();
        assert_eq!(major, class_version.major());
        assert_eq!(class_version.is_preview_enabled(), class_version.minor() == u16::MAX);
    }

    #[test]
    fn too_low_class_version(major in 0u16..45) {
        assert!(Version::from_versions(major, 0).is_err());
    }

    #[test]
    fn too_high_class_version(major in (MAX_MAJOR_VERSION + 1)..=u16::MAX) {
        assert!(Version::from_versions(major, 0).is_err());
    }

    #[test]
    fn invalid_class_version(major in 46..=MAX_MAJOR_VERSION, minor in 1..u16::MAX) {
        assert!(Version::from_versions(major, minor).is_err());
    }
}

fn arb_access_flag() -> impl Strategy<Value = AccessFlags> {
    prop_oneof![
        Just(AccessFlags::PUBLIC),
        Just(AccessFlags::PRIVATE),
        Just(AccessFlags::FINAL),
        Just(AccessFlags::SUPER),
        Just(AccessFlags::INTERFACE),
        Just(AccessFlags::ABSTRACT),
        Just(AccessFlags::SYNTHETIC),
        Just(AccessFlags::ANNOTATION),
        Just(AccessFlags::ENUM),
        Just(AccessFlags::MODULE),
    ]
}

proptest! {
    #[test]
    fn access_flags_bit_no_overlap(lhs in arb_access_flag(), rhs in arb_access_flag()) {
        prop_assume!(lhs != rhs);
        assert_eq!(lhs.bits() & rhs.bits(), 0);
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
