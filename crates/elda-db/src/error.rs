use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("mutation attempted through a read-only database handle")]
    ReadOnly,
    #[error("io failure: {0}")]
    Io(#[from] io::Error),
    #[error("sqlite failure: {0}")]
    Sql(#[from] rusqlite::Error),
}
