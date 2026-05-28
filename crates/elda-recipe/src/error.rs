use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("recipe io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("recipe json failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("recipe parse failure: {0}")]
    Parse(String),
    #[error("invalid recipe input: {0}")]
    InvalidInput(String),
}
