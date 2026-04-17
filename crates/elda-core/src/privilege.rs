use std::fmt;

use serde::{Deserialize, Serialize};

use rustix::process::geteuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PrivilegeProvider {
    Auto,
    Doas,
    Sudo,
    Run0,
    Su,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct PrivilegeConfig {
    pub provider: PrivilegeProvider,
    pub preserve_env: bool,
    pub interactive: bool,
}

impl Default for PrivilegeConfig {
    fn default() -> Self {
        Self {
            provider: PrivilegeProvider::Auto,
            preserve_env: false,
            interactive: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PrivilegeStatus {
    pub provider: PrivilegeProvider,
    pub preserve_env: bool,
    pub interactive: bool,
    pub is_superuser: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegeRequest {
    pub provider: PrivilegeProvider,
    pub preserve_env: bool,
    pub interactive: bool,
}

impl PrivilegeStatus {
    #[must_use]
    pub fn detect(config: &PrivilegeConfig) -> Self {
        Self {
            provider: config.provider,
            preserve_env: config.preserve_env,
            interactive: config.interactive,
            is_superuser: geteuid().is_root(),
        }
    }
}

impl PrivilegeRequest {
    #[must_use]
    pub fn from_config(config: &PrivilegeConfig) -> Self {
        Self {
            provider: config.provider,
            preserve_env: config.preserve_env,
            interactive: config.interactive,
        }
    }
}

impl fmt::Display for PrivilegeRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.provider {
            PrivilegeProvider::Auto => write!(
                formatter,
                "superuser access required; re-run through an available privilege provider or configure one explicitly"
            ),
            PrivilegeProvider::Doas => write!(
                formatter,
                "superuser access required; re-run through `doas` or enable a supported privilege provider"
            ),
            PrivilegeProvider::Sudo => write!(
                formatter,
                "superuser access required; re-run through `sudo` or enable a supported privilege provider"
            ),
            PrivilegeProvider::Run0 => write!(
                formatter,
                "superuser access required; re-run through `run0` or enable a supported privilege provider"
            ),
            PrivilegeProvider::Su => write!(
                formatter,
                "superuser access required; re-run through `su` or enable a supported privilege provider"
            ),
            PrivilegeProvider::None => write!(
                formatter,
                "superuser access required; configure a supported privilege provider or run as root"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PrivilegeConfig, PrivilegeProvider, PrivilegeRequest};

    #[test]
    fn privilege_config_defaults_to_auto_provider() {
        let config = PrivilegeConfig::default();

        assert_eq!(config.provider, PrivilegeProvider::Auto);
        assert!(config.interactive);
        assert!(!config.preserve_env);
    }

    #[test]
    fn auto_request_display_mentions_detected_provider() {
        let request = PrivilegeRequest {
            provider: PrivilegeProvider::Auto,
            preserve_env: false,
            interactive: true,
        };

        assert!(request.to_string().contains("available privilege provider"));
    }
}
