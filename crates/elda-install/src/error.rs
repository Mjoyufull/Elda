use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("install io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("install json failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("build failure: {0}")]
    Build(#[from] elda_build::BuildError),
    #[error("state failure: {0}")]
    Db(#[from] elda_db::DbError),
    #[error("unsupported install request: {0}")]
    Unsupported(String),
    #[error("recovery is required before the next mutation: {0} pending journal(s) found")]
    PendingRecovery(usize),
    #[error("journal failure: {0}")]
    Journal(String),
    #[error("state archive failure: {0}")]
    StateArchive(String),
    #[error("path conflict on `{path}` with installed package `{owner}`")]
    PathConflict { path: String, owner: String },
    #[error("unmanaged path collision on `{0}`")]
    UnmanagedPathCollision(String),
    #[error("package `{0}` is already installed")]
    AlreadyInstalled(String),
    #[error("package `{0}` is not installed")]
    NotInstalled(String),
}
