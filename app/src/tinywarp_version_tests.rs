use super::*;

#[test]
fn compares_numeric_versions_and_dev_before_public_release() {
    for (incoming, current) in [
        ("v0.10.0", "v0.9.9"),
        ("v0.1.0", "v0.1.0-dev.99"),
        ("v0.1.0-dev.10", "v0.1.0-dev.2"),
        ("v1.0.0", "v0.99.99"),
    ] {
        assert!(newer(incoming, current).unwrap());
        assert!(!newer(current, incoming).unwrap());
    }
    assert!(!newer("v0.1.0", "v0.1.0").unwrap());
}

#[test]
fn rejects_noncanonical_unsafe_or_unsupported_versions() {
    for value in [
        "0.1.0",
        "v0.01.0",
        "v0.1.0-dev.0",
        "v0.1.0-dev.01",
        "v0.1.0-beta.1",
        "v0.1.0+build.2",
        "v0.1.0/../escape",
        "v0.2026.11.12.00.30.tinywarp_02",
    ] {
        assert!(parse(value).is_err(), "{value}");
    }
}

#[test]
fn identifies_only_retired_local_tinywarp_versions() {
    assert!(is_legacy("v0.2026.11.12.00.30.tinywarp_02"));
    assert!(!is_legacy("v0.1.0"));
    assert!(!is_legacy("v0.2026.11.12.00.30.stable_02"));
    assert!(!is_legacy("v0.2026.11.12.00.30.tinywarp_02/escape"));
}

#[test]
fn trusted_build_info_and_about_label_agree() {
    let base = Version::parse(&BUILD_INFO.version).unwrap();
    assert!(base.pre.is_empty() && base.build.is_empty());
    assert!(BUILD_INFO.build > 0);
    assert_eq!(
        display_label("v0.1.0"),
        format!("0.1.0（构建 {}）", BUILD_INFO.build)
    );
}
