#[test]
fn default_release_priority_prefers_tarball_over_appimage() {
    let release = serde_json::json!({
        "tag_name": "v1.0.0",
        "assets": [
            { "name": "tool-1.0.0-x86_64-unknown-linux-gnu.AppImage" },
            { "name": "tool-1.0.0-x86_64-unknown-linux-gnu.tar.gz" }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let asset = super::release_options::recommended_release_asset(&release, &[])
        .expect("tar.gz should win with default format priority");
    assert_eq!(
        asset.get("name").and_then(serde_json::Value::as_str),
        Some("tool-1.0.0-x86_64-unknown-linux-gnu.tar.gz")
    );
}

#[test]
fn custom_priority_can_rank_appimage_before_tarball() {
    let release = serde_json::json!({
        "tag_name": "v1.0.0",
        "assets": [
            { "name": "tool-1.0.0-x86_64-unknown-linux-gnu.AppImage" },
            { "name": "tool-1.0.0-x86_64-unknown-linux-gnu.tar.gz" }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let pri = vec!["app-image".to_owned(), "tar-gz".to_owned()];
    let asset = super::release_options::recommended_release_asset(&release, &pri)
        .expect("AppImage should win when listed first");
    assert_eq!(
        asset.get("name").and_then(serde_json::Value::as_str),
        Some("tool-1.0.0-x86_64-unknown-linux-gnu.AppImage")
    );
}

#[test]
fn omitting_appimage_from_priority_disables_appimage_auto_selection() {
    let release = serde_json::json!({
        "tag_name": "v1.0.0",
        "assets": [
            { "name": "tool-1.0.0-x86_64-unknown-linux-gnu.AppImage" }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let pri = vec!["tar-gz".to_owned(), "zip".to_owned()];
    assert!(
        super::release_options::recommended_release_asset(&release, &pri).is_none(),
        "AppImage-only release should not auto-select when app-image is not allowed"
    );
}

#[test]
fn release_binary_lane_can_reuse_nix_metadata_without_overwriting_it() {
    let option = super::release_options::ReleaseOption {
        provider: "github".to_owned(),
        host: None,
        repo: "owner/tool".to_owned(),
        tag: "v1.2.3".to_owned(),
        asset: "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: None,
    };
    let metadata = super::metadata::GeneratedMetadata {
        description: Some("Metadata from nix".to_owned()),
        licenses: vec!["MIT".to_owned()],
        upstream: Some("https://example.invalid/upstream".to_owned()),
        version: Some("1.2.3".to_owned()),
        ..super::metadata::GeneratedMetadata::default()
    };

    let binary_strategy = super::strategy::SourceStrategy::GithubRelease(option);
    let pkg_lua = super::render::render_pkg_lua_with_binary_lane(super::render::PkgLuaRender {
        recipe_name: "tool",
        source_url: Some("https://github.com/owner/tool"),
        legacy_pkgdeps: &[],
        recipe_kind: "normal",
        source_strategy: &super::strategy::SourceStrategy::NixFlake,
        binary_strategy: Some(&binary_strategy),
        default_lane: "binary",
        metadata: &metadata,
        git_ref: None,
    });

    assert!(pkg_lua.contains(r#"description = "Metadata from nix""#));
    assert!(pkg_lua.contains(r#"licenses = { "MIT" }"#));
    assert!(pkg_lua.contains(r#"upstream = "https://example.invalid/upstream""#));
    assert!(pkg_lua.contains(r#"default_lane = "binary""#));
    assert!(pkg_lua.contains(r#"source = {"#));
    assert!(pkg_lua.contains(r#"lanes = {"#));
    assert!(pkg_lua.contains(
        r#"source = {
        kind = "nix_flake""#
    ));
    assert!(pkg_lua.contains(
        r#"binary = {
        kind = "github_release""#
    ));
    assert!(pkg_lua.contains(r#"asset = "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz""#));
}

#[test]
fn release_binary_lane_can_be_added_beside_aur_metadata() {
    let option = super::release_options::ReleaseOption {
        provider: "gitlab".to_owned(),
        host: None,
        repo: "owner/tool".to_owned(),
        tag: "v1.2.3".to_owned(),
        asset: "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned()),
        signature: Some("tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.minisig".to_owned()),
    };
    let metadata = super::metadata::GeneratedMetadata {
        description: Some("Metadata from PKGBUILD".to_owned()),
        depends: vec!["glibc".to_owned()],
        ..super::metadata::GeneratedMetadata::default()
    };

    let binary_strategy = super::strategy::SourceStrategy::GithubRelease(option);
    let pkg_lua = super::render::render_pkg_lua_with_binary_lane(super::render::PkgLuaRender {
        recipe_name: "tool",
        source_url: Some("https://gitlab.com/owner/tool"),
        legacy_pkgdeps: &[],
        recipe_kind: "normal",
        source_strategy: &super::strategy::SourceStrategy::AurPkgbuild,
        binary_strategy: Some(&binary_strategy),
        default_lane: "source",
        metadata: &metadata,
        git_ref: None,
    });

    assert!(pkg_lua.contains(r#"description = "Metadata from PKGBUILD""#));
    assert!(pkg_lua.contains(r#"depends = { "glibc" }"#));
    assert!(pkg_lua.contains(r#"default_lane = "source""#));
    assert!(pkg_lua.contains(r#"kind = "aur_pkgbuild""#));
    assert!(pkg_lua.contains(r#"kind = "release_asset""#));
    assert!(pkg_lua.contains(r#"provider = "gitlab""#));
    assert!(
        pkg_lua.contains(r#"signature = "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.minisig""#)
    );
}
