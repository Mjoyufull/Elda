use crate::privilege::PrivilegeRequest;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("config io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse failure: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("json failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("build failure: {0}")]
    Build(#[from] elda_build::BuildError),
    #[error("install failure: {0}")]
    Install(#[from] elda_install::InstallError),
    #[error("repo failure: {0}")]
    Repo(#[from] elda_repo::RepoError),
    #[error("recipe failure: {0}")]
    Recipe(#[from] elda_recipe::RecipeError),
    #[error("state failure: {0}")]
    Db(#[from] elda_db::DbError),
    #[error("{0}")]
    PrivilegeRequired(PrivilegeRequest),
    #[error("{0}")]
    Operator(String),
}
