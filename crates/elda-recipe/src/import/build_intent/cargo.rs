use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{BuildIntent, clean_bin_name, read_toml, sorted_unique};

pub(super) fn intent(source_dir: &Path) -> Option<BuildIntent> {
    let manifest = source_dir.join("Cargo.toml");
    if !manifest.is_file() {
        return None;
    }

    let mut bins = manifest_bins(source_dir, &manifest);
    if bins.is_empty() {
        bins = workspace_bins(source_dir, &manifest);
    }
    Some(BuildIntent {
        system: "cargo".to_owned(),
        bins: sorted_unique(bins),
    })
}

fn manifest_bins(source_dir: &Path, manifest: &Path) -> Vec<String> {
    let Ok(parsed) = read_toml::<CargoManifest>(manifest) else {
        return Vec::new();
    };
    let mut bins = parsed
        .bins
        .into_iter()
        .filter_map(|bin| clean_bin_name(bin.name.as_deref()))
        .collect::<Vec<_>>();
    if parsed.package.autobins != Some(false)
        && source_dir.join("src/main.rs").is_file()
        && let Some(name) = clean_bin_name(parsed.package.name.as_deref())
    {
        bins.push(name);
    }
    bins.extend(src_bin_names(source_dir));
    bins
}

fn workspace_bins(source_dir: &Path, manifest: &Path) -> Vec<String> {
    let Ok(parsed) = read_toml::<CargoWorkspaceManifest>(manifest) else {
        return Vec::new();
    };
    parsed
        .workspace
        .members
        .iter()
        .flat_map(|member| workspace_member_dirs(source_dir, member))
        .flat_map(|member| manifest_bins(&member, &member.join("Cargo.toml")))
        .collect()
}

fn src_bin_names(source_dir: &Path) -> Vec<String> {
    let bin_dir = source_dir.join("src/bin");
    let Ok(entries) = fs::read_dir(bin_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return clean_bin_name(entry.file_name().to_str());
            }
            if path.extension().is_some_and(|ext| ext == "rs") {
                return path
                    .file_stem()
                    .and_then(|stem| clean_bin_name(stem.to_str()));
            }
            None
        })
        .collect()
}

fn workspace_member_dirs(source_dir: &Path, member: &str) -> Vec<PathBuf> {
    if let Some(prefix) = member.strip_suffix("/*") {
        let base = source_dir.join(prefix);
        let Ok(entries) = fs::read_dir(base) else {
            return Vec::new();
        };
        return entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.join("Cargo.toml").is_file())
            .collect();
    }

    let path = source_dir.join(member);
    path.join("Cargo.toml")
        .is_file()
        .then_some(path)
        .into_iter()
        .collect()
}

#[derive(Default, Deserialize)]
struct CargoManifest {
    #[serde(default)]
    package: CargoPackage,
    #[serde(default, rename = "bin")]
    bins: Vec<CargoBin>,
}

#[derive(Default, Deserialize)]
struct CargoPackage {
    name: Option<String>,
    autobins: Option<bool>,
}

#[derive(Default, Deserialize)]
struct CargoBin {
    name: Option<String>,
}

#[derive(Default, Deserialize)]
struct CargoWorkspaceManifest {
    #[serde(default)]
    workspace: CargoWorkspace,
}

#[derive(Default, Deserialize)]
struct CargoWorkspace {
    #[serde(default)]
    members: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::intent;

    #[test]
    fn reads_root_and_src_bin_targets() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::create_dir_all(tempdir.path().join("src/bin")).expect("src/bin should exist");
        fs::write(
            tempdir.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n[[bin]]\nname = \"tool\"\n",
        )
        .expect("manifest should exist");
        fs::write(tempdir.path().join("src/main.rs"), "fn main() {}").expect("main should exist");
        fs::write(tempdir.path().join("src/bin/side.rs"), "fn main() {}")
            .expect("side bin should exist");

        let intent = intent(tempdir.path()).expect("cargo intent should parse");

        assert_eq!(intent.system, "cargo");
        assert_eq!(intent.bins, ["demo", "side", "tool"]);
    }
}
