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
    #[error(
        "this is a Type 1 AppImage (magic AI\\x01); its ISO 9660 payload is not readable by Elda, repackage it as Type 2 or extract it with the upstream runtime"
    )]
    TypeOneUnsupported,
    #[error("ELF layout parse error: {0}")]
    ElfParse(String),
    #[error(
        "no embedded filesystem found at the computed offsets (expected SquashFS `hsqs` or DwarFS `DWARFS`)"
    )]
    PayloadNotFound,
    #[error(
        "embedded filesystem at offset {offset} is DwarFS, which this reader cannot decode; inspect it with the upstream runtime's `--appimage-dwarfsextract`"
    )]
    DwarfsPayload { offset: u64 },
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
