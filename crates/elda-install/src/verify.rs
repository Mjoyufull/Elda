use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::InstallError;
use crate::fsops::map_manifest_path;
use elda_db::{Database, PackageFileRecord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerifyReport {
    pub packages: Vec<String>,
    pub checked_paths: usize,
    pub issues: Vec<VerifyIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerifyIssue {
    pub package: String,
    pub path: String,
    pub kind: VerifyIssueKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerifyIssueKind {
    MissingFile,
    ContentMismatch,
    MetadataDrift,
    UnmanagedPathCollision,
}

pub fn verify_packages(
    database: &Database,
    targets: &[String],
) -> Result<VerifyReport, InstallError> {
    database.bootstrap()?;

    let packages = if targets.is_empty() {
        database
            .list_installed_packages()?
            .into_iter()
            .map(|package| package.pkgname)
            .collect::<Vec<_>>()
    } else {
        targets.to_vec()
    };

    let mut checked_paths = 0;
    let mut issues = Vec::new();

    for package in &packages {
        let records = database.package_files(package)?;
        if records.is_empty() && database.installed_package(package)?.is_none() {
            return Err(InstallError::NotInstalled(package.clone()));
        }

        for record in records {
            checked_paths += 1;
            if let Some(issue) = verify_record(database, &record)? {
                issues.push(issue);
            }
        }
    }

    Ok(VerifyReport {
        packages,
        checked_paths,
        issues,
    })
}

fn verify_record(
    database: &Database,
    record: &PackageFileRecord,
) -> Result<Option<VerifyIssue>, InstallError> {
    let target_path = map_manifest_path(database.layout(), &record.path)?;
    let Ok(metadata) = fs::symlink_metadata(&target_path) else {
        return Ok(Some(VerifyIssue {
            package: record.pkgname.clone(),
            path: record.path.clone(),
            kind: VerifyIssueKind::MissingFile,
            detail: "managed path is missing from the filesystem".to_owned(),
        }));
    };

    match record.path_kind.as_str() {
        "directory" => {
            if !metadata.is_dir() {
                return Ok(Some(collision_issue(record, "expected a directory")));
            }
            if metadata.permissions().mode() != record.mode {
                return Ok(Some(metadata_issue(
                    record,
                    format!(
                        "directory mode drift: expected {:o}, found {:o}",
                        record.mode,
                        metadata.permissions().mode()
                    ),
                )));
            }
        }
        "file" => {
            if !metadata.is_file() {
                return Ok(Some(collision_issue(record, "expected a regular file")));
            }
            if let Some(expected) = &record.sha256 {
                let actual = sha256_file(&target_path)?;
                if actual != *expected {
                    return Ok(Some(VerifyIssue {
                        package: record.pkgname.clone(),
                        path: record.path.clone(),
                        kind: VerifyIssueKind::ContentMismatch,
                        detail: format!(
                            "content hash mismatch: expected {expected}, found {actual}"
                        ),
                    }));
                }
            }
            if metadata.permissions().mode() != record.mode {
                return Ok(Some(metadata_issue(
                    record,
                    format!(
                        "file mode drift: expected {:o}, found {:o}",
                        record.mode,
                        metadata.permissions().mode()
                    ),
                )));
            }
        }
        "symlink" => {
            if !metadata.file_type().is_symlink() {
                return Ok(Some(collision_issue(record, "expected a symlink")));
            }
            let actual = fs::read_link(&target_path)?.display().to_string();
            let expected = record.link_target.as_deref().unwrap_or_default();
            if actual != expected {
                return Ok(Some(metadata_issue(
                    record,
                    format!("symlink target drift: expected `{expected}`, found `{actual}`"),
                )));
            }
        }
        other => {
            return Err(InstallError::Unsupported(format!(
                "cannot verify unsupported manifest kind `{other}`"
            )));
        }
    }

    Ok(None)
}

fn sha256_file(path: &Path) -> Result<String, InstallError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collision_issue(record: &PackageFileRecord, detail: &str) -> VerifyIssue {
    VerifyIssue {
        package: record.pkgname.clone(),
        path: record.path.clone(),
        kind: VerifyIssueKind::UnmanagedPathCollision,
        detail: detail.to_owned(),
    }
}

fn metadata_issue(record: &PackageFileRecord, detail: String) -> VerifyIssue {
    VerifyIssue {
        package: record.pkgname.clone(),
        path: record.path.clone(),
        kind: VerifyIssueKind::MetadataDrift,
        detail,
    }
}
