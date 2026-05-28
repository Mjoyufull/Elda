use std::process::Command;

use crate::config::SubmissionMode;
use crate::error::CoreError;

use super::workspace::{CiWorkspacePaths, git_head_commit, git_remote_url};

#[derive(Debug, Clone)]
pub(crate) struct SubmissionRemotePush {
    pub(crate) remote_name: String,
    pub(crate) remote_url: String,
    pub(crate) pushed_ref: String,
    pub(crate) pushed_commit: Option<String>,
}

pub(crate) fn prepare_submission_checkout(
    workspace: &CiWorkspacePaths,
    branch_name: &str,
    mode: SubmissionMode,
    base_branch: &str,
) -> Result<(), CoreError> {
    workspace.ensure_exists()?;
    ensure_local_branch(workspace, base_branch, "main")?;

    match mode {
        SubmissionMode::Pr => {
            run_git(
                &workspace.packages_repo_dir,
                &["checkout", "-B", branch_name, base_branch],
            )?;
        }
        SubmissionMode::Push => {
            run_git(&workspace.packages_repo_dir, &["checkout", base_branch])?;
        }
    }

    Ok(())
}

pub(crate) fn push_submission_remote(
    workspace: &CiWorkspacePaths,
    remote_name: &str,
    branch_name: &str,
    mode: SubmissionMode,
    base_branch: &str,
) -> Result<Option<SubmissionRemotePush>, CoreError> {
    let Some(remote_url) = git_remote_url(&workspace.packages_repo_dir, remote_name)? else {
        return Ok(None);
    };

    let pushed_ref = match mode {
        SubmissionMode::Pr => {
            let destination = format!("HEAD:refs/heads/{branch_name}");
            if run_git(
                &workspace.packages_repo_dir,
                &["push", "--set-upstream", remote_name, &destination],
            )
            .is_err()
            {
                run_git(
                    &workspace.packages_repo_dir,
                    &[
                        "push",
                        "--force-with-lease",
                        "--set-upstream",
                        remote_name,
                        &destination,
                    ],
                )?;
            }
            format!("refs/heads/{branch_name}")
        }
        SubmissionMode::Push => {
            let destination = format!("HEAD:refs/heads/{base_branch}");
            run_git(
                &workspace.packages_repo_dir,
                &["push", remote_name, &destination],
            )?;
            format!("refs/heads/{base_branch}")
        }
    };

    Ok(Some(SubmissionRemotePush {
        remote_name: remote_name.to_owned(),
        remote_url,
        pushed_ref,
        pushed_commit: git_head_commit(&workspace.packages_repo_dir)?,
    }))
}

fn ensure_local_branch(
    workspace: &CiWorkspacePaths,
    branch_name: &str,
    fallback_branch: &str,
) -> Result<(), CoreError> {
    if run_git(&workspace.packages_repo_dir, &["checkout", branch_name]).is_ok() {
        return Ok(());
    }

    run_git(
        &workspace.packages_repo_dir,
        &["checkout", "-B", branch_name, fallback_branch],
    )
}

fn run_git(repo_dir: &std::path::Path, args: &[&str]) -> Result<(), CoreError> {
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
