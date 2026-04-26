use crate::CommandReport;
use crate::app_render_ci::render_ci_report;
use crate::app_render_support::{
    render_header, render_install_plan_report, render_install_success_report, render_json_block,
    render_recipe_catalog_report, render_recipe_removed_report, render_session_log_section,
};

#[must_use]
pub fn render_human(report: &CommandReport) -> String {
    if let Some(rendered) = render_specialized_report(report) {
        return append_session_log(rendered, report);
    }

    let rendered = match &report.details {
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
    };

    append_session_log(rendered, report)
}

fn render_specialized_report(report: &CommandReport) -> Option<String> {
    match (report.area, report.status) {
        ("install", "ok") => render_install_success_report(report),
        ("plan", "planned") => render_install_plan_report(report),
        ("ci", "ok") => render_ci_report(report),
        ("recipe", "ok") | ("recipe", "planned") => {
            if report
                .details
                .as_ref()
                .is_some_and(|details| details.get("catalog").is_some())
            {
                render_recipe_catalog_report(report)
            } else if report
                .details
                .as_ref()
                .is_some_and(|details| details.get("removed").is_some())
            {
                render_recipe_removed_report(report)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn append_session_log(rendered: String, report: &CommandReport) -> String {
    let Some(details) = &report.details else {
        return rendered;
    };
    let Some(section) = render_session_log_section(details) else {
        return rendered;
    };

    format!("{rendered}\n\n{section}")
}
