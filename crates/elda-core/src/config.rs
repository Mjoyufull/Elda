use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::privilege::PrivilegeConfig;

mod submission;

pub use submission::{
    ResolvedSubmissionConfig, SubmissionAuthKind, SubmissionConfig, SubmissionMode,
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TrustConfig {
    pub release_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub defaults: DefaultsConfig,
    pub privilege: PrivilegeConfig,
    pub profile: ProfileConfig,
    pub resolver: ResolverConfig,
    pub flags: FlagsConfig,
    pub logging: LoggingConfig,
    pub display: DisplayConfig,
    pub capabilities: CapabilitiesConfig,
    pub metadata: MetadataConfig,
    pub git: GitConfig,
    pub submission: SubmissionConfig,
    pub trust: TrustConfig,
}

impl Config {
    pub fn load(root_dir: &Path) -> Result<Self, CoreError> {
        let config_path = root_dir.join("etc/elda/config.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config = toml::from_str::<Self>(&content)?;

        Ok(config)
    }

    pub fn append_release_keys(root_dir: &Path, keys: &[String]) -> Result<(), CoreError> {
        if keys.is_empty() {
            return Ok(());
        }
        let config_path = root_dir.join("etc/elda/config.toml");
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut doc: toml::Table = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            content
                .parse()
                .map_err(|error| CoreError::Operator(format!("invalid config.toml: {error}")))?
        } else {
            toml::Table::new()
        };

        let trust = doc
            .entry("trust")
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        let toml::Value::Table(trust_table) = trust else {
            return Err(CoreError::Operator(
                "[trust] in config.toml must be a table".to_owned(),
            ));
        };
        let release = trust_table
            .entry("release_keys")
            .or_insert_with(|| toml::Value::Array(Vec::new()));
        let toml::Value::Array(release_keys) = release else {
            return Err(CoreError::Operator(
                "[trust].release_keys must be an array".to_owned(),
            ));
        };

        for key in keys {
            if !release_keys
                .iter()
                .any(|value| value.as_str() == Some(key.as_str()))
            {
                release_keys.push(toml::Value::String(key.clone()));
            }
        }

        let content = toml::to_string_pretty(&doc).map_err(|error| {
            CoreError::Operator(format!("failed to write config.toml: {error}"))
        })?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn write_default(root_dir: &Path) -> Result<(), CoreError> {
        let config_dir = root_dir.join("etc/elda");
        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            return Ok(());
        }
        let content = r#"[defaults]
remote = "yoka-main"
build_mode = "isolated"
prefix = "/usr"
allow_system_mode = true
snapshot_tool = "none"
auto_create_config = true
mode_policy = "host"
install_recommends = true
refresh_weak_deps = false
install_preference = "binary"

[privilege]
provider = "auto"
preserve_env = false
interactive = true

[profile]
base = ""
native_arch = "amd64"
foreign_arches = []
init = ""

[logging]
dir = "~/.config/elda/logs"
level = 0

[display]
default_mode = "human"
human_detail = "normal"
tree_chars = "auto"

[capabilities]
profile = "default-host"
network_fetch = true
network_publish = true
local_editors = true
local_exec_build = true
system_activate = true
profile_apply = true
migration = true
extension_runtime = true

[metadata]
link_option_mode = "priority"
link_strategy_priority = [
  "elda-native",
  "nix_flake",
  "gentoo_ebuild",
  "aur_pkgbuild",
  "xbps_template",
  "cargo",
  "cmake",
  "meson",
  "make",
  "go",
  "python",
  "zig",
  "nimble",
  "git_release",
  "git_source",
]

[git]
tag_policy = "semver"
include_prereleases = true
strip_v_prefix = true
allow_date_versions = true
max_tags = 50
allowed_protocols = ["https", "ssh", "file"]
"#
        .to_owned();
        fs::write(config_path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    pub remote: String,
    pub build_mode: String,
    pub prefix: PathBuf,
    pub allow_system_mode: bool,
    pub snapshot_tool: String,
    pub auto_create_config: bool,
    pub mode_policy: String,
    pub install_recommends: bool,
    pub refresh_weak_deps: bool,
    pub install_preference: InstallPreference,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            remote: "yoka-main".to_owned(),
            build_mode: "isolated".to_owned(),
            prefix: PathBuf::from("/usr"),
            allow_system_mode: false,
            snapshot_tool: "none".to_owned(),
            auto_create_config: true,
            mode_policy: "host".to_owned(),
            install_recommends: true,
            refresh_weak_deps: false,
            install_preference: InstallPreference::Binary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InstallPreference {
    Source,
    #[default]
    Binary,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfileConfig {
    pub base: String,
    pub native_arch: String,
    pub foreign_arches: Vec<String>,
    pub init: String,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            base: String::new(),
            native_arch: default_native_arch(),
            foreign_arches: Vec::new(),
            init: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FlagsConfig {
    pub global: BTreeMap<String, bool>,
    pub profile: BTreeMap<String, BTreeMap<String, bool>>,
    pub package: BTreeMap<String, BTreeMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GitConfig {
    pub tag_policy: String,
    pub include_prereleases: bool,
    pub strip_v_prefix: bool,
    pub allow_date_versions: bool,
    pub max_tags: usize,
    pub allowed_protocols: Vec<String>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            tag_policy: "semver".to_owned(),
            include_prereleases: true,
            strip_v_prefix: true,
            allow_date_versions: true,
            max_tags: 50,
            allowed_protocols: vec!["https".to_owned(), "ssh".to_owned(), "file".to_owned()],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MetadataConfig {
    pub link_strategy_priority: Vec<String>,
    pub link_option_mode: LinkOptionMode,
    /// Preferred ordering for **payload** assets on git releases (`git_release` / GitHub/GitLab
    /// API summaries). Kebab-case identifiers match release asset formats (`tar-gz`, `zip`,
    /// `app-image`, …). Empty = built-in default (archives and distro packages before AppImages).
    /// Any format omitted from a non-empty list is excluded from automatic binary selection.
    pub release_binary_format_priority: Vec<String>,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            link_strategy_priority: default_link_strategy_priority(),
            link_option_mode: LinkOptionMode::Priority,
            release_binary_format_priority: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LinkOptionMode {
    #[default]
    Priority,
    ListOptions,
}

#[must_use]
pub fn default_link_strategy_priority() -> Vec<String> {
    [
        "elda-native",
        "nix_flake",
        "gentoo_ebuild",
        "aur_pkgbuild",
        "xbps_template",
        "cargo",
        "cmake",
        "meson",
        "make",
        "go",
        "python",
        "zig",
        "nimble",
        "git_release",
        "git_source",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ResolverConfig {
    pub provider_preferences: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub dir: String,
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub default_mode: String,
    pub human_detail: String,
    pub tree_chars: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            default_mode: "human".to_owned(),
            human_detail: "normal".to_owned(),
            tree_chars: "auto".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CapabilitiesConfig {
    pub profile: String,
    pub network_fetch: bool,
    pub network_publish: bool,
    pub local_editors: bool,
    pub local_exec_build: bool,
    pub system_activate: bool,
    pub profile_apply: bool,
    pub migration: bool,
    pub extension_runtime: bool,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            profile: "default-host".to_owned(),
            network_fetch: true,
            network_publish: true,
            local_editors: true,
            local_exec_build: true,
            system_activate: true,
            profile_apply: true,
            migration: true,
            extension_runtime: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            dir: "~/.config/elda/logs".to_owned(),
            level: 0,
        }
    }
}

#[must_use]
pub fn default_native_arch() -> String {
    match env::consts::ARCH {
        "x86_64" => "amd64".to_owned(),
        "x86" | "i686" | "i386" => "i386".to_owned(),
        "aarch64" => "arm64".to_owned(),
        "arm" | "armv7" | "armv7l" => "armhf".to_owned(),
        "riscv64" => "riscv64".to_owned(),
        "powerpc64" | "powerpc64le" => "ppc64le".to_owned(),
        other => other.to_owned(),
    }
}

#[must_use]
pub fn process_root_dir() -> PathBuf {
    env::var_os("ELDA_ROOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{
        Config, InstallPreference, LinkOptionMode, LoggingConfig, ProfileConfig,
        SubmissionAuthKind, SubmissionMode, default_link_strategy_priority, default_native_arch,
    };

    #[test]
    fn load_accepts_public_config_shape_and_reads_runtime_fields() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        let config_dir = tempdir.path().join("etc/elda");
        fs::create_dir_all(&config_dir).expect("config dir should exist");
        fs::write(
            config_dir.join("config.toml"),
            r#"
[defaults]
remote = "mirror-main"
cache_policy = "prefer"
origin_style = "tag"
install_preference = "source"
build_fallback = "local"
build_mode = "host"
activation = "auto"
prefix = "/opt/elda"
allow_system_mode = true
snapshot_tool = "snapper"
install_recommends = false
refresh_weak_deps = false

[privilege]
provider = "sudo"
preserve_env = true
interactive = false

[profile]
base = "yoka-desktop"
native_arch = "amd64"
foreign_arches = ["i386"]
init = "dinit"

[resolver.provider_preferences]
gl-provider = ["mesa-provider", "zink-provider"]

[logging]
dir = "~/.config/elda/logs"
level = 2

[flags.global]
wayland = true
x11 = false

[flags.profile.yoka-desktop]
pipewire = true

[flags.package.fsel]
wayland = true

[metadata]
link_option_mode = "list-options"
link_strategy_priority = ["elda-native", "nix_flake", "gentoo_ebuild", "make"]

[git]
tag_policy = "semver"
include_prereleases = false
strip_v_prefix = true
allow_date_versions = false
max_tags = 25
allowed_protocols = ["https", "ssh", "file", "http"]

[submission]
mode = "pr"
auto_open = true
auth = "token"
token_env = "ELDA_GITHUB_TOKEN"
api_base = "https://api.github.example"
remote_name = "upstream"
base_branch = "stable"

[submission.remotes.upstream]
auto_assign = true
auth = "ssh"
token_env = "ELDA_UPSTREAM_TOKEN"
api_base = "https://forge.example/api/v1"
base_branch = "release"

[daemon]
refresh = "30m"
notify_upgrades = true

[display]
show_origin = true
show_remote = true
"#,
        )
        .expect("config should be written");

        let config = Config::load(tempdir.path()).expect("config should load");

        assert_eq!(config.defaults.remote, "mirror-main");
        assert_eq!(config.defaults.build_mode, "host");
        assert_eq!(config.defaults.prefix, PathBuf::from("/opt/elda"));
        assert!(config.defaults.allow_system_mode);
        assert_eq!(config.defaults.snapshot_tool, "snapper");
        assert!(!config.defaults.install_recommends);
        assert!(!config.defaults.refresh_weak_deps);
        assert_eq!(
            config.defaults.install_preference,
            InstallPreference::Source
        );
        assert_eq!(
            config.metadata.link_option_mode,
            LinkOptionMode::ListOptions
        );
        assert_eq!(
            config.metadata.link_strategy_priority,
            vec![
                "elda-native".to_owned(),
                "nix_flake".to_owned(),
                "gentoo_ebuild".to_owned(),
                "make".to_owned(),
            ]
        );
        assert_eq!(config.git.tag_policy, "semver");
        assert!(!config.git.include_prereleases);
        assert!(config.git.strip_v_prefix);
        assert!(!config.git.allow_date_versions);
        assert_eq!(config.git.max_tags, 25);
        assert_eq!(
            config.git.allowed_protocols,
            vec![
                "https".to_owned(),
                "ssh".to_owned(),
                "file".to_owned(),
                "http".to_owned(),
            ]
        );
        assert_eq!(config.submission.mode, SubmissionMode::Pr);
        assert!(config.submission.auto_open);
        assert_eq!(config.submission.auth, SubmissionAuthKind::Token);
        assert_eq!(config.submission.token_env, "ELDA_GITHUB_TOKEN");
        assert_eq!(
            config.submission.api_base.as_deref(),
            Some("https://api.github.example")
        );
        assert_eq!(config.submission.remote_name(), "upstream");
        assert_eq!(config.submission.base_branch(), "stable");
        let resolved = config.submission.resolve_target();
        assert_eq!(resolved.remote_name, "upstream");
        assert_eq!(resolved.base_branch, "release");
        assert_eq!(resolved.auth, SubmissionAuthKind::Ssh);
        assert_eq!(resolved.token_env, "ELDA_UPSTREAM_TOKEN");
        assert_eq!(
            resolved.api_base.as_deref(),
            Some("https://forge.example/api/v1")
        );
        assert!(resolved.auto_assign);
        assert!(config.privilege.preserve_env);
        assert!(!config.privilege.interactive);
        assert_eq!(config.profile.base, "yoka-desktop");
        assert_eq!(config.profile.native_arch, "amd64");
        assert_eq!(config.profile.foreign_arches, vec!["i386"]);
        assert_eq!(config.profile.init, "dinit");
        assert_eq!(
            config.resolver.provider_preferences.get("gl-provider"),
            Some(&vec![
                "mesa-provider".to_owned(),
                "zink-provider".to_owned(),
            ])
        );
        assert_eq!(config.logging.dir, LoggingConfig::default().dir);
        assert_eq!(config.logging.level, 2);
        assert_eq!(config.flags.global.get("wayland"), Some(&true));
        assert_eq!(config.flags.global.get("x11"), Some(&false));
        assert_eq!(
            config
                .flags
                .profile
                .get("yoka-desktop")
                .and_then(|flags| flags.get("pipewire")),
            Some(&true)
        );
        assert_eq!(
            config
                .flags
                .package
                .get("fsel")
                .and_then(|flags| flags.get("wayland")),
            Some(&true)
        );
    }

    #[test]
    fn fixture_configs_load_against_runtime_schema() {
        for relative in [
            "config.toml",
            "su/config.toml",
            "fixtures/config/system-default.toml",
            "fixtures/config/prefix-source.toml",
            "fixtures/config/profile-defaults.toml",
            "fixtures/config/su-system.toml",
        ] {
            let tempdir = TempDir::new().expect("tempdir should exist");
            let config_dir = tempdir.path().join("etc/elda");
            fs::create_dir_all(&config_dir).expect("config dir should exist");
            let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(relative);
            fs::copy(source, config_dir.join("config.toml")).expect("fixture config should copy");

            let config = Config::load(tempdir.path()).expect("fixture config should load");
            assert!(!config.metadata.link_strategy_priority.is_empty());
        }
    }

    #[test]
    fn metadata_defaults_keep_operator_strategy_order() {
        let config = Config::default();

        assert_eq!(
            config.metadata.link_strategy_priority,
            default_link_strategy_priority()
        );
        assert_eq!(config.metadata.link_option_mode, LinkOptionMode::Priority);
        assert_eq!(config.git.max_tags, 50);
    }

    #[test]
    fn profile_defaults_do_not_invent_machine_shape() {
        let profile = ProfileConfig::default();

        assert!(profile.base.is_empty());
        assert_eq!(profile.native_arch, default_native_arch());
        assert!(profile.foreign_arches.is_empty());
        assert!(profile.init.is_empty());
    }
}
