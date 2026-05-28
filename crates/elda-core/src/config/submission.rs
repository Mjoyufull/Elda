use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SubmissionConfig {
    pub mode: SubmissionMode,
    pub auto_open: bool,
    pub auto_assign: bool,
    pub auth: SubmissionAuthKind,
    pub token_env: String,
    pub api_base: Option<String>,
    pub remote_name: String,
    pub base_branch: String,
    pub remotes: BTreeMap<String, SubmissionRemoteConfig>,
}

impl SubmissionConfig {
    pub fn remote_name(&self) -> &str {
        normalize_value(&self.remote_name).unwrap_or("origin")
    }

    pub fn base_branch(&self) -> &str {
        normalize_value(&self.base_branch).unwrap_or("main")
    }

    pub fn resolve_target(&self) -> ResolvedSubmissionConfig {
        self.resolve_for_remote(self.remote_name())
    }

    pub fn resolve_for_remote(&self, remote_name: &str) -> ResolvedSubmissionConfig {
        let override_config = self.remotes.get(remote_name);
        let token_env = override_config
            .and_then(|config| normalize_owned(config.token_env.clone()))
            .or_else(|| normalize_owned(Some(self.token_env.clone())))
            .unwrap_or_default();
        let api_base = override_config
            .and_then(|config| normalize_owned(config.api_base.clone()))
            .or_else(|| normalize_owned(self.api_base.clone()));
        let base_branch = override_config
            .and_then(|config| normalize_owned(config.base_branch.clone()))
            .unwrap_or_else(|| self.base_branch().to_owned());

        ResolvedSubmissionConfig {
            mode: self.mode,
            auto_open: override_config
                .and_then(|config| config.auto_open)
                .unwrap_or(self.auto_open),
            auto_assign: override_config
                .and_then(|config| config.auto_assign)
                .unwrap_or(self.auto_assign),
            auth: override_config
                .and_then(|config| config.auth)
                .unwrap_or(self.auth),
            token_env,
            api_base,
            remote_name: remote_name.trim().to_owned(),
            base_branch,
        }
    }
}

impl Default for SubmissionConfig {
    fn default() -> Self {
        Self {
            mode: SubmissionMode::Pr,
            auto_open: false,
            auto_assign: false,
            auth: SubmissionAuthKind::None,
            token_env: String::new(),
            api_base: None,
            remote_name: "origin".to_owned(),
            base_branch: "main".to_owned(),
            remotes: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SubmissionRemoteConfig {
    pub auto_open: Option<bool>,
    pub auto_assign: Option<bool>,
    pub auth: Option<SubmissionAuthKind>,
    pub token_env: Option<String>,
    pub api_base: Option<String>,
    pub base_branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSubmissionConfig {
    pub mode: SubmissionMode,
    pub auto_open: bool,
    pub auto_assign: bool,
    pub auth: SubmissionAuthKind,
    pub token_env: String,
    pub api_base: Option<String>,
    pub remote_name: String,
    pub base_branch: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubmissionMode {
    #[default]
    Pr,
    Push,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubmissionAuthKind {
    Token,
    Bearer,
    Ssh,
    #[default]
    None,
}

fn normalize_value(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn normalize_owned(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_owned())
}
