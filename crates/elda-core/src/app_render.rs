use crate::CommandReport;
use crate::app_render_support::{render_header, render_json_block};

#[must_use]
pub fn render_human(report: &CommandReport) -> String {
    match &report.details {
        Some(details) => format!(
            "{}\n{}\n{}",
            render_header(report.area, report.status),
            report.summary,
            render_json_block(details),
        ),
        None => format!(
            "{}\n{}",
            render_header(report.area, report.status),
            report.summary
        ),
    }
}
