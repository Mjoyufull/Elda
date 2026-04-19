use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::{AppContext, RemoteRecipeSource};
use crate::error::CoreError;
use elda_recipe::{RecipeDocument, load_recipe};
use elda_repo::{RepoError, SyncedPackageRecord, load_remote};

impl AppContext {
    pub(crate) fn remote_recipe_source(
        &self,
        package: &SyncedPackageRecord,
    ) -> Result<RemoteRecipeSource, CoreError> {
        let remote = load_remote(&self.database.layout().remotes_dir, &package.remote_name)?
            .ok_or_else(|| {
                CoreError::Repo(RepoError::Parse(format!(
                    "remote `{}` is not registered locally",
                    package.remote_name
                )))
            })?;
        let packages_url = remote.packages_url.ok_or_else(|| {
            CoreError::Repo(RepoError::Parse(format!(
                "remote `{}` does not define `packages_url`; source builds from synced remotes require `rmt add --packages-url <git-url>` or a matching remote document update",
                package.remote_name
            )))
        })?;
        let repo_commit = package.repo_commit.clone().ok_or_else(|| {
            CoreError::Repo(RepoError::Parse(format!(
                "remote package `{}` is missing indexed `repo_commit`; source builds from synced remotes require a pinned package-definition commit",
                package.pkgname
            )))
        })?;

        Ok(RemoteRecipeSource {
            remote_name: package.remote_name.clone(),
            packages_url,
            package_name: package.pkgname.clone(),
            repo_commit,
            indexed_pkg_lua: package.pkg_lua.clone(),
        })
    }

    pub(crate) fn materialize_remote_recipe(
        &self,
        source: &RemoteRecipeSource,
        offline: bool,
    ) -> Result<RecipeDocument, CoreError> {
        let recipe_root = self.cached_remote_recipe_root(source);
        let pkg_lua_path = recipe_root.join("pkg.lua");
        if pkg_lua_path.is_file() {
            return self.load_cached_remote_recipe(source, &recipe_root);
        }

        if offline && !git_location_is_local(&source.packages_url) {
            return Err(CoreError::Operator(format!(
                "offline mode cannot fetch package-definition repo `{}` for remote `{}`",
                source.packages_url, source.remote_name
            )));
        }

        let temp_root = self.remote_recipe_temp_root(source);
        if temp_root.exists() {
            fs::remove_dir_all(&temp_root)?;
        }
        fs::create_dir_all(&temp_root)?;

        let checkout_dir = temp_root.join("repo");
        clone_git_repo(&source.packages_url, &checkout_dir)?;
        checkout_git_commit(&checkout_dir, &source.repo_commit)?;
        verify_git_head(&checkout_dir, &source.repo_commit)?;

        let package_dir = checkout_dir.join("packages").join(&source.package_name);
        if !package_dir.join("pkg.lua").is_file() {
            return Err(CoreError::Repo(RepoError::Parse(format!(
                "package-definition repo `{}` at `{}` does not contain `packages/{}/pkg.lua`",
                source.remote_name, source.repo_commit, source.package_name
            ))));
        }

        let staging_root = temp_root.join("package");
        copy_dir_recursive(&package_dir, &staging_root)?;
        let staged_pkg_lua = fs::read_to_string(staging_root.join("pkg.lua"))?;
        if normalize_pkg_lua_text(&staged_pkg_lua)
            != normalize_pkg_lua_text(&source.indexed_pkg_lua)
        {
            return Err(CoreError::Repo(RepoError::Parse(format!(
                "remote package `{}` indexed `pkg_lua` does not match package-definition repo contents at commit `{}`",
                source.package_name, source.repo_commit
            ))));
        }

        if let Some(parent) = recipe_root.parent() {
            fs::create_dir_all(parent)?;
        }
        if recipe_root.exists() {
            fs::remove_dir_all(&recipe_root)?;
        }
        fs::rename(&staging_root, &recipe_root)?;
        fs::remove_dir_all(&temp_root)?;

        self.load_cached_remote_recipe(source, &recipe_root)
    }

