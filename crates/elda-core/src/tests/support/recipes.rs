use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

use super::repos::{make_executable, make_git_repo, run_git};

pub(in crate::tests) fn create_git_make_conffile_repo(
    root: &Path,
    name: &str,
    binary_output: &str,
    conffile_contents: &str,
) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join(name),
        format!("#!/bin/sh\necho '{binary_output}'\n"),
    )
    .expect("script binary should be written");
    make_executable(&repo_dir.join(name));
    fs::write(repo_dir.join(format!("{name}.conf")), conffile_contents)
        .expect("conffile should be written");
    fs::write(
        repo_dir.join("Makefile"),
        format!(
            "all:\n\ninstall:\n\tmkdir -p $(DESTDIR)$(PREFIX)/bin\n\tmkdir -p $(DESTDIR)/etc\n\tcp {name} $(DESTDIR)$(PREFIX)/bin/{name}\n\tcp {name}.conf $(DESTDIR)/etc/{name}.conf\n\tchmod 755 $(DESTDIR)$(PREFIX)/bin/{name}\n\tchmod 644 $(DESTDIR)/etc/{name}.conf\n"
        ),
    )
    .expect("makefile should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn update_git_make_conffile_repo(
    repo_dir: &Path,
    binary_output: &str,
    conffile_contents: &str,
) {
    let name = repo_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .expect("repo directory should have a package name");
    fs::write(
        repo_dir.join(&name),
        format!("#!/bin/sh\necho '{binary_output}'\n"),
    )
    .expect("script binary should be updated");
    make_executable(&repo_dir.join(&name));
    fs::write(repo_dir.join(format!("{name}.conf")), conffile_contents)
        .expect("conffile should be updated");
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "update"]);
}

pub(in crate::tests) fn write_dual_lane_recipe(
    root: &Path,
    repo_dir: &Path,
    binary_source: &Path,
    name: &str,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n\n  source = {{\n    lanes = {{\n      source = {{\n        kind = \"git\",\n        url = \"file://{repo}\",\n        branch = \"main\",\n      }},\n      binary = {{\n        kind = \"url_archive\",\n        url = \"file://{binary}\",\n        sha256 = \"{sha256}\",\n        rename = \"{name}\",\n      }},\n    }},\n  }},\n\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
            name = name,
            repo = repo_dir.display(),
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
        ),
    )
    .expect("dual-lane pkg.lua should be written");
}

pub(in crate::tests) fn write_local_binary_recipe(
    root: &Path,
    name: &str,
    binary_source: &Path,
    depends: &[&str],
) {
    write_local_binary_recipe_with_version(root, name, binary_source, depends, "0.1.0");
}

pub(in crate::tests) fn write_local_binary_recipe_with_version(
    root: &Path,
    name: &str,
    binary_source: &Path,
    depends: &[&str],
    version: &str,
) {
    write_local_binary_recipe_with_lua_fields(
        root,
        name,
        binary_source,
        version,
        &string_array_lua(depends),
        "{}",
        "{}",
    );
}

pub(in crate::tests) fn write_local_binary_recipe_with_recommends(
    root: &Path,
    name: &str,
    binary_source: &Path,
    depends: &[&str],
    recommends: &[&str],
) {
    write_local_binary_recipe_with_lua_fields(
        root,
        name,
        binary_source,
        "0.1.0",
        &string_array_lua(depends),
        &string_array_lua(recommends),
        "{}",
    );
}

pub(in crate::tests) fn write_local_binary_recipe_with_provides(
    root: &Path,
    name: &str,
    binary_source: &Path,
    depends_lua: &str,
    provides: &[&str],
) {
    write_local_binary_recipe_with_lua_fields(
        root,
        name,
        binary_source,
        "0.1.0",
        depends_lua,
        "{}",
        &string_array_lua(provides),
    );
}

pub(in crate::tests) struct PolicyFieldLua<'a> {
    pub(in crate::tests) depends: &'a str,
    pub(in crate::tests) recommends: &'a str,
    pub(in crate::tests) provides: &'a str,
    pub(in crate::tests) conflicts: &'a str,
}

impl Default for PolicyFieldLua<'_> {
    fn default() -> Self {
        Self {
            depends: "{}",
            recommends: "{}",
            provides: "{}",
            conflicts: "{}",
        }
    }
}

pub(in crate::tests) fn write_local_binary_recipe_with_lua_fields(
    root: &Path,
    name: &str,
    binary_source: &Path,
    version: &str,
    depends_lua: &str,
    recommends_lua: &str,
    provides_lua: &str,
) {
    write_local_binary_recipe_with_policy_fields(
        root,
        name,
        binary_source,
        version,
        PolicyFieldLua {
            depends: depends_lua,
            recommends: recommends_lua,
            provides: provides_lua,
            ..Default::default()
        },
    );
}

pub(in crate::tests) fn write_local_binary_recipe_with_policy_fields(
    root: &Path,
    name: &str,
    binary_source: &Path,
    version: &str,
    policy: PolicyFieldLua<'_>,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"{name}\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {recommends},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {provides},\n  conflicts = {conflicts},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
            name = name,
            version = version,
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
            depends = policy.depends,
            recommends = policy.recommends,
            provides = policy.provides,
            conflicts = policy.conflicts,
        ),
    )
    .expect("binary recipe should be written");
}

