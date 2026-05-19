use super::{GitReleaseSource, ReleaseProvider};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseTarget {
    pub provider: ReleaseProvider,
    pub host: String,
    pub repo: String,
}

pub fn parse_release_target(target: &str) -> Option<ReleaseTarget> {
    parse_direct_manifest_target(target)
        .or_else(|| parse_known_host_target(target))
        .or_else(|| parse_short_github_target(target))
}

pub fn parse_github_repo_target(target: &str) -> Option<String> {
    parse_release_target(target)
        .filter(|target| target.provider == ReleaseProvider::Github)
        .map(|target| target.repo)
}

fn parse_direct_manifest_target(target: &str) -> Option<ReleaseTarget> {
    let trimmed = target.trim();
    if !trimmed.ends_with(".elda-releases.json") {
        return None;
    }
    Some(ReleaseTarget {
        provider: ReleaseProvider::Direct,
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

fn parse_short_github_target(target: &str) -> Option<ReleaseTarget> {
    let trimmed = target.trim().trim_end_matches('/');
    if trimmed.split('/').count() != 2 || trimmed.contains(':') {
        return None;
    }
    clean_owner_repo(trimmed).map(|repo| ReleaseTarget {
        provider: ReleaseProvider::Github,
        host: "github.com".to_owned(),
        repo,
    })
}

fn parse_known_host_target(target: &str) -> Option<ReleaseTarget> {
    let trimmed = target.trim().trim_end_matches('/');
    parse_ssh_target(trimmed).or_else(|| parse_http_target(trimmed))
}

fn parse_ssh_target(target: &str) -> Option<ReleaseTarget> {
    let rest = target.strip_prefix("git@")?;
    let (host, path) = rest.split_once(':')?;
    release_provider_for_host(host).and_then(|provider| {
        clean_repo_path(path, provider.allows_nested_repo()).map(|repo| ReleaseTarget {
            provider,
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
        clean_repo_path(path, provider.allows_nested_repo()).map(|repo| ReleaseTarget {
            provider,
            host: host.to_owned(),
            repo,
        })
    })
}

fn release_provider_for_host(host: &str) -> Option<ReleaseProvider> {
    match host {
        "github.com" => Some(ReleaseProvider::Github),
        "gitlab.com" => Some(ReleaseProvider::Gitlab),
        "codeberg.org" => Some(ReleaseProvider::Forgejo),
        "git.sr.ht" => Some(ReleaseProvider::Sourcehut),
        other if looks_like_gitlab_host(other) => Some(ReleaseProvider::Gitlab),
        other if looks_like_forgejo_host(other) => Some(ReleaseProvider::Forgejo),
        other if looks_like_gitea_host(other) => Some(ReleaseProvider::Gitea),
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

fn clean_owner_repo(value: &str) -> Option<String> {
    clean_repo_path(value, false)
}

fn clean_repo_path(value: &str, allow_nested: bool) -> Option<String> {
    let trimmed = value
        .trim_end_matches(".git")
        .split(['?', '#'])
        .next()?
        .trim_end_matches('/');
    let parts = trimmed
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

impl ReleaseTarget {
    pub(crate) fn source(&self) -> GitReleaseSource {
        match self.provider {
            ReleaseProvider::Github => GitReleaseSource::GithubApi,
            ReleaseProvider::Gitlab => GitReleaseSource::GitlabApi,
            ReleaseProvider::Gitea => GitReleaseSource::GiteaApi,
            ReleaseProvider::Forgejo => GitReleaseSource::ForgejoApi,
            ReleaseProvider::Sourcehut => GitReleaseSource::SourcehutApi,
            ReleaseProvider::Direct => GitReleaseSource::DirectManifest,
        }
    }
}

impl ReleaseProvider {
    fn allows_nested_repo(self) -> bool {
        matches!(
            self,
            Self::Gitlab | Self::Gitea | Self::Forgejo | Self::Sourcehut
        )
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitea => "gitea",
            Self::Forgejo => "forgejo",
            Self::Sourcehut => "sourcehut",
            Self::Direct => "direct",
        }
    }
}
