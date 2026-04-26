use std::fs;
use std::path::{Path, PathBuf};

use crate::error::RepoError;
use crate::model::{CacheDocument, DEFAULT_REMOTE_CHANNEL, RemoteDocument, TrustMode};

pub fn add_remote(remotes_dir: &Path, input: &str) -> Result<RemoteDocument, RepoError> {
    fs::create_dir_all(remotes_dir)?;
    let (name, index_url) = parse_named_url(input)?;
    save_remote(
        remotes_dir,
        RemoteDocument {
            name,
            index_url,
            channel: DEFAULT_REMOTE_CHANNEL.to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Tofu,
            trusted_keys: Vec::new(),
            allow_stale: false,
            priority: 100,
        },
    )
}

pub fn save_remote(
    remotes_dir: &Path,
    document: RemoteDocument,
) -> Result<RemoteDocument, RepoError> {
    fs::create_dir_all(remotes_dir)?;
    let path = remotes_dir.join(format!("{}.toml", document.name));
    fs::write(path, toml::to_string_pretty(&document)?)?;

    Ok(document)
}

pub fn add_cache(caches_dir: &Path, input: &str) -> Result<CacheDocument, RepoError> {
    fs::create_dir_all(caches_dir)?;
    let (name, base_url) = parse_named_url(input)?;
    save_cache(
        caches_dir,
        CacheDocument {
            name,
            base_url,
            priority: 100,
            enabled: true,
        },
    )
}

pub fn save_cache(caches_dir: &Path, document: CacheDocument) -> Result<CacheDocument, RepoError> {
    fs::create_dir_all(caches_dir)?;
    let path = caches_dir.join(format!("{}.toml", document.name));
    fs::write(path, toml::to_string_pretty(&document)?)?;

    Ok(document)
}

pub fn list_caches(caches_dir: &Path) -> Result<Vec<CacheDocument>, RepoError> {
    load_documents::<CacheDocument>(caches_dir)
}

pub fn list_remotes(remotes_dir: &Path) -> Result<Vec<RemoteDocument>, RepoError> {
    load_documents::<RemoteDocument>(remotes_dir)
}

pub fn load_remote(remotes_dir: &Path, name: &str) -> Result<Option<RemoteDocument>, RepoError> {
    Ok(list_remotes(remotes_dir)?
        .into_iter()
        .find(|remote| remote.name == name))
}

fn load_documents<T>(directory: &Path) -> Result<Vec<T>, RepoError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    entries.sort();

    let mut documents = Vec::new();
    for path in entries {
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }
        let content = fs::read_to_string(path)?;
        documents.push(toml::from_str(&content)?);
    }

    Ok(documents)
}

fn parse_named_url(input: &str) -> Result<(String, String), RepoError> {
    if let Some((name, url)) = input.split_once('=') {
        let name = sanitize_name(name);
        if name.is_empty() {
            return Err(RepoError::Parse(
                "document name must not be empty before `=`".to_owned(),
            ));
        }
        if url.trim().is_empty() {
            return Err(RepoError::Parse(
                "document url must not be empty after `=`".to_owned(),
            ));
        }
        return Ok((name, url.trim().to_owned()));
    }

    if !looks_like_url(input) {
        return Err(RepoError::Parse(
            "expected `<name>=<url>` or a bare URL".to_owned(),
        ));
    }

    let name = derive_name_from_url(input);
    Ok((name, input.trim().to_owned()))
}

fn looks_like_url(input: &str) -> bool {
    input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://")
}

fn derive_name_from_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    let tail = trimmed.rsplit('/').next().unwrap_or("remote");
    let tail = tail
        .trim_end_matches(".json")
        .trim_end_matches(".toml")
        .trim_end_matches(".idx");
    let sanitized = sanitize_name(tail);

    if sanitized.is_empty() {
        "remote".to_owned()
    } else {
        sanitized
    }
}

fn sanitize_name(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use crate::store::{add_cache, add_remote, list_caches, list_remotes};

    #[test]
    fn add_remote_derives_name_from_bare_url() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        let remote = add_remote(tempdir.path(), "https://example.invalid/yoka-main.toml")
            .expect("remote should be created");

        assert_eq!(remote.name, "yoka-main");
        assert_eq!(remote.priority, 100);
    }

    #[test]
    fn add_cache_persists_document_and_can_be_listed() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        add_cache(tempdir.path(), "lan=https://cache.invalid/elda")
            .expect("cache should be created");

        let caches = list_caches(tempdir.path()).expect("cache listing should succeed");

        assert_eq!(caches.len(), 1);
        assert_eq!(caches[0].name, "lan");
    }

    #[test]
    fn list_remotes_round_trips_documents() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        add_remote(tempdir.path(), "main=https://example.invalid/index.toml")
            .expect("remote should be created");

        let remotes = list_remotes(tempdir.path()).expect("remote listing should succeed");

        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "main");
    }
}