    fn load_cached_remote_recipe(
        &self,
        source: &RemoteRecipeSource,
        recipe_root: &Path,
    ) -> Result<RecipeDocument, CoreError> {
        let cached_pkg_lua = fs::read_to_string(recipe_root.join("pkg.lua"))?;
        if normalize_pkg_lua_text(&cached_pkg_lua)
            != normalize_pkg_lua_text(&source.indexed_pkg_lua)
        {
            return Err(CoreError::Repo(RepoError::Parse(format!(
                "cached package-definition tree for remote `{}` package `{}` no longer matches indexed `pkg_lua`; remove the cached tree and retry",
                source.remote_name, source.package_name
            ))));
        }

        load_recipe(
            recipe_root.parent().unwrap_or(recipe_root),
            &source.package_name,
        )
        .map_err(CoreError::from)
    }

    fn cached_remote_recipe_root(&self, source: &RemoteRecipeSource) -> PathBuf {
        self.database
            .layout()
            .cache_src_dir
            .join("remote-recipes")
            .join(&source.remote_name)
            .join(&source.repo_commit)
            .join(&source.package_name)
    }

    fn remote_recipe_temp_root(&self, source: &RemoteRecipeSource) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        self.database
            .layout()
            .tmp_dir
            .join("remote-recipes")
            .join(format!(
                "{}-{}-{}",
                source.remote_name, source.package_name, nonce
            ))
    }
}

fn git_location_is_local(location: &str) -> bool {
    if location.starts_with("file://") {
        return true;
    }

    Path::new(location).exists()
}

fn clone_git_repo(location: &str, target_dir: &Path) -> Result<(), CoreError> {
    let status = Command::new("git")
        .arg("clone")
        .arg(location)
        .arg(target_dir)
        .status()?;
    if status.success() {
        return Ok(());
    }

    Err(CoreError::Repo(RepoError::Parse(format!(
        "failed to clone package-definition repo `{location}`"
    ))))
}

fn checkout_git_commit(repo_dir: &Path, commit: &str) -> Result<(), CoreError> {
    let status = Command::new("git")
        .current_dir(repo_dir)
        .args(["checkout", commit])
        .status()?;
    if status.success() {
        return Ok(());
    }

    Err(CoreError::Repo(RepoError::Parse(format!(
        "failed to checkout package-definition commit `{commit}`"
    ))))
}

fn verify_git_head(repo_dir: &Path, expected_commit: &str) -> Result<(), CoreError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Err(CoreError::Repo(RepoError::Parse(
            "failed to read checked-out package-definition commit".to_owned(),
        )));
    }

    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if actual == expected_commit {
        return Ok(());
    }

    Err(CoreError::Repo(RepoError::Parse(format!(
        "package-definition repo resolved commit `{actual}` instead of expected `{expected_commit}`"
    ))))
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), CoreError> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if entry_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
            continue;
        }

        if entry_type.is_symlink() {
            copy_symlink(&source_path, &target_path)?;
            continue;
        }

        fs::copy(&source_path, &target_path)?;
    }

    Ok(())
}

fn normalize_pkg_lua_text(content: &str) -> &str {
    content.trim_end_matches(['\r', '\n'])
}

#[cfg(unix)]
fn copy_symlink(source: &Path, target: &Path) -> Result<(), CoreError> {
    use std::os::unix::fs::symlink;

    let link_target = fs::read_link(source)?;
    symlink(link_target, target)?;
    Ok(())
}

#[cfg(not(unix))]
fn copy_symlink(source: &Path, target: &Path) -> Result<(), CoreError> {
    let metadata = fs::metadata(source)?;
    if metadata.is_dir() {
        copy_dir_recursive(source, target)?;
    } else {
        fs::copy(source, target)?;
    }
    Ok(())
}
