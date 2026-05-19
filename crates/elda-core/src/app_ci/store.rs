use std::fs;

use crate::app::AppContext;
use crate::config::SubmissionMode;
use crate::error::CoreError;

use super::model::{CiBatchRecord, CiSubmissionRecord};
use super::workspace::{
    CiWorkspacePaths, current_unix_timestamp, list_json_records, read_json, write_json,
};

pub(super) fn load_batch(
    workspace: &CiWorkspacePaths,
    batch_name: &str,
) -> Result<CiBatchRecord, CoreError> {
    read_json(&workspace.batches_dir.join(format!("{batch_name}.json")))
}

pub(super) fn save_batch(
    workspace: &CiWorkspacePaths,
    batch: &CiBatchRecord,
) -> Result<(), CoreError> {
    write_json(
        &workspace.batches_dir.join(format!("{}.json", batch.name)),
        batch,
    )
}

pub(super) fn load_batches(workspace: &CiWorkspacePaths) -> Result<Vec<CiBatchRecord>, CoreError> {
    let mut batches = list_json_records::<CiBatchRecord>(&workspace.batches_dir)?;
    batches.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(batches)
}

pub(super) fn save_submission(
    workspace: &CiWorkspacePaths,
    submission: &CiSubmissionRecord,
) -> Result<(), CoreError> {
    write_json(
        &workspace
            .submissions_dir
            .join(format!("{}.json", submission.id)),
        submission,
    )
}

pub(super) fn load_submissions(
    workspace: &CiWorkspacePaths,
) -> Result<Vec<CiSubmissionRecord>, CoreError> {
    let mut submissions = list_json_records::<CiSubmissionRecord>(&workspace.submissions_dir)?;
    submissions.sort_by_key(|submission| submission.updated_at);
    Ok(submissions)
}

pub(super) fn find_submission(
    submissions: &[CiSubmissionRecord],
    selection: &str,
) -> Option<CiSubmissionRecord> {
    submissions
        .iter()
        .find(|submission| {
            submission.id == selection
                || submission
                    .packages
                    .iter()
                    .any(|package| package == selection)
                || submission
                    .requested_targets
                    .iter()
                    .any(|target| target == selection)
        })
        .cloned()
}

pub(super) fn local_recipe_names(app: &AppContext) -> Result<Vec<String>, CoreError> {
    let mut names = fs::read_dir(&app.database.layout().recipes_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry)
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

pub(super) fn submission_id(targets: &[String]) -> String {
    format!(
        "{}-{}",
        current_unix_timestamp(),
        targets
            .first()
            .map(|target| sanitize_name(target))
            .unwrap_or_else(|| "submission".to_owned())
    )
}

pub(super) fn branch_name(targets: &[String], batch_name: Option<&str>) -> String {
    if let Some(batch_name) = batch_name {
        return format!("elda/batch/{}", sanitize_name(batch_name));
    }

    format!(
        "elda/{}",
        targets
            .first()
            .map(|target| sanitize_name(target))
            .unwrap_or_else(|| "submission".to_owned())
    )
}

fn sanitize_name(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

pub(super) fn submission_mode_name(mode: SubmissionMode) -> &'static str {
    match mode {
        SubmissionMode::Pr => "pr",
        SubmissionMode::Push => "push",
    }
}
