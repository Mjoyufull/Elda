use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct CiWorkspacePaths {
    pub(crate) root: PathBuf,
    pub(crate) packages_repo_dir: PathBuf,
    pub(crate) packages_dir: PathBuf,
    pub(crate) submissions_dir: PathBuf,
    pub(crate) batches_dir: PathBuf,
    pub(crate) logs_dir: PathBuf,
    pub(crate) locks_dir: PathBuf,
    pub(crate) published_dir: PathBuf,
    pub(crate) artifacts_dir: PathBuf,
    pub(crate) index_path: PathBuf,
    pub(crate) signature_path: PathBuf,
    pub(crate) signing_key_path: PathBuf,
}

impl CiWorkspacePaths {
    pub(crate) fn new(layout: &elda_db::StateLayout) -> Self {
        let root = layout.data_dir.join("ci");
        let published_dir = root.join("published");

        Self {
            root: root.clone(),
            packages_repo_dir: root.join("pkgs"),
            packages_dir: root.join("pkgs/packages"),
            submissions_dir: root.join("submissions"),
            batches_dir: root.join("batches"),
            logs_dir: root.join("logs"),
            locks_dir: root.join("locks"),
            artifacts_dir: published_dir.join("artifacts"),
            index_path: published_dir.join("index-v1.json"),
            signature_path: published_dir.join("index-v1.json.sig"),
            signing_key_path: root.join("signing-key.bin"),
            published_dir,
        }
    }

    pub(crate) fn ensure_exists(&self) -> Result<(), CoreError> {
        for directory in [
            &self.root,
            &self.packages_dir,
            &self.submissions_dir,
            &self.batches_dir,
            &self.logs_dir,
            &self.locks_dir,
            &self.published_dir,
            &self.artifacts_dir,
        ] {
            fs::create_dir_all(directory)?;
        }

        ensure_git_repo(&self.packages_repo_dir)?;
        ensure_signing_key(&self.signing_key_path)?;

        Ok(())
    }
}

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), CoreError> {
    let parent = path.parent().ok_or_else(|| {
        CoreError::Operator(format!("path `{}` has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, CoreError> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(CoreError::from)
}

pub(crate) fn list_json_records<T: DeserializeOwned>(
    directory: &Path,
) -> Result<Vec<T>, CoreError> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        records.push(read_json(&path)?);
    }

    Ok(records)
}

pub(crate) fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub(crate) fn signing_key(path: &Path) -> Result<SigningKey, CoreError> {
    ensure_signing_key(path)?;
    let bytes = fs::read(path)?;
    let key_bytes: [u8; 32] = bytes.try_into().map_err(|_| {
        CoreError::Operator(format!(
            "signing key `{}` does not contain 32 bytes",
            path.display()
        ))
    })?;
    Ok(SigningKey::from_bytes(&key_bytes))
}

pub(crate) fn fingerprint_for_key(signing_key: &SigningKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(signing_key.verifying_key().as_bytes());
    hex_digest(hasher.finalize())
}

pub(crate) fn sign_bytes(signing_key: &SigningKey, content: &[u8]) -> String {
    let signature = signing_key.sign(content);
    STANDARD.encode(signature.to_bytes())
}

pub(crate) fn write_signature_envelope(
    signing_key: &SigningKey,
    path: &Path,
    signature: &str,
) -> Result<(), CoreError> {
    let envelope = format!(
        "key_id = \"local-ci\"\npublic_key = \"{}\"\nsignature = \"{signature}\"\n",
        STANDARD.encode(signing_key.verifying_key().as_bytes()),
    );
    fs::write(path, envelope)?;
    Ok(())
}

pub(crate) fn sync_recipe_into_packages_repo(
    workspace: &CiWorkspacePaths,
    recipes_dir: &Path,
    package_name: &str,
) -> Result<PathBuf, CoreError> {
    workspace.ensure_exists()?;
    let source = recipes_dir.join(package_name);
    if !source.join("pkg.lua").is_file() {
        return Err(CoreError::Operator(format!(
            "local recipe `{package_name}` does not exist under {}",
            recipes_dir.display()
        )));
    }

    let destination = workspace.packages_dir.join(package_name);
    remove_path_if_exists(&destination)?;
    copy_dir_recursive(&source, &destination)?;

    Ok(destination)
}

pub(crate) fn commit_packages_repo(
    workspace: &CiWorkspacePaths,
    message: &str,
) -> Result<Option<String>, CoreError> {
    workspace.ensure_exists()?;
    if !git_repo_dirty(&workspace.packages_repo_dir)? {
        return git_head_commit(&workspace.packages_repo_dir);
    }

    run_git(&workspace.packages_repo_dir, &["add", "."])?;
    run_git(&workspace.packages_repo_dir, &["commit", "-m", message])?;

    git_head_commit(&workspace.packages_repo_dir)
}

pub(crate) fn git_head_commit(repo_dir: &Path) -> Result<Option<String>, CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let commit = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if commit.is_empty() {
        Ok(None)
    } else {
        Ok(Some(commit))
    }
}

pub(crate) fn git_remote_url(
    repo_dir: &Path,
    remote_name: &str,
) -> Result<Option<String>, CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["config", "--get", &format!("remote.{remote_name}.url")])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if remote_url.is_empty() {
        Ok(None)
    } else {
        Ok(Some(remote_url))
    }
}

