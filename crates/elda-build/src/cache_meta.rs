use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::BuildError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CacheEntryKind {
    SourceArtifact,
    PackagePayload,
    PackageManifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    pub schema_version: u32,
    pub kind: CacheEntryKind,
    pub last_access_unix: u64,
}

pub fn record_cache_access(path: &Path, kind: CacheEntryKind) -> Result<(), BuildError> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = CacheEntryMetadata {
        schema_version: 1,
        kind,
        last_access_unix: current_unix_timestamp()?,
    };
    fs::write(
        cache_metadata_path(path),
        serde_json::to_vec_pretty(&metadata)?,
    )?;

    Ok(())
}

pub fn load_cache_metadata(path: &Path) -> Result<Option<CacheEntryMetadata>, BuildError> {
    let metadata_path = cache_metadata_path(path);
    if !metadata_path.exists() {
        return Ok(None);
    }

    let content = fs::read(&metadata_path)?;
    Ok(Some(serde_json::from_slice(&content)?))
}

#[must_use]
pub fn cache_metadata_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.eldameta.json", path.display()))
}

fn current_unix_timestamp() -> Result<u64, BuildError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            BuildError::Invalid(format!("system clock is before unix epoch: {error}"))
        })?
        .as_secs())
}
