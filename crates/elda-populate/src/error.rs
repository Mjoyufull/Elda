use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum PopulateError {
    #[error("{0}")]
    Operator(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Db(#[from] elda_db::DbError),
    #[error(transparent)]
    Repo(#[from] elda_repo::RepoError),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("http fetch failed: {0}")]
    Fetch(String),
    #[error("payload sha256 mismatch for `{path}`: expected `{expected}`, got `{actual}`")]
    ShaMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
}
