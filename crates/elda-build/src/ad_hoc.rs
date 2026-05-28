use crate::{BuildError, BuildRequest};

pub(super) fn resolved_pkgver(
    request: &BuildRequest<'_>,
    repo_commit: Option<&str>,
    repo_commit_unix: Option<u64>,
) -> Result<String, BuildError> {
    if !request.ad_hoc_git {
        return Ok(request.recipe.package.version.clone());
    }

    let commit = repo_commit.ok_or_else(|| {
        BuildError::Invalid(
            "ad hoc git install could not determine the resolved repository commit".to_owned(),
        )
    })?;
    let commit_unix = repo_commit_unix.ok_or_else(|| {
        BuildError::Invalid(
            "ad hoc git install could not determine the resolved commit timestamp".to_owned(),
        )
    })?;

    Ok(format!("0.git.{commit_unix}.{}", short_commit(commit)))
}

pub(super) fn resolved_pkgrel(request: &BuildRequest<'_>) -> u64 {
    if request.ad_hoc_git {
        1
    } else {
        request.recipe.package.rel
    }
}

fn short_commit(commit: &str) -> &str {
    let short_len = 12;
    if commit.len() <= short_len {
        commit
    } else {
        &commit[..short_len]
    }
}
