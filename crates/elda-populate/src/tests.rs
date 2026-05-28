use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use elda_core::{CommandRequest, OutputMode, run_from_root};
use elda_repo::{CacheDocument, RemoteDocument, TrustMode, save_cache, save_remote, sync_remotes};

use crate::cli::{MirrorRemoteArgs, PushLocalArgs};
use crate::operations::{mirror_remote, push_local};

mod channel;

#[test]
fn push_local_installed_payloads_to_file_cache() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let repo_dir = create_git_make_repo(tempdir.path(), "local-tool", "hello from cache");
    write_local_git_recipe(tempdir.path(), "local-tool", &repo_dir);

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["local-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    let layout = elda_db::StateLayout::new(tempdir.path(), "/opt/elda");
    save_cache(
        &layout.caches_dir,
        CacheDocument {
            name: "lan".to_owned(),
            base_url: format!("file://{}", tempdir.path().join("lan-cache").display()),
            priority: 10,
            enabled: true,
        },
    )
    .expect("cache should be saved");

    let report = push_local(
        layout.clone(),
        PushLocalArgs {
            cache: "lan".to_owned(),
            installed: true,
            package: Vec::new(),
            manifest_out: None,
            dry_run: false,
        },
    )
    .expect("push-local should succeed");

    assert_eq!(report.mode, "push-local");
    assert_eq!(report.mirrored, 1);
    let cache_entries = fs::read_dir(tempdir.path().join("lan-cache"))
        .expect("cache directory should exist")
        .count();
    assert_eq!(cache_entries, 1);
}

#[test]
fn mirror_remote_copies_binary_payloads_from_synced_snapshot() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let layout = elda_db::StateLayout::new(tempdir.path(), "/opt/elda");
    layout.ensure_exists().expect("layout should exist");

    let payload_path = tempdir.path().join("remote-tool.bin");
    fs::write(&payload_path, b"remote payload").expect("payload should exist");
    let payload_sha256 = sha256_file(&payload_path);
    let index_path = tempdir.path().join("remote-index.toml");
    fs::write(
        &index_path,
        format!(
            "[[packages]]\npkgname = \"remote-tool\"\nsummary = \"Remote payload\"\ndescription = \"Binary record\"\nasset_url = \"file://{}\"\nsha256 = \"{}\"\npayload_sig = \"sig\"\npkg_lua = '''\npkg = {{\n  name = \"remote-tool\",\n  epoch = 0,\n  version = \"1.0.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{}\",\n    sha256 = \"{}\",\n    rename = \"remote-tool\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}}\n}}\n'''\n",
            payload_path.display(),
            payload_sha256,
            payload_path.display(),
            payload_sha256,
        ),
    )
    .expect("index should be written");
    save_remote(
        &layout.remotes_dir,
        RemoteDocument {
            name: "main".to_owned(),
            index_url: format!("file://{}", index_path.display()),
            channel: "stable".to_owned(),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Insecure,
            trusted_keys: Vec::new(),
            allow_stale: false,
            exclude: Vec::new(),
            priority: 10,
        },
    )
    .expect("remote should be saved");
    save_cache(
        &layout.caches_dir,
        CacheDocument {
            name: "lan".to_owned(),
            base_url: format!("file://{}", tempdir.path().join("lan-cache").display()),
            priority: 10,
            enabled: true,
        },
    )
    .expect("cache should be saved");
    sync_remotes(
        &layout.remotes_dir,
        &layout.db_dir.join("repo-snapshot.json"),
        Default::default(),
    )
    .expect("sync should succeed");

    let manifest_path = tempdir.path().join("cache-seed.json");
    let report = mirror_remote(
        layout,
        MirrorRemoteArgs {
            cache: "lan".to_owned(),
            remote: "main".to_owned(),
            channel: None,
            package: Vec::new(),
            manifest_out: Some(manifest_path.clone()),
            dry_run: false,
        },
    )
    .expect("mirror-remote should succeed");

    assert_eq!(report.mode, "mirror-remote");
    assert_eq!(report.mirrored, 1);
    assert!(manifest_path.exists(), "manifest should be written");
    let mirrored = tempdir.path().join("lan-cache").join(payload_sha256);
    assert!(mirrored.exists(), "payload should be mirrored by digest");
}

fn write_prefix_config(root: &Path, prefix: &str) {
    let config_dir = root.join("etc/elda");
    fs::create_dir_all(&config_dir).expect("config dir should exist");
    fs::write(
        config_dir.join("config.toml"),
        format!("[defaults]\nprefix = \"{prefix}\"\nbuild_mode = \"host\"\n"),
    )
    .expect("config should be written");
}

fn create_git_make_repo(root: &Path, name: &str, output: &str) -> PathBuf {
    let repo_dir = root.join(format!("{name}-repo"));
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join(name),
        format!(
            "#!/bin/sh\nprintf '%s\\n' '{}'\n",
            output.replace('\'', "'\\''")
        ),
    )
    .expect("binary should be written");
    make_executable(&repo_dir.join(name));
    fs::write(
        repo_dir.join("Makefile"),
        format!(
            "all:\n\t@true\n\ninstall:\n\tmkdir -p $(DESTDIR)/usr/bin\n\tcp {name} $(DESTDIR)/usr/bin/{name}\n"
        ),
    )
    .expect("makefile should be written");
    git(&repo_dir, ["init", "-b", "main"]);
    git(&repo_dir, ["config", "user.email", "elda@example.invalid"]);
    git(&repo_dir, ["config", "user.name", "Elda Tests"]);
    git(&repo_dir, ["add", "."]);
    git(&repo_dir, ["commit", "-m", "initial"]);
    repo_dir
}

fn write_local_git_recipe(root: &Path, name: &str, repo_dir: &Path) {
    let recipe_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipe_dir).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{ kind = \"git\", url = \"file://{}\", branch = \"main\" }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}}\n}}\n",
            repo_dir.display()
        ),
    )
    .expect("recipe should be written");
    fs::write(
        recipe_dir.join("build.lua"),
        "return { system = \"make\" }\n",
    )
    .expect("build.lua should be written");
}

fn git<const N: usize>(repo: &Path, args: [&str; N]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should run");
    assert!(status.success(), "git command should succeed");
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .expect("metadata should exist")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("permissions should be updated");
    }
}

fn sha256_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(fs::read(path).expect("payload should be readable"));
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