pub(crate) fn forge_pr_url(
    origin_url: &str,
    branch_name: &str,
    base_branch: &str,
) -> Option<String> {
    let repository_url = normalize_repository_url(origin_url)?;
    let encoded_branch = percent_encode(branch_name);
    let encoded_base = percent_encode(base_branch);

    if repository_url.contains("gitlab") {
        return Some(format!(
            "{repository_url}/-/merge_requests/new?merge_request[source_branch]={encoded_branch}&merge_request[target_branch]={encoded_base}"
        ));
    }

    Some(format!(
        "{repository_url}/compare/{encoded_base}...{encoded_branch}?expand=1"
    ))
}

pub(crate) fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), CoreError> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

fn normalize_repository_url(origin_url: &str) -> Option<String> {
    let trimmed = origin_url
        .trim()
        .strip_suffix(".git")
        .unwrap_or(origin_url.trim());
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return Some(trimmed.to_owned());
    }

    let ssh = trimmed.strip_prefix("git@")?;
    let (host, path) = ssh.split_once(':')?;
    Some(format!("https://{host}/{path}"))
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

pub(crate) fn remove_path_if_exists(path: &Path) -> Result<(), CoreError> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }

    Ok(())
}

fn ensure_git_repo(repo_dir: &Path) -> Result<(), CoreError> {
    if repo_dir.join(".git").is_dir() {
        return Ok(());
    }

    fs::create_dir_all(repo_dir)?;
    run_git(repo_dir, &["init", "-b", "main"])?;
    run_git(repo_dir, &["config", "user.name", "Elda CI"])?;
    run_git(
        repo_dir,
        &["config", "user.email", "elda-ci@example.invalid"],
    )?;
    fs::write(
        repo_dir.join(".gitignore"),
        "index-v1.json\nindex-v1.json.sig\n",
    )?;
    run_git(repo_dir, &["add", ".gitignore"])?;
    run_git(repo_dir, &["commit", "-m", "initialize local ci workspace"])?;

    Ok(())
}

fn run_git(repo_dir: &Path, args: &[&str]) -> Result<(), CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(CoreError::Operator(format!(
        "git {} failed in `{}`: {}",
        args.join(" "),
        repo_dir.display(),
        String::from_utf8_lossy(&output.stderr).trim(),
    )))
}

fn git_repo_dirty(repo_dir: &Path) -> Result<bool, CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Err(CoreError::Operator(format!(
            "git status failed in `{}`: {}",
            repo_dir.display(),
            String::from_utf8_lossy(&output.stderr).trim(),
        )));
    }

    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn ensure_signing_key(path: &Path) -> Result<(), CoreError> {
    if path.is_file() {
        return Ok(());
    }

    let parent = path.parent().ok_or_else(|| {
        CoreError::Operator(format!("path `{}` has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let mut hasher = Sha256::new();
    hasher.update(path.display().to_string());
    hasher.update(current_unix_timestamp().to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let seed = hasher.finalize();
    fs::write(path, &seed[..32])?;

    Ok(())
}

fn hex_digest<T: AsRef<[u8]>>(bytes: T) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
