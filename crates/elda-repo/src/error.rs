use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("repo io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("repo snapshot is missing; run `elda sync` first")]
    SnapshotMissing,
    #[error("repo parse failure: {0}")]
    Parse(String),
    #[error("repo json failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("repo regex failure: {0}")]
    Regex(#[from] regex::Error),
    #[error("repo trust failure: {0}")]
    Trust(String),
    #[error("repo toml encode failure: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("repo toml decode failure: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("repo http failure: {0}")]
    Http(String),
}
