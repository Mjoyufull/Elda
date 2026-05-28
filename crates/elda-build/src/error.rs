use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("build io failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("build json failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid build metadata: {0}")]
    Invalid(String),
    #[error("unsupported build input: {0}")]
    Unsupported(String),
    #[error("fetch failure: {0}")]
    Fetch(String),
    #[error("{program} failed while {context}: {stderr}")]
    CommandFailed {
        program: &'static str,
        context: String,
        stderr: String,
    },
}
