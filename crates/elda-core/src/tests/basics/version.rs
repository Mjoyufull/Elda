use crate::version::{codename, release_number, version_details, ELDA_VERSION};

#[test]
fn version_string_includes_sumomo_codename() {
    assert_eq!(ELDA_VERSION, "0.1.49-Sumomo");
    assert_eq!(release_number(), "0.1.49");
    assert_eq!(codename(), Some("Sumomo"));
}

#[test]
fn version_details_include_components_and_schemas() {
    let details = version_details();
    assert_eq!(
        details
            .get("elda_version")
            .and_then(|value| value.as_str()),
        Some("0.1.49-Sumomo")
    );
    assert!(details.get("components").and_then(|v| v.as_array()).is_some_and(
        |components| !components.is_empty()
    ));
    assert!(details.get("schemas").and_then(|v| v.as_object()).is_some());
}
