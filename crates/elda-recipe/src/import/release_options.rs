use serde_json::Value;

use super::model::SourceOptionReport;
use super::release_sidecars::{matching_sha256, matching_signature};
use super::release_target::{ReleaseTarget, parse_release_target};
use super::strategy::push_release_option;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReleaseOption {
    pub(super) provider: String,
    pub(super) host: Option<String>,
    pub(super) repo: String,
    pub(super) tag: String,
    pub(super) asset: String,
    pub(super) compatibility: String,
    pub(super) sha256: Option<String>,
    pub(super) signature: Option<String>,
}

impl ReleaseOption {
    pub(super) fn source_kind(&self) -> &'static str {
        let basename = self
            .asset
            .rsplit_once('/')
            .map(|(_, tail)| tail)
            .unwrap_or(self.asset.as_str())
            .to_ascii_lowercase();
        if basename.ends_with(".appimage") {
            return "appimage";
        }
        if self.provider == "github" {
            "github_release"
        } else {
            "release_asset"
        }
    }

    pub(super) fn extra_fields(&self) -> String {
        let provider_line = if self.provider == "github" {
            String::new()
        } else {
            format!(
                "    provider = \"{}\",\n",
                escape_lua_string(&self.provider)
            )
        };
        let host_line = self
            .host
            .as_ref()
            .filter(|host| should_render_host(&self.provider, host))
            .map(|host| format!("    host = \"{}\",\n", escape_lua_string(host)))
            .unwrap_or_default();
        let signature_line = self
            .signature
            .as_ref()
            .map(|signature| format!("    signature = \"{}\",\n", escape_lua_string(signature)))
            .unwrap_or_default();
        let binary_line = if self.source_kind() == "appimage" {
            format!(
                "    binary = \"{}\",\n",
                escape_lua_string(&launcher_name_from_appimage_asset(&self.asset))
            )
        } else {
            String::new()
        };
        format!(
            "{provider_line}{host_line}    repo = \"{}\",\n    tag = \"{}\",\n    asset = \"{}\",\n    sha256 = \"{}\",\n{signature_line}{binary_line}",
            escape_lua_string(&self.repo),
            escape_lua_string(&self.tag),
            escape_lua_string(&self.asset),
            escape_lua_string(self.sha256.as_deref().unwrap_or_default()),
        )
    }
}

fn launcher_name_from_appimage_asset(asset: &str) -> String {
    let basename = asset
        .rsplit_once('/')
        .map(|(_, tail)| tail)
        .unwrap_or(asset)
        .to_ascii_lowercase();
    basename
        .strip_suffix(".appimage")
        .unwrap_or(basename.as_str())
        .to_owned()
}

fn should_render_host(provider: &str, host: &str) -> bool {
    !matches!(
        (provider, host),
        ("github", "github.com")
            | ("gitlab", "gitlab.com")
            | ("gitea", "codeberg.org")
            | ("forgejo", "codeberg.org")
            | ("sourcehut", "git.sr.ht")
    )
}

fn escape_lua_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn append_release_option(
    source_url: Option<&str>,
    options: &mut Vec<SourceOptionReport>,
    release_binary_format_priority: &[String],
) {
    let Some(option) = detect_release_option(source_url, release_binary_format_priority) else {
        return;
    };
    push_release_option(options, &option);
}

pub(super) fn detect_release_option(
    source_url: Option<&str>,
    release_binary_format_priority: &[String],
) -> Option<ReleaseOption> {
    let source_url = source_url?;
    let target = parse_release_target(source_url)?;
    let release = fetch_release_summary(&target)?;
    release_option_from_summary(
        &target.provider,
        Some(&target.host),
        &target.repo,
        &release,
        release_binary_format_priority,
    )
}

pub(super) fn release_option_from_summary(
    provider: &str,
    host: Option<&str>,
    repo: &str,
    release: &Value,
    release_binary_format_priority: &[String],
) -> Option<ReleaseOption> {
    let asset = recommended_release_asset(release, release_binary_format_priority)?;
    let asset_name = asset.get("name").and_then(Value::as_str)?;
    let tag = release
        .get("tag_name")
        .and_then(Value::as_str)
        .unwrap_or("latest");
    Some(ReleaseOption {
        provider: provider.to_owned(),
        host: host.map(str::to_owned),
        repo: repo.to_owned(),
        tag: tag.to_owned(),
        asset: asset_name.to_owned(),
        compatibility: asset
            .get("compatibility")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        sha256: matching_sha256(release, asset_name),
        signature: matching_signature(release, asset_name),
    })
}

