//! Human framing for command reports not covered by specialized renderers (`app_render_*`).

mod config_view;
mod extras;
mod forge_view;
mod helpers;
mod inspect;
mod maint_view;
mod plans;
mod profile_policy;
mod qa_view;
mod repo_net;
mod repo_net_remote;
mod repo_net_remote_trust;
mod review_view;
mod state_paths;
mod transactions;
mod trigger_view;
mod vendor_view;

pub(crate) use plans::render_extended_plan_report;

use crate::CommandReport;

#[must_use]
pub fn render_extended_human(report: &CommandReport) -> Option<String> {
    match report.area {
        "verify" => inspect::render_verify(report),
        "diff" => inspect::render_diff(report),
        "check" => inspect::render_check(report),
        "doctor" => inspect::render_doctor(report),
        "review" => review_view::render_review(report),
        "maint" => maint_view::render_maint(report),
        "init" => maint_view::render_init(report),
        "trigger" => trigger_view::render_trigger(report),
        "config" => config_view::render_config(report),
        "info" => inspect::render_info(report),
        "recipe" => inspect::render_recipe_check(report)
            .or_else(|| inspect::render_recipe_diff(report))
            .or_else(|| inspect::render_publish_ready(report))
            .or_else(|| extras::render_recipe_show(report)),
        "sync" => repo_net::render_sync(report),
        "cache" => repo_net::render_cache(report),
        "remote" => repo_net_remote::render_remote(report),
        "daemon" => repo_net::render_daemon(report),
        "profile" => profile_policy::render_profile(report),
        "flags" => profile_policy::render_flags(report),
        "policy" => profile_policy::render_policy(report),
        "upgrade" => transactions::render_upgrade(report),
        "remove" => transactions::render_remove(report),
        "downgrade" => transactions::render_downgrade(report),
        "recovery" => transactions::render_recovery(report),
        "rollback" => transactions::render_rollback_done(report),
        "ops" => transactions::render_ops(report),
        "qa" => qa_view::render_qa(report),
        "forge" => forge_view::render_forge(report),
        "vendor" => vendor_view::render_vendor(report),
        "extension" => extras::render_extension(report),
        "command" => extras::render_command_stub(report),
        "state" => state_paths::render_state_files(report)
            .or_else(|| state_paths::render_state_export_import(report)),
        _ => None,
    }
}
