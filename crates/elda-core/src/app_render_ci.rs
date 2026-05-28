use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{render_header, render_section};

pub(crate) fn render_ci_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;

    if details.get("log_path").is_some() && details.get("content").is_some() {
        return Some(render_logs_report(report, details));
    }
    if let Some(submission) = details.get("submission") {
        return Some(render_submission_report(report, submission));
    }
    if let Some(batch) = details.get("batch") {
        return Some(render_batch_report(report, batch));
    }
    if details.get("submissions").is_some() {
        return Some(render_status_report(report, details));
    }
    if details.get("submission_id").is_some() {
        return Some(render_submission_reference_report(report, details));
    }

    None
}

fn render_submission_report(report: &CommandReport, submission: &Value) -> String {
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];

    let mut submission_lines = vec![
        format!(
            "id: {}",
            json_string(submission, &["id"]).unwrap_or("unknown")
        ),
        format!(
            "state: {}",
            json_string(submission, &["state"]).unwrap_or("unknown")
        ),
        format!(
            "mode: {}",
            json_string(submission, &["mode"]).unwrap_or("unknown")
        ),
        format!(
            "branch: {}",
            json_string(submission, &["branch_name"]).unwrap_or("unknown")
        ),
    ];
    if let Some(target_branch) = json_string(submission, &["target_branch"]) {
        submission_lines.push(format!("target branch: {target_branch}"));
    }
    if let Some(targets) = string_array(submission, &["requested_targets"]) {
        submission_lines.push(format!("targets: {targets}"));
    }
    if let Some(packages) = string_array(submission, &["packages"]) {
        submission_lines.push(format!("packages: {packages}"));
    }
    sections.push(render_section("Submission", &submission_lines));

    let mut scheduler_lines = vec![format!(
        "attempts: {}",
        json_u64(submission, &["attempts"]).unwrap_or(0)
    )];
    if let Some(planned_layers) = json_u64(submission, &["planned_layers"]) {
        let completed_layers = json_u64(submission, &["completed_layers"]).unwrap_or(0);
        scheduler_lines.push(format!("layers: {completed_layers}/{planned_layers}"));
    }
    if let Some(queued_at) = json_u64(submission, &["queued_at"]) {
        scheduler_lines.push(format!("queued at: {queued_at}"));
    }
    if let Some(started_at) = json_u64(submission, &["started_at"]) {
        scheduler_lines.push(format!("started at: {started_at}"));
    }
    if let Some(completed_at) = json_u64(submission, &["completed_at"]) {
        scheduler_lines.push(format!("completed at: {completed_at}"));
    }
    if let Some(last_error) = json_string(submission, &["last_error"]) {
        scheduler_lines.push(format!("last error: {last_error}"));
    }
    sections.push(render_section("Scheduler", &scheduler_lines));

    let mut remote_lines = vec![format!(
        "remote: {}",
        json_string(submission, &["remote_name"]).unwrap_or("local-only")
    )];
    if let Some(remote_url) = json_string(submission, &["remote_url"]) {
        remote_lines.push(format!("url: {remote_url}"));
    }
    if let Some(pushed_ref) = json_string(submission, &["pushed_ref"]) {
        remote_lines.push(format!("ref: {pushed_ref}"));
    }
    if let Some(pushed_commit) = json_string(submission, &["pushed_commit"]) {
        remote_lines.push(format!("commit: {pushed_commit}"));
    }
    sections.push(render_section("Remote", &remote_lines));

    if let Some(review_url) = json_string(submission, &["review_url"]) {
        let mut review_lines = vec![format!("url: {review_url}")];
        if let Some(review_kind) = json_string(submission, &["review_kind"]) {
            review_lines.push(format!("kind: {review_kind}"));
        }
        if let Some(review_id) = json_string(submission, &["review_id"]) {
            review_lines.push(format!("id: {review_id}"));
        }
        sections.push(render_section("Review", &review_lines));
    }

    let mut artifact_lines = vec![format!(
        "packages repo: {}",
        json_string(submission, &["packages_repo_path"]).unwrap_or("unknown")
    )];
    if let Some(log_path) = json_string(submission, &["log_path"]) {
        artifact_lines.push(format!("log: {log_path}"));
    }
    if let Some(lock_path) = json_string(submission, &["lock_path"]) {
        artifact_lines.push(format!("lock: {lock_path}"));
    }
    if let Some(index_path) = json_string(submission, &["index_path"]) {
        artifact_lines.push(format!("index: {index_path}"));
    }
    if let Some(count) = submission
        .get("published_packages")
        .and_then(Value::as_array)
        .map(|packages| packages.len())
    {
        artifact_lines.push(format!("published packages: {count}"));
    }
    sections.push(render_section("Artifacts", &artifact_lines));

    sections.join("\n\n")
}

