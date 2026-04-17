use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};

use ureq::Error as UreqError;

use crate::cache_meta::{CacheEntryKind, record_cache_access};
use crate::manifest::sha256_file;
use crate::{BinaryCache, BuildError};

pub(super) fn fetch_binary_source(
    source_url: &str,
    expected_sha256: &str,
    cache_src_dir: &Path,
    configured_caches: &[BinaryCache],
    offline: bool,
) -> Result<PathBuf, BuildError> {
    fs::create_dir_all(cache_src_dir)?;
    let cached_payload_path = cache_src_dir.join(expected_sha256);

    if matches_expected_payload(&cached_payload_path, expected_sha256)? {
        record_cache_access(&cached_payload_path, CacheEntryKind::SourceArtifact)?;
        return Ok(cached_payload_path);
    }
    remove_if_exists(&cached_payload_path)?;

    for cache in sorted_caches(configured_caches) {
        if offline && !is_local_location(&cache.base_url) {
            continue;
        }

        let candidate = cache_payload_url(&cache.base_url, expected_sha256);
        if try_copy_location(&candidate, &cached_payload_path, offline)? {
            if matches_expected_payload(&cached_payload_path, expected_sha256)? {
                record_cache_access(&cached_payload_path, CacheEntryKind::SourceArtifact)?;
                return Ok(cached_payload_path);
            }
            remove_if_exists(&cached_payload_path)?;
        }
    }

    if offline {
        return Err(BuildError::Unsupported(format!(
            "offline mode could not find payload `{expected_sha256}` in the local cache or local cache nodes"
        )));
    }

    copy_location(source_url, &cached_payload_path, false)?;
    if matches_expected_payload(&cached_payload_path, expected_sha256)? {
        record_cache_access(&cached_payload_path, CacheEntryKind::SourceArtifact)?;
        return Ok(cached_payload_path);
    }
    remove_if_exists(&cached_payload_path)?;

    Err(BuildError::Invalid(format!(
        "downloaded source sha256 mismatch for `{source_url}`; expected `{expected_sha256}`"
    )))
}

fn sorted_caches(configured_caches: &[BinaryCache]) -> Vec<BinaryCache> {
    let mut caches = configured_caches.to_vec();
    caches.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.name.cmp(&right.name))
    });
    caches
}

fn cache_payload_url(base_url: &str, expected_sha256: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), expected_sha256)
}

fn matches_expected_payload(path: &Path, expected_sha256: &str) -> Result<bool, BuildError> {
    if !path.exists() {
        return Ok(false);
    }

    Ok(sha256_file(path)? == expected_sha256)
}

fn remove_if_exists(path: &Path) -> Result<(), BuildError> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn try_copy_location(
    source_url: &str,
    destination: &Path,
    offline: bool,
) -> Result<bool, BuildError> {
    match copy_location(source_url, destination, offline) {
        Ok(()) => Ok(true),
        Err(BuildError::Fetch(_)) | Err(BuildError::Unsupported(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

fn copy_location(source_url: &str, destination: &Path, offline: bool) -> Result<(), BuildError> {
    if let Some(path) = source_url.strip_prefix("file://") {
        let source_path = Path::new(path);
        if !source_path.exists() {
            return Err(BuildError::Unsupported(format!(
                "local cache path `{}` does not exist",
                source_path.display()
            )));
        }
        fs::copy(source_path, destination)?;
        return Ok(());
    }

    let local_path = Path::new(source_url);
    if local_path.exists() {
        fs::copy(local_path, destination)?;
        return Ok(());
    }

    if source_url.starts_with("http://") || source_url.starts_with("https://") {
        if offline {
            return Err(BuildError::Unsupported(format!(
                "offline mode cannot fetch remote archive `{source_url}`"
            )));
        }

        let response = ureq::get(source_url).call().map_err(http_error)?;
        let mut reader = response.into_reader();
        let mut file = fs::File::create(destination)?;
        copy(&mut reader, &mut file)?;
        return Ok(());
    }

    Err(BuildError::Unsupported(format!(
        "unsupported binary source URL `{source_url}`"
    )))
}

fn http_error(error: UreqError) -> BuildError {
    match error {
        UreqError::Status(code, response) => BuildError::Fetch(format!(
            "http {code} while fetching `{}`",
            response.get_url()
        )),
        other => BuildError::Fetch(other.to_string()),
    }
}

fn is_local_location(location: &str) -> bool {
    location.starts_with("file://") || Path::new(location).exists()
}
