use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use goblin::Object;
use serde::{Deserialize, Serialize};

use crate::{BuildError, ManifestEntryKind, PackageManifest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ObjectMetadata {
    pub shlib_provides: Vec<SharedLibraryProvide>,
    pub shlib_requires: Vec<SharedLibraryRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SharedLibraryProvide {
    pub path: String,
    pub soname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SharedLibraryRequirement {
    pub path: String,
    pub library: String,
}

pub fn analyze_stage_objects(
    stage_root: &Path,
    manifest: &PackageManifest,
) -> Result<ObjectMetadata, BuildError> {
    let mut shlib_provides = BTreeSet::new();
    let mut shlib_requires = BTreeSet::new();

    for entry in &manifest.entries {
        if entry.kind != ManifestEntryKind::RegularFile {
            continue;
        }

        let staged_path = stage_path(stage_root, &entry.path)?;
        let bytes = fs::read(&staged_path)?;
        let Ok(Object::Elf(elf)) = Object::parse(&bytes) else {
            continue;
        };

        if let Some(soname) = elf.soname
            && !soname.trim().is_empty()
        {
            shlib_provides.insert(SharedLibraryProvide {
                path: entry.path.clone(),
                soname: soname.to_owned(),
            });
        }

        for library in elf.libraries {
            if library.trim().is_empty() {
                continue;
            }
            shlib_requires.insert(SharedLibraryRequirement {
                path: entry.path.clone(),
                library: library.to_owned(),
            });
        }
    }

    Ok(ObjectMetadata {
        shlib_provides: shlib_provides.into_iter().collect(),
        shlib_requires: shlib_requires.into_iter().collect(),
    })
}

fn stage_path(stage_root: &Path, manifest_path: &str) -> Result<std::path::PathBuf, BuildError> {
    let relative = manifest_path.trim_start_matches('/');
    if relative.is_empty() {
        return Err(BuildError::Invalid(
            "manifest entry path cannot be empty during object analysis".to_owned(),
        ));
    }

    Ok(stage_root.join(relative))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::process::Command;

    use tempfile::TempDir;

    use super::analyze_stage_objects;
    use crate::manifest::collect_manifest;

    #[test]
    fn analyze_stage_objects_collects_shlib_requires_and_provides() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        let stage_root = tempdir.path().join("stage");
        fs::create_dir_all(stage_root.join("usr/bin")).expect("bin dir should exist");
        fs::create_dir_all(stage_root.join("usr/lib")).expect("lib dir should exist");

        compile_rust_binary(
            tempdir.path().join("main.rs"),
            stage_root.join("usr/bin/demo-bin"),
            "fn main() { println!(\"demo\"); }\n",
            &[],
        );
        compile_rust_binary(
            tempdir.path().join("lib.rs"),
            stage_root.join("usr/lib/libdemo.so.1.0"),
            "#[unsafe(no_mangle)] pub extern \"C\" fn demo() {}\n",
            &[
                "--crate-type",
                "cdylib",
                "-C",
                "link-arg=-Wl,-soname,libdemo.so.1",
            ],
        );

        let manifest = collect_manifest(&stage_root).expect("manifest should be collected");
        let metadata = analyze_stage_objects(&stage_root, &manifest)
            .expect("object metadata should be collected");

        assert!(
            metadata.shlib_requires.iter().any(|entry| {
                entry.path == "/usr/bin/demo-bin" && entry.library.contains(".so")
            })
        );
        assert!(metadata.shlib_provides.iter().any(|entry| {
            entry.path == "/usr/lib/libdemo.so.1.0" && entry.soname == "libdemo.so.1"
        }));
    }

    fn compile_rust_binary(
        source_path: std::path::PathBuf,
        output_path: std::path::PathBuf,
        source: &str,
        extra_args: &[&str],
    ) {
        fs::write(&source_path, source).expect("rust source should be written");
        let status = Command::new("rustc")
            .arg(&source_path)
            .args(extra_args)
            .arg("-o")
            .arg(&output_path)
            .status()
            .expect("rustc should launch");
        assert!(status.success(), "rustc should succeed for test fixture");
    }
}
