use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{make_git_repo, remote_payload_signature, sha256_file, sign_remote_index};

pub(in crate::tests) fn create_package_definition_repo(
    root: &Path,
    package_name: &str,
    pkg_lua: &str,
    extra_files: &[(&str, &str)],
) -> PathBuf {
    let repo_dir = root.join(format!("{package_name}-pkgs"));
    let package_dir = repo_dir.join("packages").join(package_name);
    fs::create_dir_all(&package_dir).expect("package definition dir should exist");
    fs::write(package_dir.join("pkg.lua"), pkg_lua).expect("pkg.lua should be written");
    for (relative_path, content) in extra_files {
        let path = package_dir.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("extra file parent should exist");
        }
        fs::write(path, content).expect("extra file should be written");
    }

    make_git_repo(&repo_dir);
    repo_dir
}

pub(in crate::tests) fn git_head_commit(repo_dir: &Path) -> String {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse should launch");
    assert!(output.status.success(), "git rev-parse should succeed");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub(in crate::tests) fn write_remote_recipe_index(
    index_path: &Path,
    package_name: &str,
    pkg_lua: &str,
    repo_commit: &str,
    binary_source: Option<&Path>,
) {
    let binary_fields = binary_source
        .map(|binary| {
            format!(
                "asset_url = \"file://{binary}\"\nsha256 = \"{sha256}\"\npayload_sig = \"{payload_sig}\"\n",
                binary = binary.display(),
                sha256 = sha256_file(binary),
                payload_sig = remote_payload_signature(binary),
            )
        })
        .unwrap_or_default();

    fs::write(
        index_path,
        format!(
            "[[packages]]\npkgname = \"{package_name}\"\nsummary = \"Remote source package\"\ndescription = \"Snapshot-backed source recipe fixture\"\nrepo_commit = \"{repo_commit}\"\n{binary_fields}pkg_lua = '''\n{pkg_lua}\n'''\n",
        ),
    )
    .expect("remote recipe index should be written");
    sign_remote_index(index_path);
}
