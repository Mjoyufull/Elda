#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReleaseTarget {
    pub(super) provider: String,
    pub(super) host: String,
    pub(super) repo: String,
}

pub(super) fn parse_release_target(target: &str) -> Option<ReleaseTarget> {
    parse_direct_manifest_target(target).or_else(|| {
        let trimmed = target.trim().trim_end_matches('/');
        parse_ssh_target(trimmed).or_else(|| parse_http_target(trimmed))
    })
}

fn parse_direct_manifest_target(target: &str) -> Option<ReleaseTarget> {
    let trimmed = target.trim();
    if !trimmed.ends_with(".elda-releases.json") {
        return None;
    }
    Some(ReleaseTarget {
        provider: "direct".to_owned(),
        host: direct_manifest_host(trimmed),
        repo: trimmed.to_owned(),
    })
}

fn direct_manifest_host(target: &str) -> String {
    target
        .strip_prefix("https://")
        .or_else(|| target.strip_prefix("http://"))
        .and_then(|rest| rest.split_once('/').map(|(host, _)| host.to_owned()))
        .unwrap_or_else(|| "local".to_owned())
}

fn parse_ssh_target(target: &str) -> Option<ReleaseTarget> {
    let rest = target.strip_prefix("git@")?;
    let (host, path) = rest.split_once(':')?;
    release_provider_for_host(host).and_then(|provider| {
        clean_repo_path(path, provider_allows_nested_repo(provider)).map(|repo| ReleaseTarget {
            provider: provider.to_owned(),
            host: host.to_owned(),
            repo,
        })
    })
}

fn parse_http_target(target: &str) -> Option<ReleaseTarget> {
    let rest = target
        .strip_prefix("https://")
        .or_else(|| target.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/')?;
    release_provider_for_host(host).and_then(|provider| {
        clean_repo_path(path, provider_allows_nested_repo(provider)).map(|repo| ReleaseTarget {
            provider: provider.to_owned(),
            host: host.to_owned(),
            repo,
        })
    })
}

fn provider_allows_nested_repo(provider: &str) -> bool {
    matches!(provider, "gitlab" | "gitea" | "forgejo" | "sourcehut")
}

fn release_provider_for_host(host: &str) -> Option<&'static str> {
    match host {
        "github.com" => Some("github"),
        "gitlab.com" => Some("gitlab"),
        "codeberg.org" => Some("forgejo"),
        "git.sr.ht" => Some("sourcehut"),
        other if looks_like_gitlab_host(other) => Some("gitlab"),
        other if looks_like_forgejo_host(other) => Some("forgejo"),
        other if looks_like_gitea_host(other) => Some("gitea"),
        _ => None,
    }
}

fn looks_like_gitlab_host(host: &str) -> bool {
    host.contains("gitlab")
}

fn looks_like_forgejo_host(host: &str) -> bool {
    host.contains("forgejo")
}

fn looks_like_gitea_host(host: &str) -> bool {
    host.contains("gitea")
}

fn clean_repo_path(value: &str, allow_nested: bool) -> Option<String> {
    let cleaned = value
        .trim_end_matches(".git")
        .split(['?', '#'])
        .next()?
        .trim_end_matches('/');
    let parts = cleaned
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    if allow_nested {
        Some(parts.join("/"))
    } else {
        Some(format!("{}/{}", parts[0], parts[1]))
    }
}
