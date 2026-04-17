use elda_recipe::{GitHubReleaseAssetDefinition, ScalarValue, SourceDefinition};

use crate::BuildError;

pub(super) fn materialize_binary_source(
    source: &SourceDefinition,
    package_arch: Option<&str>,
) -> Result<SourceDefinition, BuildError> {
    if source.kind != "github_release" || source.github_release_assets.is_empty() {
        return Ok(source.clone());
    }

    let package_arch = package_arch.ok_or_else(|| {
        BuildError::Invalid(
            "binary source recipe is missing a canonical architecture for asset selection"
                .to_owned(),
        )
    })?;
    let asset = source
        .github_release_assets
        .get(package_arch)
        .ok_or_else(|| {
            BuildError::Invalid(format!(
                "github_release source does not define an asset for architecture `{package_arch}`"
            ))
        })?;

    Ok(SourceDefinition::single_lane_with_assets(
        source.kind.clone(),
        merged_fields(source, asset),
        source.github_release_assets.clone(),
    ))
}

fn merged_fields(
    source: &SourceDefinition,
    asset: &GitHubReleaseAssetDefinition,
) -> std::collections::BTreeMap<String, ScalarValue> {
    let mut fields = source.fields.clone();
    fields.insert("asset".to_owned(), ScalarValue::String(asset.asset.clone()));
    fields.insert(
        "sha256".to_owned(),
        ScalarValue::String(asset.sha256.clone()),
    );
    merge_optional_string(&mut fields, "binary", asset.binary.as_deref());
    merge_optional_integer(&mut fields, "strip_components", asset.strip_components);
    merge_optional_string(&mut fields, "subdir", asset.subdir.as_deref());
    merge_optional_string(&mut fields, "rename", asset.rename.as_deref());
    fields
}

fn merge_optional_string(
    fields: &mut std::collections::BTreeMap<String, ScalarValue>,
    key: &str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        fields.insert(key.to_owned(), ScalarValue::String(value.to_owned()));
    }
}

fn merge_optional_integer(
    fields: &mut std::collections::BTreeMap<String, ScalarValue>,
    key: &str,
    value: Option<i64>,
) {
    if let Some(value) = value {
        fields.insert(key.to_owned(), ScalarValue::Integer(value));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::materialize_binary_source;
    use elda_recipe::{GitHubReleaseAssetDefinition, ScalarValue, SourceDefinition};

    #[test]
    fn github_release_assets_override_top_level_binary_fields_for_selected_arch() {
        let mut fields = BTreeMap::new();
        fields.insert(
            "repo".to_owned(),
            ScalarValue::String("Mjoyufull/fsel".to_owned()),
        );
        fields.insert("tag".to_owned(), ScalarValue::String("v1.0.0".to_owned()));
        fields.insert(
            "binary".to_owned(),
            ScalarValue::String("default-bin".to_owned()),
        );
        let mut assets = BTreeMap::new();
        assets.insert(
            "amd64".to_owned(),
            GitHubReleaseAssetDefinition {
                asset: "fsel-x86_64.tar.xz".to_owned(),
                sha256: "abc123".to_owned(),
                binary: Some("fsel".to_owned()),
                strip_components: Some(1),
                subdir: None,
                rename: Some("fsel-renamed".to_owned()),
            },
        );

        let source =
            SourceDefinition::single_lane_with_assets("github_release".to_owned(), fields, assets);
        let materialized =
            materialize_binary_source(&source, Some("amd64")).expect("asset should resolve");

        assert_eq!(
            materialized.fields.get("asset"),
            Some(&ScalarValue::String("fsel-x86_64.tar.xz".to_owned()))
        );
        assert_eq!(
            materialized.fields.get("sha256"),
            Some(&ScalarValue::String("abc123".to_owned()))
        );
        assert_eq!(
            materialized.fields.get("binary"),
            Some(&ScalarValue::String("fsel".to_owned()))
        );
        assert_eq!(
            materialized.fields.get("rename"),
            Some(&ScalarValue::String("fsel-renamed".to_owned()))
        );
        assert_eq!(
            materialized.fields.get("strip_components"),
            Some(&ScalarValue::Integer(1))
        );
    }

    #[test]
    fn github_release_assets_fail_when_selected_arch_is_missing() {
        let source = SourceDefinition::single_lane_with_assets(
            "github_release".to_owned(),
            BTreeMap::new(),
            BTreeMap::from([(
                "arm64".to_owned(),
                GitHubReleaseAssetDefinition {
                    asset: "fsel-aarch64.tar.xz".to_owned(),
                    sha256: "abc123".to_owned(),
                    binary: None,
                    strip_components: None,
                    subdir: None,
                    rename: None,
                },
            )]),
        );

        let error =
            materialize_binary_source(&source, Some("amd64")).expect_err("arch should fail");
        assert!(
            error
                .to_string()
                .contains("does not define an asset for architecture `amd64`")
        );
    }
}
