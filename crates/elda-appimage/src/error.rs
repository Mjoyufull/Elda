use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppImageError {
    #[error("failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("file is too small to be an ELF AppImage")]
    TooSmall,
    #[error("not a Linux ELF binary")]
    NotElf,
    #[error("unsupported AppImage generation (expected Type 2 magic AI\\x02 at bytes 8–10)")]
    UnsupportedGeneration,
    #[error("ELF layout parse error: {0}")]
    ElfParse(String),
    #[error(
        "embedded filesystem is not SquashFS at computed offsets (DwarFS or exotic layouts are unsupported)"
    )]
    SquashfsNotFound,
    #[error("SquashFS reader error: {0}")]
    Squashfs(String),
    #[error("desktop entry parse error: {0}")]
    DesktopParse(String),
}

impl AppImageError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