pub(in crate::tests) fn write_local_profile_recipe(
    root: &Path,
    name: &str,
    repo_dir: &Path,
    depends: &[&str],
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"profile\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{repo}\",\n    branch = \"main\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n",
            name = name,
            repo = repo_dir.display(),
            depends = string_array_lua(depends),
        ),
    )
    .expect("profile recipe should be written");
}

pub(in crate::tests) fn string_array_lua(values: &[&str]) -> String {
    if values.is_empty() {
        return "{}".to_owned();
    }

    format!(
        "{{ {} }}",
        values
            .iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(in crate::tests) fn write_local_git_conffile_recipe(
    root: &Path,
    name: &str,
    repo_dir: &Path,
    version: &str,
) {
    let recipes_dir = root.join("etc/elda/recipes").join(name);
    fs::create_dir_all(&recipes_dir).expect("recipe dir should exist");
    fs::write(
        recipes_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{repo}\",\n    branch = \"main\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{ \"/etc/{name}.conf\" }},\n}}\n",
            repo = repo_dir.display(),
        ),
    )
    .expect("git conffile recipe should be written");
}

pub(in crate::tests) fn write_remote_index(
    root: &Path,
    name: &str,
    binary_source: &Path,
) -> PathBuf {
    let index_path = root.join("remote-index.toml");
    write_remote_index_with_version(&index_path, name, binary_source, "1.2.3");

    index_path
}

pub(in crate::tests) fn write_remote_index_with_version(
    index_path: &Path,
    name: &str,
    binary_source: &Path,
    version: &str,
) {
    write_remote_index_with_lua_fields(index_path, name, binary_source, version, "{}", "{}");
}

pub(in crate::tests) fn write_remote_index_with_lua_fields(
    index_path: &Path,
    name: &str,
    binary_source: &Path,
    version: &str,
    depends_lua: &str,
    recommends_lua: &str,
) {
    fs::write(
        index_path,
        format!(
            "[[packages]]\npkgname = \"{name}\"\nsummary = \"Remote binary package\"\ndescription = \"Snapshot-backed install fixture\"\nasset_url = \"file://{binary}\"\nsha256 = \"{sha256}\"\npayload_sig = \"{payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"{name}\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {recommends},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            name = name,
            version = version,
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
            payload_sig = remote_payload_signature(binary_source),
            depends = depends_lua,
            recommends = recommends_lua,
        ),
    )
    .expect("remote index should be written");
    sign_remote_index(index_path);
}

pub(in crate::tests) fn write_remote_index_with_lua_fields_and_provides(
    index_path: &Path,
    name: &str,
    binary_source: &Path,
    version: &str,
    depends_lua: &str,
    recommends_lua: &str,
    provides_lua: &str,
) {
    fs::write(
        index_path,
        format!(
            "[[packages]]\npkgname = \"{name}\"\nsummary = \"Remote binary package\"\ndescription = \"Snapshot-backed install fixture\"\nasset_url = \"file://{binary}\"\nsha256 = \"{sha256}\"\npayload_sig = \"{payload_sig}\"\npkg_lua = '''\npkg = {{\n  name = \"{name}\",\n  epoch = 0,\n  version = \"{version}\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"url_archive\",\n    url = \"file://{binary}\",\n    sha256 = \"{sha256}\",\n    rename = \"{name}\",\n  }},\n  depends = {depends},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {recommends},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {provides},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n}}\n'''\n",
            name = name,
            version = version,
            binary = binary_source.display(),
            sha256 = sha256_file(binary_source),
            payload_sig = remote_payload_signature(binary_source),
            depends = depends_lua,
            recommends = recommends_lua,
            provides = provides_lua,
        ),
    )
    .expect("remote index should be written");
    sign_remote_index(index_path);
}

pub(in crate::tests) fn sha256_file(path: &Path) -> String {
    let mut file = fs::File::open(path).expect("sha256 input should exist");
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer).expect("sha256 read should succeed");
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    format!("{:x}", hasher.finalize())
}

pub(in crate::tests) fn sign_remote_index(index_path: &Path) {
    let content = fs::read(index_path).expect("remote index should exist before signing");
    let signing_key = remote_signing_key();
    let signature = signing_key.sign(&content);
    let signature_path = index_path.with_extension(format!(
        "{}.sig",
        index_path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
    ));

    fs::write(
        signature_path,
        format!(
            "key_id = \"fixture-remote\"\npublic_key = \"{}\"\nsignature = \"{}\"\n",
            STANDARD.encode(signing_key.verifying_key().as_bytes()),
            STANDARD.encode(signature.to_bytes()),
        ),
    )
    .expect("remote signature should be written");
}

pub(in crate::tests) fn remote_payload_signature(path: &Path) -> String {
    let payload = fs::read(path).expect("payload should exist before signing");
    let signing_key = remote_signing_key();
    let signature = signing_key.sign(&payload);

    STANDARD.encode(signature.to_bytes())
}

pub(in crate::tests) fn fixture_remote_key_fingerprint() -> String {
    let mut hasher = Sha256::new();
    hasher.update(remote_signing_key().verifying_key().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(in crate::tests) fn fixture_remote_release_trust_public_key() -> String {
    STANDARD.encode(remote_signing_key().verifying_key().as_bytes())
}

fn remote_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}
