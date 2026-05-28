#[test]
fn codeberg_target_parses_as_forgejo_provider() {
    let target = super::release_target::parse_release_target("https://codeberg.org/owner/tool")
        .expect("codeberg target should parse");
    assert_eq!(target.provider, "forgejo");
    assert_eq!(target.host, "codeberg.org");
    assert_eq!(target.repo, "owner/tool");
}

#[test]
fn explicit_forgejo_host_still_classifies_as_forgejo() {
    let target =
        super::release_target::parse_release_target("https://forgejo.example.invalid/team/tool")
            .expect("forgejo target should parse");
    assert_eq!(target.provider, "forgejo");
    assert_eq!(target.host, "forgejo.example.invalid");
    assert_eq!(target.repo, "team/tool");
}

#[test]
fn explicit_gitea_host_classifies_as_gitea() {
    let target =
        super::release_target::parse_release_target("https://gitea.example.invalid/team/tool")
            .expect("gitea target should parse");
    assert_eq!(target.provider, "gitea");
    assert_eq!(target.host, "gitea.example.invalid");
    assert_eq!(target.repo, "team/tool");
}

#[test]
fn github_release_option_stays_visible_but_does_not_select_binary_without_metadata_conversion() {
    let mut options = Vec::new();
    super::strategy::push_release_option(
        &mut options,
        &super::release_options::ReleaseOption {
            provider: "github".to_owned(),
            host: None,
            repo: "Mjoyufull/fsel".to_owned(),
            tag: "v3.4.1".to_owned(),
            asset: "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
            compatibility: "native-exact".to_owned(),
            sha256: None,
            signature: None,
        },
    );

    assert_eq!(options[0].strategy, "git_release");
    assert_eq!(options[0].lane, "binary");
    assert_eq!(options[0].tag.as_deref(), Some("v3.4.1"));
    assert_eq!(
        options[0].asset.as_deref(),
        Some("fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz")
    );
    assert!(!options[0].selected);
}

