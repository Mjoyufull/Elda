use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use elda_db::StateLayout;

use crate::error::PopulateError;

const DEFAULT_PREFIX: &str = "/usr";

pub(crate) fn state_layout(
    root: Option<PathBuf>,
    prefix: Option<PathBuf>,
) -> Result<StateLayout, PopulateError> {
    let root_dir = root.unwrap_or_else(process_root_dir);
    let prefix = match prefix {
        Some(prefix) => prefix,
        None => {
            load_prefix_from_config(&root_dir)?.unwrap_or_else(|| PathBuf::from(DEFAULT_PREFIX))
        }
    };

    Ok(StateLayout::new(root_dir, prefix))
}

fn process_root_dir() -> PathBuf {
    env::var_os("ELDA_ROOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn load_prefix_from_config(root: &Path) -> Result<Option<PathBuf>, PopulateError> {
    let config_path = root.join("etc/elda/config.toml");
    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(config_path)?;
    let document = toml::from_str::<toml::Value>(&content)?;
    Ok(document
        .get("defaults")
        .and_then(|defaults| defaults.get("prefix"))
        .and_then(toml::Value::as_str)
        .map(PathBuf::from))
}