fn fetch_release_summary(target: &ReleaseTarget) -> Option<Value> {
    let api_url = latest_release_api_url(target)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(2))
        .build();
    let response = agent.get(&api_url).set("User-Agent", "elda").call().ok()?;
    let release = serde_json::from_reader::<_, Value>(response.into_reader()).ok()?;
    Some(classified_release_summary(normalized_release_summary(
        &target.provider,
        release,
    )))
}

fn latest_release_api_url(target: &ReleaseTarget) -> Option<String> {
    match target.provider.as_str() {
        "github" => Some(format!(
            "https://api.github.com/repos/{}/releases/latest",
            target.repo
        )),
        "gitlab" => Some(format!(
            "https://{}/api/v4/projects/{}/releases/permalink/latest",
            target.host,
            target.repo.replace('/', "%2F")
        )),
        "gitea" | "forgejo" => Some(format!(
            "https://{}/api/v1/repos/{}/releases/latest",
            target.host, target.repo
        )),
        "direct" => Some(target.repo.clone()),
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn normalized_release_summary_for_test(provider: &str, release: Value) -> Value {
    normalized_release_summary(provider, release)
}

fn normalized_release_summary(provider: &str, release: Value) -> Value {
    if provider == "gitlab" {
        return normalized_gitlab_release_summary(release);
    }
    if provider == "direct" {
        return normalized_direct_release_summary(release);
    }
    release
}

fn normalized_direct_release_summary(release: Value) -> Value {
    release
        .get("releases")
        .and_then(Value::as_array)
        .and_then(|releases| releases.first())
        .cloned()
        .unwrap_or(release)
}

fn normalized_gitlab_release_summary(mut release: Value) -> Value {
    let links = release
        .get_mut("assets")
        .and_then(|assets| assets.get_mut("links"))
        .and_then(Value::as_array_mut)
        .map(std::mem::take)
        .unwrap_or_default();
    let assets = links
        .into_iter()
        .filter_map(gitlab_link_asset)
        .collect::<Vec<_>>();
    if let Some(object) = release.as_object_mut() {
        object.insert("assets".to_owned(), Value::Array(assets));
    }
    release
}

fn gitlab_link_asset(mut link: Value) -> Option<Value> {
    let url = link
        .get("direct_asset_url")
        .and_then(Value::as_str)
        .or_else(|| link.get("url").and_then(Value::as_str))?
        .to_owned();
    if let Some(object) = link.as_object_mut() {
        object.insert("browser_download_url".to_owned(), Value::String(url));
    }
    Some(link)
}

pub(super) fn classified_release_summary(mut release: Value) -> Value {
    let Some(assets) = release.get_mut("assets").and_then(Value::as_array_mut) else {
        return release;
    };
    for asset in assets {
        classify_release_asset(asset);
    }
    release
}

fn classify_release_asset(asset: &mut Value) {
    let name = asset
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    let kind = release_asset_kind(&lower);
    let compatibility = release_asset_compatibility(kind, &lower);
    if let Some(object) = asset.as_object_mut() {
        object.insert("kind".to_owned(), Value::String(kind.to_owned()));
        object.insert(
            "compatibility".to_owned(),
            Value::String(compatibility.to_owned()),
        );
    }
}

/// Default preference order for git-release binary payloads. Archives and distro-native packages
/// come before AppImages so operators get normal tarballs when both exist.
#[must_use]
pub fn default_release_binary_format_priority() -> Vec<String> {
    [
        "tar-gz",
        "tar-xz",
        "tar-zst",
        "zip",
        "raw-binary",
        "deb",
        "rpm",
        "apk",
        "pacman-package",
        "app-image",
        "unknown",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[must_use]
pub fn effective_release_binary_format_priority(config: &[String]) -> Vec<String> {
    if config.is_empty() {
        default_release_binary_format_priority()
    } else {
        config
            .iter()
            .map(|entry| normalize_format_key(entry))
            .collect()
    }
}

fn normalize_format_key(raw: &str) -> String {
    let trimmed = raw.trim();
    let lowered = trimmed.to_ascii_lowercase().replace('_', "-");
    match lowered.as_str() {
        "appimage" => "app-image".to_owned(),
        "tgz" => "tar-gz".to_owned(),
        "txz" => "tar-xz".to_owned(),
        "tzst" => "tar-zst".to_owned(),
        other => other.to_owned(),
    }
}

/// Mirrors `elda-git` release payload classification (kebab-case format ids).
fn payload_format_kebab(lower: &str) -> &'static str {
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        "tar-gz"
    } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        "tar-xz"
    } else if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        "tar-zst"
    } else if lower.ends_with(".zip") {
        "zip"
    } else if lower.ends_with(".appimage") {
        "app-image"
    } else if lower.ends_with(".deb") {
        "deb"
    } else if lower.ends_with(".rpm") {
        "rpm"
    } else if lower.ends_with(".apk") {
        "apk"
    } else if lower.ends_with(".pkg.tar.zst") || lower.ends_with(".pkg.tar.xz") {
        "pacman-package"
    } else if has_no_extension(lower) {
        "raw-binary"
    } else {
        "unknown"
    }
}