#[test]
fn release_summary_prefers_current_host_payload_over_sidecars() {
    let release = serde_json::json!({
        "tag_name": "v3.4.1",
        "assets": [
            { "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz.sha256" },
            { "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz" }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let asset = super::release_options::recommended_release_asset(&release, &[])
        .expect("native asset should be recommended");

    assert_eq!(
        asset.get("name").and_then(serde_json::Value::as_str),
        Some("fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz")
    );
    assert_eq!(
        asset
            .get("compatibility")
            .and_then(serde_json::Value::as_str),
        Some("native-exact")
    );
}

#[test]
fn release_summary_records_uppercase_sha256sums_sidecar() {
    let release = serde_json::json!({
        "tag_name": "v3.4.1",
        "assets": [
            {
                "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                "browser_download_url": "https://example.invalid/fsel.tar.gz"
            },
            {
                "name": "SHA256SUMS",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let option = super::release_options::release_option_from_summary(
        "github",
        None,
        "Mjoyufull/fsel",
        &release,
        &[],
    )
    .expect("uppercase SHA256SUMS sidecar should still match");

    assert_eq!(
        option.sha256.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
}

#[test]
fn release_summary_records_per_asset_sums_sidecar() {
    let release = serde_json::json!({
        "tag_name": "v1.2.3",
        "assets": [
            {
                "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                "browser_download_url": "https://example.invalid/tool.tar.gz"
            },
            {
                "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.sums",
                "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let option = super::release_options::release_option_from_summary(
        "github",
        None,
        "owner/tool",
        &release,
        &[],
    )
    .expect("per-asset .sums sidecar should match");

    assert_eq!(
        option.sha256.as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
}

#[test]
fn release_summary_records_checksum_sidecar_for_metadata_conversion() {
    let release = serde_json::json!({
        "tag_name": "v3.4.1",
        "assets": [
            {
                "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
                "browser_download_url": "https://example.invalid/fsel.tar.gz"
            },
            {
                "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz.sha256",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        ]
    });
    let release = super::release_options::classified_release_summary(release);
    let option = super::release_options::release_option_from_summary(
        "github",
        None,
        "Mjoyufull/fsel",
        &release,
        &[],
    )
    .expect("release option should exist");

    assert_eq!(option.repo, "Mjoyufull/fsel");
    assert_eq!(option.tag, "v3.4.1");
    assert_eq!(option.asset, "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz");
    assert_eq!(
        option.sha256.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
}

#[test]
fn github_release_strategy_renders_pinned_binary_metadata_when_checksum_exists() {
    let option = super::release_options::ReleaseOption {
        provider: "github".to_owned(),
        host: None,
        repo: "Mjoyufull/fsel".to_owned(),
        tag: "v3.4.1".to_owned(),
        asset: "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: None,
    };
    let metadata = super::metadata::GeneratedMetadata::default();
    let pkg_lua = super::render::render_pkg_lua(
        "fsel",
        Some("https://github.com/Mjoyufull/fsel"),
        &[],
        "normal",
        &super::strategy::SourceStrategy::GithubRelease(option),
        &metadata,
        None,
    );

    assert!(pkg_lua.contains(r#"kind = "github_release""#));
    assert!(pkg_lua.contains(r#"repo = "Mjoyufull/fsel""#));
    assert!(pkg_lua.contains(r#"tag = "v3.4.1""#));
    assert!(pkg_lua.contains(r#"asset = "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz""#));
    assert!(pkg_lua.contains(
        r#"sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa""#
    ));
    assert!(!pkg_lua.contains("url = \"https://github.com/Mjoyufull/fsel\""));
    assert!(!pkg_lua.contains("branch = \"main\""));
}

#[test]
fn github_appimage_release_strategy_renders_appimage_kind_and_derived_binary() {
    let option = super::release_options::ReleaseOption {
        provider: "github".to_owned(),
        host: None,
        repo: "owner/demo".to_owned(),
        tag: "v2.0.0".to_owned(),
        asset: "Demo-2.0.0-x86_64-unknown-linux-gnu.AppImage".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: None,
    };
    let metadata = super::metadata::GeneratedMetadata::default();
    let pkg_lua = super::render::render_pkg_lua(
        "demo",
        Some("https://github.com/owner/demo"),
        &[],
        "normal",
        &super::strategy::SourceStrategy::GithubRelease(option),
        &metadata,
        None,
    );

    assert!(pkg_lua.contains(r#"kind = "appimage""#));
    assert!(pkg_lua.contains(r#"binary = "demo-2.0.0-x86_64-unknown-linux-gnu""#));
    assert!(pkg_lua.contains(r#"asset = "Demo-2.0.0-x86_64-unknown-linux-gnu.AppImage""#));
}

#[test]
fn gitlab_release_summary_links_convert_to_release_option() {
    let release = serde_json::json!({
        "tag_name": "v1.2.3",
        "assets": [{
            "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
            "browser_download_url": "https://gitlab.com/owner/tool/-/releases/v1.2.3/downloads/tool.tar.gz",
            "digest": "sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        }]
    });
    let release = super::release_options::classified_release_summary(release);
    let option = super::release_options::release_option_from_summary(
        "gitlab",
        None,
        "owner/tool",
        &release,
        &[],
    )
    .expect("release option should exist");

    assert_eq!(option.provider, "gitlab");
    assert_eq!(option.source_kind(), "release_asset");
    assert_eq!(option.repo, "owner/tool");
    assert_eq!(option.tag, "v1.2.3");
    assert_eq!(
        option.sha256.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
}

#[test]
fn gitlab_release_strategy_renders_provider_neutral_metadata_when_checksum_exists() {
    let option = super::release_options::ReleaseOption {
        provider: "gitlab".to_owned(),
        host: None,
        repo: "owner/tool".to_owned(),
        tag: "v1.2.3".to_owned(),
        asset: "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: None,
    };
    let metadata = super::metadata::GeneratedMetadata::default();
    let pkg_lua = super::render::render_pkg_lua(
        "tool",
        Some("https://gitlab.com/owner/tool"),
        &[],
        "normal",
        &super::strategy::SourceStrategy::GithubRelease(option),
        &metadata,
        None,
    );

    assert!(pkg_lua.contains(r#"kind = "release_asset""#));
    assert!(pkg_lua.contains(r#"provider = "gitlab""#));
    assert!(pkg_lua.contains(r#"repo = "owner/tool""#));
    assert!(pkg_lua.contains(r#"tag = "v1.2.3""#));
    assert!(pkg_lua.contains(r#"asset = "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz""#));
}

#[test]
fn self_hosted_gitlab_release_strategy_renders_host_metadata() {
    let option = super::release_options::ReleaseOption {
        provider: "gitlab".to_owned(),
        host: Some("gitlab.example.invalid".to_owned()),
        repo: "team/tool".to_owned(),
        tag: "v1.2.3".to_owned(),
        asset: "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: None,
    };
    let metadata = super::metadata::GeneratedMetadata::default();
    let pkg_lua = super::render::render_pkg_lua(
        "tool",
        Some("https://gitlab.example.invalid/team/tool"),
        &[],
        "normal",
        &super::strategy::SourceStrategy::GithubRelease(option),
        &metadata,
        None,
    );

    assert!(pkg_lua.contains(r#"kind = "release_asset""#));
    assert!(pkg_lua.contains(r#"provider = "gitlab""#));
    assert!(pkg_lua.contains(r#"host = "gitlab.example.invalid""#));
    assert!(pkg_lua.contains(r#"repo = "team/tool""#));
}

#[test]
fn release_summary_uses_github_asset_digest_when_present() {
    let release = serde_json::json!({
        "tag_name": "v3.4.1",
        "assets": [{
            "name": "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz",
            "browser_download_url": "https://example.invalid/fsel.tar.gz",
            "digest": "sha256:BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"
        }]
    });
    let release = super::release_options::classified_release_summary(release);
    let option = super::release_options::release_option_from_summary(
        "github",
        None,
        "Mjoyufull/fsel",
        &release,
        &[],
    )
    .expect("release option should exist");

    assert_eq!(
        option.sha256.as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
}

#[test]
fn direct_release_manifest_converts_to_provider_neutral_option() {
    let release = serde_json::json!({
        "releases": [{
            "tag_name": "v1.2.3",
            "assets": [
                {
                    "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://example.invalid/tool.tar.gz",
                    "digest": "sha256:CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
                },
                {
                    "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.minisig",
                    "browser_download_url": "https://example.invalid/tool.tar.gz.minisig"
                }
            ]
        }]
    });
    let release = super::release_options::classified_release_summary(
        super::release_options::normalized_release_summary_for_test("direct", release),
    );
    let option = super::release_options::release_option_from_summary(
        "direct",
        None,
        "https://example.invalid/tool.elda-releases.json",
        &release,
        &[],
    )
    .expect("direct release option should exist");

    assert_eq!(option.provider, "direct");
    assert_eq!(option.source_kind(), "release_asset");
    assert_eq!(option.tag, "v1.2.3");
    assert_eq!(
        option.sha256.as_deref(),
        Some("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
    );
    assert_eq!(
        option.signature.as_deref(),
        Some("https://example.invalid/tool.tar.gz.minisig")
    );
}

#[test]
fn sourcehut_release_option_renders_provider_neutral_metadata() {
    let option = super::release_options::ReleaseOption {
        provider: "sourcehut".to_owned(),
        host: None,
        repo: "~chris/tool".to_owned(),
        tag: "v1.2.3".to_owned(),
        asset: "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        compatibility: "native-exact".to_owned(),
        sha256: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned()),
        signature: Some("tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.minisig".to_owned()),
    };
    let metadata = super::metadata::GeneratedMetadata::default();
    let pkg_lua = super::render::render_pkg_lua(
        "tool",
        Some("https://git.sr.ht/~chris/tool"),
        &[],
        "normal",
        &super::strategy::SourceStrategy::GithubRelease(option),
        &metadata,
        None,
    );

    assert!(pkg_lua.contains(r#"kind = "release_asset""#));
    assert!(pkg_lua.contains(r#"provider = "sourcehut""#));
    assert!(pkg_lua.contains(r#"repo = "~chris/tool""#));
    assert!(
        pkg_lua.contains(r#"signature = "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz.minisig""#)
    );
}
