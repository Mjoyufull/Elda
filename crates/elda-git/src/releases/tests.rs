use super::github::{ReleaseAssetResponse, normalize_release_response};
use super::{
    AssetCompatibility, AssetFormat, AssetKind, ReleaseProvider, ReleaseTarget,
    classify_release_asset, parse_github_repo_target, parse_release_target,
};

#[test]
fn parses_github_repository_targets() {
    assert_eq!(
        parse_github_repo_target("Mjoyufull/fsel"),
        Some("Mjoyufull/fsel".to_owned())
    );
    assert_eq!(
        parse_github_repo_target("https://github.com/Mjoyufull/fsel.git"),
        Some("Mjoyufull/fsel".to_owned())
    );
    assert_eq!(
        parse_github_repo_target("git@github.com:Mjoyufull/fsel.git"),
        Some("Mjoyufull/fsel".to_owned())
    );
}

#[test]
fn parses_provider_release_targets() {
    let gitlab = parse_release_target("https://gitlab.com/group/subgroup/tool.git")
        .expect("gitlab target should parse");
    assert_eq!(gitlab.provider, ReleaseProvider::Gitlab);
    assert_eq!(gitlab.repo, "group/subgroup/tool");

    let codeberg = parse_release_target("https://codeberg.org/forgejo/forgejo")
        .expect("codeberg target should parse");
    assert_eq!(codeberg.provider, ReleaseProvider::Forgejo);
    assert_eq!(codeberg.host, "codeberg.org");
    assert_eq!(codeberg.repo, "forgejo/forgejo");

    let forgejo = parse_release_target("https://forgejo.example.invalid/team/tool")
        .expect("forgejo target should parse");
    assert_eq!(forgejo.provider, ReleaseProvider::Forgejo);
    assert_eq!(forgejo.host, "forgejo.example.invalid");
    assert_eq!(forgejo.repo, "team/tool");

    let gitea = parse_release_target("https://gitea.example.invalid/team/tool")
        .expect("gitea target should parse");
    assert_eq!(gitea.provider, ReleaseProvider::Gitea);
    assert_eq!(gitea.host, "gitea.example.invalid");
    assert_eq!(gitea.repo, "team/tool");
}

#[test]
fn normalizes_gitlab_release_links_into_assets() {
    let target = ReleaseTarget {
        provider: ReleaseProvider::Gitlab,
        host: "gitlab.com".to_owned(),
        repo: "group/subgroup/tool".to_owned(),
    };
    let releases = normalize_release_response(
        &target,
        serde_json::json!([{
            "tag_name": "v1.2.3",
            "name": "v1.2.3",
            "released_at": "2026-01-01T00:00:00Z",
            "assets": { "links": [{
                "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                "direct_asset_url": "https://gitlab.com/group/subgroup/tool/-/releases/v1.2.3/downloads/tool.tar.gz"
            }]}
        }]),
    )
    .expect("gitlab release should normalize");

    assert_eq!(releases[0].tag_name, "v1.2.3");
    assert_eq!(
        releases[0].assets[0].name,
        "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
    );
    assert_eq!(
        releases[0].assets[0].browser_download_url,
        "https://gitlab.com/group/subgroup/tool/-/releases/v1.2.3/downloads/tool.tar.gz"
    );
}

#[test]
fn classifies_native_linux_release_archive() {
    let asset = classify_release_asset(ReleaseAssetResponse {
        name: "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz".to_owned(),
        browser_download_url: "https://example.invalid/fsel.tar.gz".to_owned(),
    });

    assert_eq!(asset.kind, AssetKind::Payload);
    assert_eq!(asset.format, AssetFormat::TarGz);
    assert_eq!(asset.os.as_deref(), Some("linux"));
    assert_eq!(asset.arch.as_deref(), Some("x86_64"));
    assert_eq!(asset.compatibility, AssetCompatibility::NativeExact);
}

#[test]
fn classifies_checksum_as_sidecar() {
    let asset = classify_release_asset(ReleaseAssetResponse {
        name: "fsel-v3.4.1-x86_64-unknown-linux-gnu.tar.gz.sha256".to_owned(),
        browser_download_url: "https://example.invalid/fsel.sha256".to_owned(),
    });

    assert_eq!(asset.kind, AssetKind::Checksum);
    assert_eq!(asset.format, AssetFormat::Checksum);
    assert_eq!(asset.compatibility, AssetCompatibility::Sidecar);
    assert_eq!(asset.score, 0);
}

#[test]
fn parses_sourcehut_and_direct_release_targets() {
    let sourcehut = parse_release_target("https://git.sr.ht/~chris/tool")
        .expect("sourcehut target should parse");
    assert_eq!(sourcehut.provider, ReleaseProvider::Sourcehut);
    assert_eq!(sourcehut.repo, "~chris/tool");

    let direct = parse_release_target("https://example.invalid/tool.elda-releases.json")
        .expect("direct manifest target should parse");
    assert_eq!(direct.provider, ReleaseProvider::Direct);
    assert_eq!(
        direct.repo,
        "https://example.invalid/tool.elda-releases.json"
    );
}

#[test]
fn normalizes_sourcehut_tag_artifacts_into_release_assets() {
    let target = ReleaseTarget {
        provider: ReleaseProvider::Sourcehut,
        host: "git.sr.ht".to_owned(),
        repo: "~chris/tool".to_owned(),
    };
    let releases = normalize_release_response(
        &target,
        serde_json::json!({
            "data": { "repository": { "references": { "results": [{
                "name": "v1.2.3",
                "artifacts": { "results": [{
                    "filename": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "url": "https://git.sr.ht/~chris/tool/refs/download/v1.2.3/tool.tar.gz"
                }]}
            }]}}}
        }),
    )
    .expect("sourcehut release should normalize");

    assert_eq!(releases[0].tag_name, "v1.2.3");
    assert_eq!(
        releases[0].assets[0].name,
        "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz"
    );
}

#[test]
fn normalizes_direct_manifest_release_assets() {
    let target = ReleaseTarget {
        provider: ReleaseProvider::Direct,
        host: "example.invalid".to_owned(),
        repo: "https://example.invalid/tool.elda-releases.json".to_owned(),
    };
    let releases = normalize_release_response(
        &target,
        serde_json::json!({
            "releases": [{
                "tag_name": "v1.2.3",
                "assets": [{
                    "name": "tool-v1.2.3-x86_64-unknown-linux-gnu.tar.gz",
                    "browser_download_url": "https://example.invalid/tool.tar.gz"
                }]
            }]
        }),
    )
    .expect("direct release manifest should normalize");

    assert_eq!(releases[0].tag_name, "v1.2.3");
    assert_eq!(
        releases[0].assets[0].browser_download_url,
        "https://example.invalid/tool.tar.gz"
    );
}
