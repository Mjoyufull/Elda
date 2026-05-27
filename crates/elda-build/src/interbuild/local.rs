use std::path::{Path, PathBuf};

use elda_recipe::{RecipeDocument, ScalarValue};

/// Recipe trees imported from bulk metadata snapshots materialize interbuild
/// files locally; builds must not re-clone the snapshot repository for metadata.
pub fn local_interbuild_root(recipe: &RecipeDocument) -> Option<PathBuf> {
    snapshot_rev(recipe)?;
    let recipe_dir = recipe.path.parent()?;
    match recipe.package.source.kind.as_str() {
        "xbps_template" if recipe_dir.join("template").is_file() => Some(recipe_dir.to_path_buf()),
        "aur_pkgbuild" if recipe_dir.join("PKGBUILD").is_file() => Some(recipe_dir.to_path_buf()),
        "nix_flake" if recipe_dir.join("flake.nix").is_file() => Some(recipe_dir.to_path_buf()),
        "gentoo_overlay" if has_ebuild(recipe_dir) => Some(recipe_dir.to_path_buf()),
        _ => None,
    }
}

pub fn snapshot_rev(recipe: &RecipeDocument) -> Option<String> {
    match recipe.package.source.fields.get("snapshot_rev") {
        Some(ScalarValue::String(value)) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn has_ebuild(dir: &Path) -> bool {
    dir.read_dir()
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                == Some("ebuild")
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use std::collections::BTreeMap;

    use elda_recipe::{PackageDefinition, RecipeDocument, ScalarValue, SourceDefinition};
    use tempfile::TempDir;

    use super::{local_interbuild_root, snapshot_rev};

    fn recipe_with_template(dir: &Path, kind: &str) -> RecipeDocument {
        RecipeDocument {
            path: dir.join("pkg.lua"),
            package: PackageDefinition {
                name: "demo".to_owned(),
                description: None,
                licenses: Vec::new(),
                upstream: None,
                epoch: 0,
                version: "1.0.0".to_owned(),
                rel: 1,
                arch: vec!["amd64".to_owned()],
                kind: "package".to_owned(),
                source: SourceDefinition {
                    kind: kind.to_owned(),
                    fields: BTreeMap::new(),
                    lanes: BTreeMap::new(),
                    default_lane: None,
                    github_release_assets: BTreeMap::new(),
                },
                depends: Vec::new(),
                makedepends: Vec::new(),
                checkdepends: Vec::new(),
                recommends: Vec::new(),
                suggests: Vec::new(),
                supplements: Vec::new(),
                enhances: Vec::new(),
                provides: Vec::new(),
                conflicts: Vec::new(),
                replaces: Vec::new(),
                conffiles: Vec::new(),
                sysusers: None,
                tmpfiles: None,
                alternatives: None,
                hooks: None,
                provider_assets: None,
                flags_default: None,
                flags_allowed: None,
                flags_implies: None,
                flags_conflicts: None,
                flags_descriptions: None,
                flags_required_one_of: None,
                flags_required_at_most_one: None,
                flags_required_any_of: None,
                subpackages: None,
                profile: None,
                build: None,
                has_build_table: false,
            },
        }
    }

    #[test]
    fn local_interbuild_root_detects_xbps_template_in_recipe_tree() {
        let tempdir = TempDir::new().expect("tempdir");
        let recipe_dir = tempdir.path().join("demo");
        fs::create_dir_all(&recipe_dir).expect("recipe dir");
        fs::write(recipe_dir.join("template"), "pkgname=demo\n").expect("template");
        fs::write(recipe_dir.join("pkg.lua"), "pkg = {}\n").expect("pkg.lua");

        let mut recipe = recipe_with_template(&recipe_dir, "xbps_template");
        recipe.package.source.fields.insert(
            "snapshot_rev".to_owned(),
            ScalarValue::String("abc123".to_owned()),
        );
        assert_eq!(local_interbuild_root(&recipe), Some(recipe_dir.clone()));
    }

    #[test]
    fn local_interbuild_root_requires_snapshot_provenance() {
        let tempdir = TempDir::new().expect("tempdir");
        let recipe_dir = tempdir.path().join("demo");
        fs::create_dir_all(&recipe_dir).expect("recipe dir");
        fs::write(recipe_dir.join("template"), "pkgname=demo\n").expect("template");
        fs::write(recipe_dir.join("pkg.lua"), "pkg = {}\n").expect("pkg.lua");

        let recipe = recipe_with_template(&recipe_dir, "xbps_template");

        assert_eq!(local_interbuild_root(&recipe), None);
    }

    #[test]
    fn snapshot_rev_reads_provenance_field() {
        let tempdir = TempDir::new().expect("tempdir");
        let recipe_dir = tempdir.path().join("demo");
        fs::create_dir_all(&recipe_dir).expect("recipe dir");

        let mut recipe = recipe_with_template(&recipe_dir, "xbps_template");
        recipe.package.source.fields.insert(
            "snapshot_rev".to_owned(),
            ScalarValue::String("abc123".to_owned()),
        );

        assert_eq!(snapshot_rev(&recipe), Some("abc123".to_owned()));
    }
}
