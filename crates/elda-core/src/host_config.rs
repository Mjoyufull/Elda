use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::CoreError;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostConfigFile {
    pub host: HostSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostSection {
    pub profile: Option<String>,
    pub tree: Option<String>,
    pub packages_subdir: Option<String>,
    pub default_channel: Option<String>,
    pub signing_key: Option<String>,
    pub signing_key_env: Option<String>,
    #[serde(default)]
    pub forge: HostForgeConfig,
    #[serde(default)]
    pub channels: BTreeMap<String, HostChannelConfig>,
    #[serde(default)]
    pub publish: HostPublishConfig,
    #[serde(default)]
    pub cache: HostCacheConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostForgeConfig {
    pub remote: Option<String>,
    pub base_branch: Option<String>,
    pub mr_mode: Option<String>,
    pub token_env: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostChannelConfig {
    pub branch: Option<String>,
    pub index_subpath: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostPublishConfig {
    pub index_target: Option<String>,
    pub release_tag_template: Option<String>,
    pub base_url_env: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct HostCacheConfig {
    pub populate_after_publish: Option<String>,
    pub upload_command_env: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedHostProfile {
    pub name: String,
    pub path: PathBuf,
    pub section: HostSection,
}

impl ResolvedHostProfile {
    pub fn packages_subdir(&self) -> &str {
        self.section
            .packages_subdir
            .as_deref()
            .unwrap_or("packages")
    }

    pub fn default_channel(&self) -> &str {
        self.section.default_channel.as_deref().unwrap_or("stable")
    }

    pub fn tree_path(&self) -> Option<PathBuf> {
        self.section.tree.as_ref().map(PathBuf::from)
    }

    pub fn channel_branch(&self, channel: &str) -> String {
        self.section
            .channels
            .get(channel)
            .and_then(|entry| entry.branch.clone())
            .unwrap_or_else(|| channel.to_owned())
    }

    pub fn channel_index_subpath(&self, channel: &str) -> Option<String> {
        self.section
            .channels
            .get(channel)
            .and_then(|entry| entry.index_subpath.clone())
            .or_else(|| Some(channel.to_owned()))
    }

    pub fn resolve_base_url(&self) -> Option<String> {
        if let Some(url) = &self.section.publish.base_url {
            if !url.trim().is_empty() {
                return Some(url.trim().to_owned());
            }
        }
        if let Some(env_name) = &self.section.publish.base_url_env {
            if let Ok(value) = std::env::var(env_name) {
                if !value.trim().is_empty() {
                    return Some(value.trim().to_owned());
                }
            }
        }
        None
    }

    pub fn signing_key_path(&self) -> Option<PathBuf> {
        if let Some(path) = &self.section.signing_key {
            return Some(PathBuf::from(path));
        }
        if let Some(env_name) = &self.section.signing_key_env {
            if let Ok(path) = std::env::var(env_name) {
                if !path.trim().is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
        None
    }
}

pub fn load_host_profile(
    root_dir: &Path,
    profile_name: Option<&str>,
) -> Result<ResolvedHostProfile, CoreError> {
    let host_dir = root_dir.join("etc/elda/host.d");
    let mut profiles = Vec::new();

    if host_dir.is_dir() {
        for entry in fs::read_dir(&host_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("host")
                .to_owned();
            let content = fs::read_to_string(&path)?;
            let parsed = toml::from_str::<HostConfigFile>(&content).map_err(|error| {
                CoreError::Operator(format!(
                    "invalid host profile `{}`: {error}",
                    path.display()
                ))
            })?;
            profiles.push(ResolvedHostProfile {
                name,
                path,
                section: parsed.host,
            });
        }
    }

    let config_path = root_dir.join("etc/elda/config.toml");
    if config_path.is_file() {
        let content = fs::read_to_string(&config_path)?;
        if let Ok(parsed) = toml::from_str::<HostConfigFile>(&content) {
            if parsed.host.profile.is_some() || parsed.host.tree.is_some() {
                let name = parsed
                    .host
                    .profile
                    .clone()
                    .unwrap_or_else(|| "config".to_owned());
                profiles.push(ResolvedHostProfile {
                    name,
                    path: config_path.clone(),
                    section: parsed.host,
                });
            }
        }
    }

    if profiles.is_empty() {
        return Err(CoreError::Operator(
            "no host profile found; create etc/elda/host.d/<name>.toml or add [host] to config.toml"
                .to_owned(),
        ));
    }

    profiles.sort_by(|left, right| left.name.cmp(&right.name));

    if let Some(requested) = profile_name {
        profiles
            .into_iter()
            .find(|profile| profile.name == requested)
            .ok_or_else(|| {
                CoreError::Operator(format!(
                    "host profile `{requested}` was not found under etc/elda/host.d/"
                ))
            })
    } else {
        Ok(profiles.remove(0))
    }
}
