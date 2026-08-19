//! Shared description of a local artifact file that Elda was handed directly.
//!
//! An operator can point Elda at a release archive they already downloaded
//! (`elda a ~/Downloads/delta-linux-x86_64.tar.gz`). Surveying that file needs
//! archive decoders, and rendering a recipe from the survey does not, so the
//! survey result is a plain data type both sides can see.

use serde::{Deserialize, Serialize};

/// Container format, decided by content magic rather than by file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactFormat {
    Tar,
    TarGz,
    TarXz,
    TarZst,
    TarBz2,
    Zip,
    Elf,
    AppImage,
}

impl ArtifactFormat {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarXz => "tar.xz",
            Self::TarZst => "tar.zst",
            Self::TarBz2 => "tar.bz2",
            Self::Zip => "zip",
            Self::Elf => "elf",
            Self::AppImage => "appimage",
        }
    }

    /// True when the artifact is an archive whose members must be surveyed.
    #[must_use]
    pub fn is_archive(self) -> bool {
        !matches!(self, Self::Elf | Self::AppImage)
    }

    /// The Elda source kind a generated recipe should declare for this format.
    #[must_use]
    pub fn source_kind(self) -> &'static str {
        match self {
            Self::AppImage => "appimage",
            _ => "url_archive",
        }
    }
}

/// What Elda decided a surveyed archive member is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactEntryKind {
    Executable,
    Library,
    ManPage,
    Completion,
    Desktop,
    Icon,
    Metainfo,
    License,
    Doc,
    /// Recognised but not installable on this platform (`.exe`, `.dll`, `.ps1`, …).
    ForeignPlatform,
    Other,
}

impl ArtifactEntryKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Executable => "executable",
            Self::Library => "library",
            Self::ManPage => "man page",
            Self::Completion => "completion",
            Self::Desktop => "desktop entry",
            Self::Icon => "icon",
            Self::Metainfo => "appstream",
            Self::License => "license",
            Self::Doc => "doc",
            Self::ForeignPlatform => "foreign platform",
            Self::Other => "other",
        }
    }

    /// Foreign-platform members are recorded and then dropped rather than staged.
    #[must_use]
    pub fn is_droppable(self) -> bool {
        matches!(self, Self::ForeignPlatform)
    }
}

/// One classified member of a surveyed archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactEntry {
    /// Path as stored in the archive, before `strip_components` is applied.
    pub path: String,
    pub kind: ArtifactEntryKind,
    pub size: u64,
    pub executable: bool,
}

/// The result of inspecting a local artifact without executing it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSurvey {
    pub file_name: String,
    pub format: ArtifactFormat,
    pub sha256: String,
    pub size: u64,
    /// Leading path components shared by every member, so staging can strip them.
    pub strip_components: u32,
    /// Inferred package name, from the archive root or the file name.
    pub name: Option<String>,
    /// Inferred upstream version, when the archive root or file name carries one.
    pub version: Option<String>,
    pub entries: Vec<ArtifactEntry>,
}

impl ArtifactSurvey {
    /// Members of one classification, in archive order.
    #[must_use]
    pub fn by_kind(&self, kind: ArtifactEntryKind) -> Vec<&ArtifactEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == kind)
            .collect()
    }

    /// Launcher candidates: executable members that are not libraries.
    #[must_use]
    pub fn executables(&self) -> Vec<&ArtifactEntry> {
        self.by_kind(ArtifactEntryKind::Executable)
    }

    /// The single launcher, when the archive is unambiguous about it.
    ///
    /// Ambiguity is not resolved by guessing; the caller must ask for an
    /// explicit `binary` instead.
    #[must_use]
    pub fn sole_executable(&self) -> Option<&ArtifactEntry> {
        match self.executables().as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }

    #[must_use]
    pub fn dropped(&self) -> Vec<&ArtifactEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind.is_droppable())
            .collect()
    }
}

/// Where a local artifact came from, recorded so the package can be re-fetched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "value")]
pub enum ArtifactAcquisition {
    /// Downloaded from this URL; the generated recipe pins it plus the digest.
    Url(String),
    /// No upstream. The package is pinned to the local file and cannot upgrade.
    LocalOnly,
}

impl ArtifactAcquisition {
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Url(url) => Some(url),
            Self::LocalOnly => None,
        }
    }
}
