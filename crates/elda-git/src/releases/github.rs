use serde::Deserialize;

use super::{ReleaseInspectError, ReleaseProvider, ReleaseTarget};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReleaseResponse {
    #[serde(alias = "tag_name")]
    pub(crate) tag_name: String,
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) draft: bool,
    #[serde(default)]
    pub(crate) prerelease: bool,
    pub(crate) published_at: Option<String>,
    #[serde(default)]
    pub(crate) assets: Vec<ReleaseAssetResponse>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReleaseAssetResponse {
    pub(crate) name: String,
    #[serde(alias = "browser_download_url", alias = "direct_asset_url")]
    pub(crate) browser_download_url: String,
}

pub(crate) fn fetch_provider_releases(
    target: &ReleaseTarget,
    max_releases: usize,
) -> Result<Vec<ReleaseResponse>, ReleaseInspectError> {
    let per_page = max_releases.clamp(1, 100);
    let api_url = release_api_url(target, per_page);
    let response = ureq::get(&api_url)
        .set("User-Agent", "elda")
        .call()
        .map_err(|error| release_lookup_error(target, error))?;
    let value = serde_json::from_reader(response.into_reader()).map_err(|source| {
        ReleaseInspectError::Decode {
            provider: target.provider.as_str().to_owned(),
            repo: target.repo.clone(),
            source,
        }
    })?;
    normalize_release_response(target, value)
}

pub(crate) fn normalize_release_response(
    target: &ReleaseTarget,
    value: serde_json::Value,
) -> Result<Vec<ReleaseResponse>, ReleaseInspectError> {
    match target.provider {
        ReleaseProvider::Github | ReleaseProvider::Gitea | ReleaseProvider::Forgejo => {
            serde_json::from_value(value).map_err(|source| ReleaseInspectError::Decode {
                provider: target.provider.as_str().to_owned(),
                repo: target.repo.clone(),
                source,
            })
        }
        ReleaseProvider::Gitlab => normalize_gitlab_releases(value),
        ReleaseProvider::Sourcehut => normalize_sourcehut_releases(value),
        ReleaseProvider::Direct => normalize_direct_releases(value),
    }
}

fn normalize_sourcehut_releases(
    value: serde_json::Value,
) -> Result<Vec<ReleaseResponse>, ReleaseInspectError> {
    let references = value
        .get("data")
        .and_then(|data| data.get("repository"))
        .and_then(|repo| repo.get("references"))
        .and_then(|references| references.get("results"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(references.into_iter().map(sourcehut_reference).collect())
}

fn sourcehut_reference(value: serde_json::Value) -> ReleaseResponse {
    let tag = string_value(&value, "name")
        .or_else(|| string_value(&value, "target"))
        .unwrap_or_else(|| "latest".to_owned());
    ReleaseResponse {
        tag_name: tag.clone(),
        name: Some(tag),
        draft: false,
        prerelease: false,
        published_at: string_value(&value, "created"),
        assets: value
            .get("artifacts")
            .and_then(|artifacts| artifacts.get("results"))
            .and_then(serde_json::Value::as_array)
            .map(|artifacts| artifacts.iter().filter_map(sourcehut_artifact).collect())
            .unwrap_or_default(),
    }
}

fn sourcehut_artifact(value: &serde_json::Value) -> Option<ReleaseAssetResponse> {
    let name = string_value(value, "filename").or_else(|| string_value(value, "name"))?;
    let url = string_value(value, "url")?;
    Some(ReleaseAssetResponse {
        name,
        browser_download_url: url,
    })
}

fn normalize_direct_releases(
    value: serde_json::Value,
) -> Result<Vec<ReleaseResponse>, ReleaseInspectError> {
    if value.get("releases").is_some() {
        return serde_json::from_value(value.get("releases").cloned().unwrap_or_default()).map_err(
            |source| ReleaseInspectError::Decode {
                provider: "direct".to_owned(),
                repo: "direct".to_owned(),
                source,
            },
        );
    }
    serde_json::from_value(value).map_err(|source| ReleaseInspectError::Decode {
        provider: "direct".to_owned(),
        repo: "direct".to_owned(),
        source,
    })
}

fn normalize_gitlab_releases(
    value: serde_json::Value,
) -> Result<Vec<ReleaseResponse>, ReleaseInspectError> {
    let releases = value.as_array().cloned().unwrap_or_else(|| vec![value]);
    Ok(releases.into_iter().map(gitlab_release).collect())
}

fn gitlab_release(value: serde_json::Value) -> ReleaseResponse {
    ReleaseResponse {
        tag_name: string_value(&value, "tag_name").unwrap_or_else(|| "latest".to_owned()),
        name: string_value(&value, "name"),
        draft: false,
        prerelease: false,
        published_at: string_value(&value, "released_at")
            .or_else(|| string_value(&value, "created_at")),
        assets: gitlab_assets(&value),
    }
}

fn gitlab_assets(value: &serde_json::Value) -> Vec<ReleaseAssetResponse> {
    let mut assets = Vec::new();
    if let Some(links) = value
        .get("assets")
        .and_then(|assets| assets.get("links"))
        .and_then(serde_json::Value::as_array)
    {
        for link in links {
            if let (Some(name), Some(url)) = (
                string_value(link, "name"),
                string_value(link, "direct_asset_url").or_else(|| string_value(link, "url")),
            ) {
                assets.push(ReleaseAssetResponse {
                    name,
                    browser_download_url: url,
                });
            }
        }
    }
    assets
}

fn string_value(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn release_api_url(target: &ReleaseTarget, per_page: usize) -> String {
    match target.provider {
        ReleaseProvider::Github => {
            format!(
                "https://api.github.com/repos/{}/releases?per_page={per_page}",
                target.repo
            )
        }
        ReleaseProvider::Gitlab => {
            let encoded = target.repo.replace('/', "%2F");
            format!(
                "https://{}/api/v4/projects/{encoded}/releases?per_page={per_page}",
                target.host
            )
        }
        ReleaseProvider::Gitea | ReleaseProvider::Forgejo => {
            format!(
                "https://{}/api/v1/repos/{}/releases?limit={per_page}",
                target.host, target.repo
            )
        }
        ReleaseProvider::Sourcehut => {
            format!(
                "https://{}/query?query={}",
                target.host,
                sourcehut_releases_query(&target.repo, per_page)
            )
        }
        ReleaseProvider::Direct => target.repo.clone(),
    }
}

fn sourcehut_releases_query(repo: &str, limit: usize) -> String {
    let query = format!(
        "query {{ repository(name: \"{repo}\") {{ references(pattern: \"refs/tags/*\", first: {limit}) {{ results {{ name artifacts {{ results {{ filename url }} }} }} }} }} }}"
    );
    encode_query_component(&query)
}

fn encode_query_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn release_lookup_error(target: &ReleaseTarget, error: ureq::Error) -> ReleaseInspectError {
    let detail = match error {
        ureq::Error::Status(status, response) => status_error(status, response),
        ureq::Error::Transport(error) => error.to_string(),
    };
    ReleaseInspectError::LookupFailed {
        provider: target.provider.as_str().to_owned(),
        repo: target.repo.clone(),
        detail,
    }
}

fn status_error(status: u16, response: ureq::Response) -> String {
    let body = response
        .into_string()
        .unwrap_or_else(|_| "<unreadable response>".to_owned());
    format!("http {status}: {}", body.trim())
}
