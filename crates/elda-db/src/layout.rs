use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::DbError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallationMode {
    System,
    Prefix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateLayout {
    pub root_dir: PathBuf,
    pub prefix: PathBuf,
    pub mode: InstallationMode,
    pub config_dir: PathBuf,
    pub recipes_dir: PathBuf,
    pub remotes_dir: PathBuf,
    pub caches_dir: PathBuf,
    pub extensions_dir: PathBuf,
    pub data_dir: PathBuf,
    pub db_dir: PathBuf,
    pub db_path: PathBuf,
    pub world_path: PathBuf,
    pub journal_dir: PathBuf,
    pub manifests_dir: PathBuf,
    pub state_dir: PathBuf,
    pub current_state_path: PathBuf,
    pub states_dir: PathBuf,
    pub cache_src_dir: PathBuf,
    pub cache_pkg_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub lock_path: PathBuf,
}

impl StateLayout {
    #[must_use]
    pub fn new(root_dir: impl Into<PathBuf>, prefix: impl Into<PathBuf>) -> Self {
        let root_dir = root_dir.into();
        let prefix = prefix.into();
        let config_dir = root_dir.join("etc/elda");

        if prefix == Path::new("/usr") {
            let data_dir = root_dir.join("var/lib/elda");
            let db_dir = data_dir.join("db");
            let state_dir = data_dir.join("state");
            let cache_dir = root_dir.join("var/cache/elda");

            return Self {
                root_dir: root_dir.clone(),
                prefix,
                mode: InstallationMode::System,
                recipes_dir: config_dir.join("recipes"),
                remotes_dir: config_dir.join("remotes.d"),
                caches_dir: config_dir.join("caches.d"),
                extensions_dir: config_dir.join("extensions.d"),
                config_dir,
                db_path: db_dir.join("elda.sqlite"),
                world_path: db_dir.join("world"),
                journal_dir: db_dir.join("journal"),
                manifests_dir: db_dir.join("manifests"),
                current_state_path: state_dir.join("current"),
                states_dir: data_dir.join("states"),
                cache_src_dir: cache_dir.join("src"),
                cache_pkg_dir: cache_dir.join("pkgs"),
                tmp_dir: root_dir.join("var/tmp/elda"),
                lock_path: db_dir.join("mutation.lock"),
                data_dir,
                db_dir,
                state_dir,
            };
        }

        let namespace = prefix_namespace(&prefix);
        let prefix_root = root_dir.join("var/lib/elda/prefixes").join(&namespace);
        let db_dir = prefix_root.join("db");
        let state_dir = prefix_root.join("state");
        let cache_root = root_dir.join("var/cache/elda/prefixes").join(&namespace);

        Self {
            root_dir: root_dir.clone(),
            prefix,
            mode: InstallationMode::Prefix,
            recipes_dir: config_dir.join("recipes"),
            remotes_dir: config_dir.join("remotes.d"),
            caches_dir: config_dir.join("caches.d"),
            extensions_dir: config_dir.join("extensions.d"),
            config_dir,
            db_path: db_dir.join("elda.sqlite"),
            world_path: db_dir.join("world"),
            journal_dir: db_dir.join("journal"),
            manifests_dir: db_dir.join("manifests"),
            current_state_path: state_dir.join("current"),
            states_dir: prefix_root.join("states"),
            cache_src_dir: cache_root.join("src"),
            cache_pkg_dir: cache_root.join("pkgs"),
            tmp_dir: root_dir.join("var/tmp/elda/prefixes").join(namespace),
            lock_path: db_dir.join("mutation.lock"),
            data_dir: prefix_root,
            db_dir,
            state_dir,
        }
    }

    pub fn ensure_exists(&self) -> Result<(), DbError> {
        for directory in [
            &self.config_dir,
            &self.recipes_dir,
            &self.remotes_dir,
            &self.caches_dir,
            &self.extensions_dir,
            &self.db_dir,
            &self.journal_dir,
            &self.manifests_dir,
            &self.state_dir,
            &self.states_dir,
            &self.cache_src_dir,
            &self.cache_pkg_dir,
            &self.tmp_dir,
        ] {
            fs::create_dir_all(directory)?;
        }

        if !self.world_path.exists() {
            fs::write(&self.world_path, "")?;
        }
        if !self.current_state_path.exists() {
            fs::write(&self.current_state_path, "")?;
        }

        Ok(())
    }
}

fn prefix_namespace(prefix: &Path) -> String {
    let raw = prefix.to_string_lossy();
    let mut encoded = String::from("prefix-");
    for byte in raw.as_bytes() {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}
