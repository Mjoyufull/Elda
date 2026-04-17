use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub recipe_name: String,
    pub recipe_dir: PathBuf,
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