fn has_no_extension(lower: &str) -> bool {
    !lower.rsplit('/').next().unwrap_or(lower).contains('.')
}

pub(super) fn recommended_release_asset<'a>(
    release: &'a Value,
    release_binary_format_priority: &[String],
) -> Option<&'a Value> {
    let assets = release.get("assets").and_then(Value::as_array)?;
    let allowed = effective_release_binary_format_priority(release_binary_format_priority);

    let mut best: Option<(u8, usize, &Value)> = None;
    for asset in assets {
        let kind = asset.get("kind").and_then(Value::as_str)?;
        if kind != "payload" {
            continue;
        }
        let compat = asset.get("compatibility").and_then(Value::as_str)?;
        let tier = match compat {
            "native-exact" => 0_u8,
            "native-partial" => 1_u8,
            _ => continue,
        };
        let name = asset.get("name").and_then(Value::as_str)?;
        let format_key = normalize_format_key(payload_format_kebab(&name.to_ascii_lowercase()));
        let Some(pos) = allowed
            .iter()
            .position(|allowed_key| allowed_key == &format_key)
        else {
            continue;
        };
        let replace = match best {
            None => true,
            Some((best_tier, best_pos, _)) => {
                tier < best_tier || (tier == best_tier && pos < best_pos)
            }
        };
        if replace {
            best = Some((tier, pos, asset));
        }
    }

    if let Some((_, _, asset)) = best {
        return Some(asset);
    }

    // Only fall back to compatibility-only selection when the operator did not supply an explicit
    // format filter; otherwise returning an excluded format (for example `app-image`) would violate
    // `release_binary_format_priority`.
    if release_binary_format_priority.is_empty() {
        return assets
            .iter()
            .find(|asset| {
                asset.get("kind").and_then(Value::as_str) == Some("payload")
                    && asset.get("compatibility").and_then(Value::as_str) == Some("native-exact")
            })
            .or_else(|| {
                assets.iter().find(|asset| {
                    asset.get("kind").and_then(Value::as_str) == Some("payload")
                        && asset.get("compatibility").and_then(Value::as_str)
                            == Some("native-partial")
                })
            });
    }

    None
}

fn release_asset_kind(lower: &str) -> &'static str {
    if [
        ".sha256",
        ".sha256sum",
        ".sha512",
        ".sig",
        ".asc",
        ".minisig",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
    {
        "sidecar"
    } else {
        "payload"
    }
}

fn release_asset_compatibility(kind: &str, lower: &str) -> &'static str {
    if kind != "payload" {
        return "sidecar";
    }
    let os_match = os_aliases().iter().any(|alias| lower.contains(alias));
    let arch_match = arch_aliases().iter().any(|alias| lower.contains(alias));
    let libc_match = libc_aliases().iter().any(|alias| lower.contains(alias));
    match (os_match, arch_match, libc_match) {
        (true, true, true) => "native-exact",
        (true, true, false) => "native-partial",
        (false, false, false) => "unknown",
        _ => "foreign",
    }
}

fn os_aliases() -> &'static [&'static str] {
    match std::env::consts::OS {
        "linux" => &["linux", "unknown-linux"],
        "macos" => &["darwin", "apple-darwin", "macos", "osx"],
        "freebsd" => &["freebsd", "unknown-freebsd"],
        "windows" => &["windows", "pc-windows", "win64", "win32"],
        _ => &[std::env::consts::OS],
    }
}

fn arch_aliases() -> &'static [&'static str] {
    match std::env::consts::ARCH {
        "x86_64" => &["x86_64", "amd64"],
        "aarch64" => &["aarch64", "arm64"],
        "x86" => &["i686", "i386", "386"],
        "arm" => &["armv7", "armv7l", "armhf"],
        _ => &[std::env::consts::ARCH],
    }
}

fn libc_aliases() -> &'static [&'static str] {
    if cfg!(target_env = "gnu") {
        &["gnu", "glibc"]
    } else if cfg!(target_env = "musl") {
        &["musl"]
    } else {
        &[]
    }
}
