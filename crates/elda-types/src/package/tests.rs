use std::str::FromStr;

use super::{Architecture, PackageIdentity, PackageVersion};

#[test]
fn parses_version_without_explicit_epoch() {
    let version = PackageVersion::from_str("3.4.1-2").expect("version should parse");

    assert_eq!(version.epoch, 0);
    assert_eq!(version.pkgver, "3.4.1");
    assert_eq!(version.pkgrel, 2);
}

#[test]
fn parses_identity_with_architecture() {
    let identity =
        PackageIdentity::from_str("libfoo:i386 1:1.2.3-4").expect("identity should parse");

    assert_eq!(identity.name, "libfoo");
    assert_eq!(identity.arch, Some(Architecture::I386));
    assert_eq!(identity.version.epoch, 1);
}

#[test]
fn numeric_segments_sort_naturally() {
    let newer = PackageVersion::from_str("1.10-1").expect("newer version should parse");
    let older = PackageVersion::from_str("1.9-1").expect("older version should parse");

    assert!(newer > older);
}

#[test]
fn numeric_segments_sort_newer_than_alpha_runs() {
    let newer = PackageVersion::from_str("1.0.0-1").expect("newer version should parse");
    let older = PackageVersion::from_str("1.0.0rc1-1").expect("older version should parse");

    assert!(newer > older);
}

#[test]
fn epoch_beats_pkgver_and_pkgrel() {
    let newer = PackageVersion::from_str("1:1.0-1").expect("newer version should parse");
    let older = PackageVersion::from_str("0:9.9-999").expect("older version should parse");

    assert!(newer > older);
}

#[test]
fn pkgrel_breaks_ties_after_pkgver() {
    let newer = PackageVersion::from_str("1.2.0-2").expect("newer version should parse");
    let older = PackageVersion::from_str("1.2.0-1").expect("older version should parse");

    assert!(newer > older);
}

#[test]
fn missing_numeric_tail_sorts_older_than_present_numeric_tail() {
    let older = PackageVersion::from_str("1.0-1").expect("older version should parse");
    let newer = PackageVersion::from_str("1.0.1-1").expect("newer version should parse");

    assert!(older < newer);
}
