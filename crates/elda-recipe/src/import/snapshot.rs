use crate::error::RecipeError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotType {
    Void,
    Gentoo,
    Elda,
}

impl SnapshotType {
    pub fn display(&self) -> &'static str {
        match self {
            SnapshotType::Void => "Void Linux (xbps-src)",
            SnapshotType::Gentoo => "Gentoo Overlay (ebuild)",
            SnapshotType::Elda => "Elda Monorepo",
        }
    }
}

pub fn detect_snapshot_type(dir: &Path) -> Option<SnapshotType> {
    // Void detection: srcpkgs directory
    if dir.join("srcpkgs").is_dir() {
        return Some(SnapshotType::Void);
    }

    // Gentoo detection: profiles/repo_name file
    if dir.join("profiles/repo_name").is_file() {
        return Some(SnapshotType::Gentoo);
    }

    // Elda Monorepo detection:
    // We look for a 'recipes' directory that contains subdirectories with pkg.lua
    let recipes_dir = dir.join("recipes");
    if recipes_dir.is_dir() && !dir.join("pkg.lua").is_file() {
        // Confirm it has at least one pkg.lua in a child
        if let Ok(entries) = fs::read_dir(&recipes_dir)
            && entries
                .flatten()
                .any(|e| e.path().is_dir() && e.path().join("pkg.lua").is_file())
        {
            return Some(SnapshotType::Elda);
        }
    }

    None
}

pub struct RecipeCandidate {
    pub name: String,
    pub source_path: PathBuf,
    pub rel_path: PathBuf,
}

pub fn scan_snapshot(dir: &Path, kind: SnapshotType) -> Result<Vec<RecipeCandidate>, RecipeError> {
    let mut candidates = Vec::new();

    match kind {
        SnapshotType::Void => {
            let srcpkgs = dir.join("srcpkgs");
            if let Ok(entries) = fs::read_dir(srcpkgs) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    // Skip hidden dirs
                    if path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|s| s.starts_with('.'))
                    {
                        continue;
                    }
                    if path.is_dir()
                        && path.join("template").is_file()
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        let rel_path = path.strip_prefix(dir).unwrap_or(&path).to_path_buf();
                        candidates.push(RecipeCandidate {
                            name: name.to_owned(),
                            source_path: path,
                            rel_path,
                        });
                    }
                }
            }
        }
        SnapshotType::Gentoo => {
            // Gentoo overlays have categories
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if name.starts_with('.')
                            || name == "profiles"
                            || name == "metadata"
                            || name == "scripts"
                            || name == "eclass"
                            || name == "licenses"
                        {
                            continue;
                        }

                        // Scan category for packages
                        if let Ok(pkg_entries) = fs::read_dir(&path) {
                            for pkg_entry in pkg_entries.flatten() {
                                let pkg_path = pkg_entry.path();
                                if pkg_path.is_dir()
                                    && let Ok(files) = fs::read_dir(&pkg_path)
                                    && files.flatten().any(|f| {
                                        f.path().extension().is_some_and(|ext| ext == "ebuild")
                                    })
                                    && let Some(pkg_name) =
                                        pkg_path.file_name().and_then(|n| n.to_str())
                                {
                                    let rel_path = pkg_path
                                        .strip_prefix(dir)
                                        .unwrap_or(&pkg_path)
                                        .to_path_buf();
                                    candidates.push(RecipeCandidate {
                                        name: pkg_name.to_owned(),
                                        source_path: pkg_path,
                                        rel_path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        SnapshotType::Elda => {
            let recipes_dir = dir.join("recipes");
            if let Ok(entries) = fs::read_dir(recipes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path.join("pkg.lua").is_file()
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        let rel_path = path.strip_prefix(dir).unwrap_or(&path).to_path_buf();
                        candidates.push(RecipeCandidate {
                            name: name.to_owned(),
                            source_path: path,
                            rel_path,
                        });
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(candidates)
}