fn render_batch_report(report: &CommandReport, batch: &Value) -> String {
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    let mut lines = vec![
        format!(
            "name: {}",
            json_string(batch, &["name"]).unwrap_or("unknown")
        ),
        format!(
            "state: {}",
            json_string(batch, &["state"]).unwrap_or("unknown")
        ),
    ];
    if let Some(packages) = string_array(batch, &["packages"]) {
        lines.push(format!("packages: {packages}"));
    }
    if let Some(last_submission_id) = json_string(batch, &["last_submission_id"]) {
        lines.push(format!("last submission: {last_submission_id}"));
    }
    sections.push(render_section("Batch", &lines));
    sections.join("\n\n")
}

fn render_status_report(report: &CommandReport, details: &Value) -> String {
    let submissions = details
        .get("submissions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];

    let mut queue_lines = vec![format!(
        "pending: {}",
        json_u64(details, &["pending_count"]).unwrap_or(0)
    )];
    if let Some(state_counts) = details.get("state_counts").and_then(Value::as_object) {
        for (state, count) in state_counts {
            queue_lines.push(format!("{state}: {}", count.as_u64().unwrap_or(0)));
        }
    }
    sections.push(render_section("Queue", &queue_lines));

    let lines = if submissions.is_empty() {
        vec!["no submissions".to_owned()]
    } else {
        submissions
            .iter()
            .map(|submission| {
                format!(
                    "{} [{} / {}] {} attempts={} layers={}/{}",
                    json_string(submission, &["id"]).unwrap_or("unknown"),
                    json_string(submission, &["state"]).unwrap_or("unknown"),
                    json_string(submission, &["mode"]).unwrap_or("unknown"),
                    json_string(submission, &["branch_name"]).unwrap_or("unknown"),
                    json_u64(submission, &["attempts"]).unwrap_or(0),
                    json_u64(submission, &["completed_layers"]).unwrap_or(0),
                    json_u64(submission, &["planned_layers"]).unwrap_or(0),
                )
            })
            .collect()
    };
    sections.push(render_section("Submissions", &lines));
    sections.join("\n\n")
}

fn render_submission_reference_report(report: &CommandReport, details: &Value) -> String {
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    let mut lines = vec![format!(
        "submission: {}",
        json_string(details, &["submission_id"]).unwrap_or("unknown")
    )];
    if let Some(mode) = json_string(details, &["mode"]) {
        lines.push(format!("mode: {mode}"));
    }
    if let Some(state) = json_string(details, &["state"]) {
        lines.push(format!("state: {state}"));
    }
    if let Some(branch_name) = json_string(details, &["branch_name"]) {
        lines.push(format!("branch: {branch_name}"));
    }
    if let Some(target_branch) = json_string(details, &["target_branch"]) {
        lines.push(format!("target branch: {target_branch}"));
    }
    if let Some(remote_name) = json_string(details, &["remote_name"]) {
        lines.push(format!("remote: {remote_name}"));
    }
    if let Some(pushed_ref) = json_string(details, &["pushed_ref"]) {
        lines.push(format!("ref: {pushed_ref}"));
    }
    if let Some(review_kind) = json_string(details, &["review_kind"]) {
        lines.push(format!("review kind: {review_kind}"));
    }
    if let Some(review_id) = json_string(details, &["review_id"]) {
        lines.push(format!("review id: {review_id}"));
    }
    if let Some(pr_url) = json_string(details, &["pr_url"]) {
        lines.push(format!("review URL: {pr_url}"));
    }
    sections.push(render_section("Reference", &lines));
    sections.join("\n\n")
}

fn render_logs_report(report: &CommandReport, details: &Value) -> String {
    let mut sections = vec![
        render_header(report.area, report.status),
        report.summary.clone(),
    ];
    let mut lines = vec![format!(
        "submission: {}",
        json_string(details, &["submission_id"]).unwrap_or("unknown")
    )];
    if let Some(state) = json_string(details, &["state"]) {
        lines.push(format!("state: {state}"));
    }
    if let Some(attempts) = json_u64(details, &["attempts"]) {
        lines.push(format!("attempts: {attempts}"));
    }
    if let Some(log_path) = json_string(details, &["log_path"]) {
        lines.push(format!("path: {log_path}"));
    }
    sections.push(render_section("Log", &lines));

    let content = json_string(details, &["content"]).unwrap_or("");
    sections.push(render_section(
        "Content",
        &content.lines().map(ToOwned::to_owned).collect::<Vec<_>>(),
    ));
    sections.join("\n\n")
}

fn json_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_u64()
}

fn string_array(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(
        current
            .as_array()?
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
    )
}
