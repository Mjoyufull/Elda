use std::collections::BTreeSet;
use std::fs;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;

use super::model::{CiSubmissionRecord, PublishedPackageRecord};
use super::publish_plan::{PlannedCiWork, resolve_ci_targets};
use super::store::load_batch;
use super::workspace::{CiWorkspacePaths, list_json_records};

pub(super) fn qa_targets(
    app: &AppContext,
    operand: Option<&String>,
) -> Result<Vec<String>, CoreError> {
    if let Some(value) = operand {
        if let Ok(batch) = load_batch(&CiWorkspacePaths::new(app.database.layout()), value) {
            return Ok(batch.packages);
        }
        return resolve_ci_targets(app, std::slice::from_ref(value));
    }

    let mut targets = fs::read_dir(&app.database.layout().recipes_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry)
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if targets.is_empty() {
        return Err(CoreError::Operator(
            "no local recipes exist for qa work".to_owned(),
        ));
    }

    targets.sort();
    Ok(targets)
}

pub(super) fn qa_plan_json(plan: &PlannedCiWork) -> serde_json::Value {
    json!({
        "requested_targets": plan.requested_targets,
        "packages": plan
            .packages
            .iter()
            .map(|package| json!({
                "pkgname": package.package_name,
                "layer": package.layer,
                "runtime_depends": package.runtime_depends,
                "makedepends": package.makedepends,
                "checkdepends": package.checkdepends,
            }))
            .collect::<Vec<_>>(),
    })
}

pub(super) fn latest_published_package(
    app: &AppContext,
    package: &str,
) -> Result<Option<PublishedPackageRecord>, CoreError> {
    let workspace = CiWorkspacePaths::new(app.database.layout());
    let mut submissions = list_json_records::<CiSubmissionRecord>(&workspace.submissions_dir)?;
    submissions.sort_by(|left, right| left.updated_at.cmp(&right.updated_at));

    Ok(submissions.into_iter().rev().find_map(|submission| {
        submission
            .published_packages
            .into_iter()
            .find(|record| record.pkgname == package)
    }))
}
