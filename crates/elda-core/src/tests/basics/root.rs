use super::*;

#[test]
fn cli_surface_contains_all_spec_namespaces() {
    let namespace_names = cli_surface()
        .iter()
        .map(|namespace| namespace.name)
        .collect::<Vec<_>>();

    assert!(namespace_names.contains(&"(root)"));
    assert!(namespace_names.contains(&"state"));
    assert!(namespace_names.contains(&"cache"));
    assert!(namespace_names.contains(&"qa"));
}

#[test]
fn version_examples_match_spec_ordering() {
    let newer = PackageVersion::from_str("1.10-1").expect("version should parse");
    let older = PackageVersion::from_str("1.9-1").expect("version should parse");

    assert!(newer > older);
}

#[test]
fn identity_examples_parse() {
    let identity =
        PackageIdentity::from_str("libfoo:i386 1:1.2.3-4").expect("identity should parse");

    assert_eq!(identity.to_string(), "libfoo:i386 1:1.2.3-4");
}

#[test]
fn ls_bootstraps_empty_root() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["ls".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("ls should succeed");

    assert_eq!(report.area, "state");
    assert!(
        report
            .details
            .and_then(|details| details.get("packages").cloned())
            .and_then(|packages| packages.as_array().cloned())
            .is_some_and(|packages| packages.is_empty())
    );
}

#[test]
fn state_show_reports_empty_world() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["state".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("state show should succeed");

    assert!(
        report
            .details
            .and_then(|details| details.get("world").cloned())
            .and_then(|world| world.as_array().cloned())
            .is_some_and(|world| world.is_empty())
    );
}
