use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::PopulateError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CacheSeedEntry {
    pub(crate) package_name: String,
    pub(crate) sha256: String,
    pub(crate) source_url: String,
    pub(crate) cache_name: String,
    pub(crate) cache_base_url: String,
    pub(crate) cache_object: String,
    pub(crate) mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CacheSeedManifest {
    pub(crate) entries: Vec<CacheSeedEntry>,
}

pub(crate) fn write_manifest(
    path: &Path,
    entries: Vec<CacheSeedEntry>,
) -> Result<PathBuf, PopulateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let manifest = CacheSeedManifest { entries };
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(path.to_path_buf())
}
