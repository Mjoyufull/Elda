use elda_recipe::ScalarValue;

use crate::BuildError;

pub(super) fn resolve_release_asset_url(
    source: &elda_recipe::SourceDefinition,
) -> Result<String, BuildError> {
    let provider = release_provider(source)?;
    let repo = string_field(source, "repo")?;
    let host = release_host(source, provider);
    let asset = string_field(source, "asset")?;
    if let Some(tag) = string_field_optional(source, "tag") {
        return tagged_release_asset_url(provider, host, repo, tag, asset);
    }

    match string_field_optional(source, "release") {
        Some("latest") => latest_release_asset_url(provider, repo, asset),
        Some(other) => Err(BuildError::Invalid(format!(
            "{} `release` must be `latest` in the current build slice, got `{other}`",
            source.kind
        ))),
        None => Err(BuildError::Invalid(format!(
            "{} source requires `tag` or `release`",
            source.kind
        ))),
    }
}

fn release_provider(source: &elda_recipe::SourceDefinition) -> Result<&str, BuildError> {
    if source.kind == "github_release" {
        return Ok("github");
    }
    if source.kind == "appimage" {
        return match string_field_optional(source, "provider") {
            None => Ok("github"),
            Some(provider) => match provider {
                "github" | "gitlab" | "gitea" | "forgejo" | "sourcehut" | "direct" => Ok(provider),
                other => Err(BuildError::Unsupported(format!(
                    "appimage provider `{other}` is not implemented by the current build slice"
                ))),
            },
        };
    }
    match string_field_optional(source, "provider") {
        Some("github" | "gitlab" | "gitea" | "forgejo" | "sourcehut" | "direct") => {
            Ok(string_field(source, "provider")?)
        }
        Some(provider) => Err(BuildError::Unsupported(format!(
            "release_asset provider `{provider}` is not implemented by the current build slice"
        ))),
        None => Err(BuildError::Invalid(
            "release_asset source requires `provider`".to_owned(),
        )),
    }
}

fn tagged_release_asset_url(
    provider: &str,
    host: &str,
    repo: &str,
    tag: &str,
    asset: &str,
) -> Result<String, BuildError> {
    match provider {
        "github" => Ok(format!(
            "https://{host}/{repo}/releases/download/{tag}/{asset}"
        )),
        "gitlab" => Ok(format!(
            "https://{host}/{repo}/-/releases/{tag}/downloads/{asset}"
        )),
        "gitea" | "forgejo" => Ok(format!(
            "https://{host}/{repo}/releases/download/{tag}/{asset}"
        )),
        "sourcehut" => Ok(format!("https://{host}/{repo}/refs/download/{tag}/{asset}")),
        "direct" => Ok(format!(
            "{}/{tag}/{asset}",
            direct_manifest_base(repo).trim_end_matches('/')
        )),
        other => Err(BuildError::Unsupported(format!(
            "release_asset provider `{other}` is not implemented by the current build slice"
        ))),
    }
}

fn direct_manifest_base(repo: &str) -> String {
    repo.rsplit_once('/')
        .filter(|(_, file)| file.ends_with(".elda-releases.json"))
        .map(|(base, _)| base.to_owned())
        .unwrap_or_else(|| repo.to_owned())
}

fn release_host<'a>(source: &'a elda_recipe::SourceDefinition, provider: &str) -> &'a str {
    string_field_optional(source, "host").unwrap_or_else(|| default_release_host(provider))
}

fn default_release_host(provider: &str) -> &'static str {
    match provider {
        "github" => "github.com",
        "gitlab" => "gitlab.com",
        "forgejo" | "gitea" => "codeberg.org",
        "sourcehut" => "git.sr.ht",
        _ => "",
    }
}

fn latest_release_asset_url(provider: &str, repo: &str, asset: &str) -> Result<String, BuildError> {
    match provider {
        "github" => Ok(format!(
            "https://github.com/{repo}/releases/latest/download/{asset}"
        )),
        other => Err(BuildError::Invalid(format!(
            "release_asset provider `{other}` requires an explicit `tag` in the current build slice"
        ))),
    }
}

fn string_field<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Result<&'a str, BuildError> {
    string_field_optional(source, key).ok_or_else(|| {
        BuildError::Invalid(format!("source.kind `{}` is missing `{key}`", source.kind))
    })
}

fn string_field_optional<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Option<&'a str> {
    match source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use elda_recipe::{ScalarValue, SourceDefinition};

    use super::resolve_release_asset_url;

    fn release_source(provider: &str) -> SourceDefinition {
        SourceDefinition {
            kind: "release_asset".to_owned(),
            fields: BTreeMap::from([
                (
                    "provider".to_owned(),
                    ScalarValue::String(provider.to_owned()),
                ),
                (
                    "repo".to_owned(),
                    ScalarValue::String("owner/tool".to_owned()),
                ),
                ("tag".to_owned(), ScalarValue::String("v1.2.3".to_owned())),
                (
                    "asset".to_owned(),
                    ScalarValue::String("tool-linux-amd64.tar.gz".to_owned()),
                ),
                ("sha256".to_owned(), ScalarValue::String("x".to_owned())),
            ]),
            github_release_assets: BTreeMap::new(),
            default_lane: None,
            lanes: BTreeMap::new(),
        }
    }

    #[test]
    fn gitlab_release_asset_resolves_to_gitlab_download_url() {
        let source = release_source("gitlab");

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://gitlab.com/owner/tool/-/releases/v1.2.3/downloads/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn gitea_release_asset_resolves_to_codeberg_download_url() {
        let source = release_source("gitea");

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://codeberg.org/owner/tool/releases/download/v1.2.3/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn forgejo_release_asset_resolves_to_codeberg_download_url() {
        let source = release_source("forgejo");

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://codeberg.org/owner/tool/releases/download/v1.2.3/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn self_hosted_gitlab_release_asset_honors_host_field() {
        let mut source = release_source("gitlab");
        source.fields.insert(
            "host".to_owned(),
            ScalarValue::String("gitlab.example.invalid".to_owned()),
        );

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://gitlab.example.invalid/owner/tool/-/releases/v1.2.3/downloads/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn self_hosted_gitea_release_asset_honors_host_field() {
        let mut source = release_source("gitea");
        source.fields.insert(
            "host".to_owned(),
            ScalarValue::String("forgejo.example.invalid".to_owned()),
        );

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://forgejo.example.invalid/owner/tool/releases/download/v1.2.3/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn sourcehut_release_asset_resolves_to_tag_artifact_url() {
        let source = release_source("sourcehut");

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://git.sr.ht/owner/tool/refs/download/v1.2.3/tool-linux-amd64.tar.gz"
        );
    }

    #[test]
    fn direct_release_asset_resolves_relative_to_manifest_base() {
        let mut source = release_source("direct");
        source.fields.insert(
            "repo".to_owned(),
            ScalarValue::String("https://example.invalid/tool.elda-releases.json".to_owned()),
        );

        assert_eq!(
            resolve_release_asset_url(&source).expect("url should resolve"),
            "https://example.invalid/v1.2.3/tool-linux-amd64.tar.gz"
        );
    }
}
