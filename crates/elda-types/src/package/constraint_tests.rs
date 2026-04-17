use std::str::FromStr;

use super::{ConstraintOperator, ConstraintVersion, NamedConstraint, PackageVersion};

#[test]
fn dependency_constraint_parses_versioned_expression() {
    let constraint =
        NamedConstraint::parse_dependency("openssl>=3:3.2.1-1").expect("constraint should parse");

    assert_eq!(constraint.name, "openssl");
    assert_eq!(constraint.operator, Some(ConstraintOperator::GreaterEqual));
    assert_eq!(
        constraint.version,
        Some(ConstraintVersion::from_parts(3, "3.2.1", Some(1)))
    );
}

#[test]
fn provide_constraint_requires_equal_operator() {
    let error = NamedConstraint::parse_provide("libgl>=1")
        .expect_err("versioned provides should only allow `=`");

    assert!(error.to_string().contains("must use `=`"));
}

#[test]
fn version_without_pkgrel_matches_any_pkgrel_for_exact_operator() {
    let constraint =
        NamedConstraint::parse_dependency("openssl=3.2.1").expect("constraint should parse");
    let actual = PackageVersion::from_str("0:3.2.1-4").expect("version should parse");

    assert!(constraint.matches_version(&ConstraintVersion::from(&actual)));
}

#[test]
fn explicit_pkgrel_is_checked_when_present() {
    let constraint =
        NamedConstraint::parse_dependency("openssl>=3.2.1-3").expect("constraint should parse");
    let older = PackageVersion::from_str("0:3.2.1-2").expect("older version should parse");
    let newer = PackageVersion::from_str("0:3.2.1-4").expect("newer version should parse");

    assert!(!constraint.matches_version(&ConstraintVersion::from(&older)));
    assert!(constraint.matches_version(&ConstraintVersion::from(&newer)));
}
