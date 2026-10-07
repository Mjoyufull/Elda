use crate::CommandReport;
use crate::app_render_support::{json_string, render_header};
use crate::app_render_tree::{FrameFooter, Glyph, TreeStyle, frame_from_sections};

pub(crate) fn render_self_update_report(report: &CommandReport) -> Option<String> {
    if report.area != "self-update" {
        return None;
    }
    let details = report.details.as_ref()?;
    let repository = json_string(details, &["repository"])?;
    let branch = json_string(details, &["branch"])?;
    let executable = json_string(details, &["executable"])?;
    let mut lines = vec![
        format!("repository: {repository}"),
        format!("branch: {branch}"),
        format!("executable: {executable}"),
    ];
    if let Some(commit) = json_string(details, &["commit"]) {
        lines.push(format!("commit: {commit}"));
    }
    if let Some(version) = json_string(details, &["version"]) {
        lines.push(format!("version: {version}"));
    }

    let footer = FrameFooter {
        glyph: (!report.dry_run).then_some(Glyph::Done),
        text: report.summary.clone(),
    };
    let frame = frame_from_sections(
        "Elda Self Update",
        &[("Source".to_owned(), lines)],
        Some(footer),
    );
    Some(format!(
        "{}\n{}",
        render_header(report.area, report.status),
        frame.render(TreeStyle::detect())
    ))
}
