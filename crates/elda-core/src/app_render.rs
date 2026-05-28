use crate::CommandReport;
use crate::app_render_appimage::render_appimage_report;
use crate::app_render_ci::render_ci_report;
use crate::app_render_git::render_git_report;
use crate::app_render_host::render_host_report;
use crate::app_render_install::{render_install_plan_report, render_install_success_report};
use crate::app_render_migration::render_migration_report;
use crate::app_render_misc::{
    render_failure_report, render_metadata_add_report, render_recipe_catalog_report,
    render_recipe_removed_report, render_search_report, render_session_log_section,
};
use crate::app_render_remove::{render_remove_plan_report, render_remove_success_report};
use crate::app_render_state::{render_installed_packages_report, render_state_show_report};
use crate::app_render_support::render_header;
use crate::app_version::render_version_report;
use crate::render_style::highlight_operator_frame;

#[must_use]
pub fn render_human(report: &CommandReport) -> String {
    if let Some(rendered) = render_specialized_report(report) {
        return append_session_log(highlight_operator_frame(&rendered), report);
    }

    let rendered = if let Some(framed) = crate::app_render_extended::render_extended_human(report) {
        highlight_operator_frame(&framed)
    } else {
        format!(
            "{}\n{}",
            render_header(report.area, report.status),
            report.summary,
        )
    };

    append_session_log(rendered, report)
}

fn render_specialized_report(report: &CommandReport) -> Option<String> {
    if report.status == "blocked" {
        return render_failure_report(report);
    }

    match (report.area, report.status) {
        ("install", "ok") => render_install_success_report(report),
        ("remove", "ok") => render_remove_success_report(report),
        ("plan", "planned") => render_install_plan_report(report)
            .or_else(|| render_remove_plan_report(report))
            .or_else(|| crate::app_render_extended::render_extended_plan_report(report)),
        ("version", "ok") => render_version_report(report),
        ("ci", "ok") => render_ci_report(report),
        ("host", "ok") | ("host", "issues") | ("host", "blocked") => render_host_report(report),
        ("publish", "ok") | ("publish", "planned") => None,
        ("git", "ok") => render_git_report(report),
        ("appimage", "ok") => render_appimage_report(report),
        ("metadata", "ok") | ("metadata", "planned") => render_metadata_add_report(report),
        ("migration", "ok") | ("migration", "planned") => render_migration_report(report),
        ("search", "ok") => render_search_report(report),
        ("state", "ok") => {
            render_installed_packages_report(report).or_else(|| render_state_show_report(report))
        }
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
