use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportOptions {
    pub strategy_priority: Vec<String>,
    /// When empty, Elda uses the built-in default order (archives and distro packages before
    /// `app-image`). When non-empty, only formats listed here are considered for automatic git
    /// release binary selection; order is preference (earlier wins). Omit `app-image` to disable
    /// AppImage payloads entirely for auto-selection.
    pub release_binary_format_priority: Vec<String>,
    pub selected_source_option: Option<usize>,
    pub git_ref: Option<GitRefRequest>,
    pub replace: bool,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRefRequest {
    pub kind: GitRefKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRefKind {
    Branch,
    Tag,
    Rev,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ImportResult {
    Single(ImportReport),
    Bulk(SnapshotImportReport),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotImportReport {
    pub source_url: String,
    pub replace: bool,
    pub source_commit: Option<String>,
    pub repository_type: String,
    pub discovered: usize,
    pub excluded: usize,
    pub skipped_existing: usize,
    pub to_import: usize,
    pub generated_recipes: Vec<String>,
    pub staging_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub recipe_name: String,
    pub recipe_dir: PathBuf,
    pub source_options: Vec<SourceOptionReport>,
    pub selected_source_option: Option<SourceOptionReport>,
    pub imported_pkg_lua: bool,
    pub imported_build_lua: bool,
    pub imported_patches: bool,
    pub generated_pkg_lua: bool,
    pub generated_build_lua: bool,
    pub imported_legacy_pkgdeps: bool,
    pub imported_legacy_bldit: bool,
    pub wrote_legacy_summary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct LegacyPkgdep {
    pub(super) raw: String,
    pub(super) source: String,
    pub(super) tag: Option<String>,
    pub(super) package_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceOptionReport {
    pub index: usize,
    pub strategy: String,
    pub source_kind: String,
    pub lane: String,
    pub confidence: String,
    pub summary: String,
    pub selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
    pub checksum_available: bool,
}
