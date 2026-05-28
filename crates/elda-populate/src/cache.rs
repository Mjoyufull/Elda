use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use ureq::Error as UreqError;

use elda_repo::CacheDocument;

use crate::error::PopulateError;
use crate::manifest::CacheSeedEntry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalPayload {
    pub(crate) package_name: String,
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
}

pub(crate) fn local_cache_directory(base_url: &str) -> Option<PathBuf> {
    if let Some(path) = base_url.strip_prefix("file://") {
        return Some(PathBuf::from(path));
    }
    if base_url.starts_with("http://") || base_url.starts_with("https://") {
        return None;
    }
    Some(PathBuf::from(base_url))
}

pub(crate) fn local_payloads(cache_pkg_dir: &Path) -> Result<Vec<LocalPayload>, PopulateError> {
    if !cache_pkg_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(cache_pkg_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    let mut payloads = Vec::new();
    for path in entries {
        if path.extension().and_then(|ext| ext.to_str()) != Some("zst") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".pkg.tar.zst") {
            continue;
        }
        let package_name = name
            .split_once('-')
            .map(|(package_name, _)| package_name)
            .unwrap_or(name)
            .to_owned();
        payloads.push(LocalPayload {
            package_name,
            sha256: sha256_file(&path)?,
            path,
        });
    }

    Ok(payloads)
}

pub(crate) fn ensure_cached_blob(
    cache: &CacheDocument,
    digest: &str,
    source_url: &str,
    package_name: &str,
    dry_run: bool,
) -> Result<CacheSeedEntry, PopulateError> {
    let cache_object = format!("{}/{}", cache.base_url.trim_end_matches('/'), digest);
    let mode = if local_cache_directory(&cache.base_url).is_some() {
        if dry_run { "dry-run" } else { "local-write" }
    } else {
        "manifest-only"
    };
    let entry = CacheSeedEntry {
        package_name: package_name.to_owned(),
        sha256: digest.to_owned(),
        source_url: source_url.to_owned(),
        cache_name: cache.name.clone(),
        cache_base_url: cache.base_url.clone(),
        cache_object,
        mode: mode.to_owned(),
    };

    let Some(cache_dir) = local_cache_directory(&cache.base_url) else {
        return Ok(entry);
    };
    if dry_run {
        return Ok(entry);
    }

    fs::create_dir_all(&cache_dir)?;
    let target = cache_dir.join(digest);
    if target.exists() && sha256_file(&target)? == digest {
        return Ok(entry);
    }
    if target.exists() {
        fs::remove_file(&target)?;
    }

    let temp = fetch_to_temp(source_url, cache_dir.as_path())?;
    let actual = sha256_file(temp.path())?;
    if actual != digest {
        return Err(PopulateError::ShaMismatch {
            path: temp.path().to_path_buf(),
            expected: digest.to_owned(),
            actual,
        });
    }
    temp.persist(&target)
        .map_err(|error| PopulateError::Io(error.error))?;

    Ok(entry)
}

fn fetch_to_temp(source_url: &str, temp_root: &Path) -> Result<NamedTempFile, PopulateError> {
    let temp = NamedTempFile::new_in(temp_root)?;
    if let Some(path) = source_url.strip_prefix("file://") {
        fs::copy(path, temp.path())?;
        return Ok(temp);
    }

    let local_path = Path::new(source_url);
    if local_path.exists() {
        fs::copy(local_path, temp.path())?;
        return Ok(temp);
    }

    if source_url.starts_with("http://") || source_url.starts_with("https://") {
        let response = ureq::get(source_url).call().map_err(http_error)?;
        let mut reader = response.into_reader();
        let mut file = fs::File::create(temp.path())?;
        copy(&mut reader, &mut file)?;
        return Ok(temp);
    }

    Err(PopulateError::Operator(format!(
        "unsupported payload source URL `{source_url}`"
    )))
}

fn http_error(error: UreqError) -> PopulateError {
    match error {
        UreqError::Status(code, response) => PopulateError::Fetch(format!(
            "http {code} while fetching `{}`",
            response.get_url()
        )),
        other => PopulateError::Fetch(other.to_string()),
    }
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, PopulateError> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
