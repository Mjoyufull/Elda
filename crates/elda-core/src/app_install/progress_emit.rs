//! Helpers for emitting [`crate::progress::ProgressEvent`]s during the
//! install execution flow.
//!
//! The structured `progress` array on the JSON report is still populated
//! by [`super::progress`]. This module mirrors the same step ids onto the
//! live sink so the operator sees a tree-style frame as the install runs
//! instead of waiting for the post-action dump.

use elda_build::BuiltPackage;
use elda_install::InstallReport;

use crate::app::PlannedInstallAction;
use crate::progress::{FrameId, FrameOutcome, ProgressEvent, ProgressSink};

pub(crate) fn emit_frame_start(
    sink: &dyn ProgressSink,
    frame: FrameId,
    action: &PlannedInstallAction,
) {
    sink.emit(ProgressEvent::FrameStart {
        frame,
        title: format!(
            "Install ({}/{})",
            action.resolved.selected_lane, action.resolved.selected_source_kind
        ),
        subject: Some(action.package_name.clone()),
    });
}

pub(crate) fn emit_already_installed_frame(
    sink: &dyn ProgressSink,
    frame: FrameId,
    action: &PlannedInstallAction,
) {
    emit_step_done(
        sink,
        frame,
        "reuse-installed-state",
        "reuse installed state",
        Some(format!(
            "{} already at the requested version",
            action.package_name
        )),
    );
    sink.emit(ProgressEvent::FrameEnd {
        frame,
        outcome: FrameOutcome::Ok,
        summary: Some(format!("{} already-installed", action.package_name)),
    });
}

pub(crate) fn emit_acquire_and_build_done(
    sink: &dyn ProgressSink,
    frame: FrameId,
    action: &PlannedInstallAction,
    package: &BuiltPackage,
) {
    emit_step_done(
        sink,
        frame,
        "build-inner",
        "build source",
        Some(build_summary(action)),
    );
    emit_step_done(
        sink,
        frame,
        "acquire-source",
        "acquire source",
        Some("source ready".to_owned()),
    );
    let kind = action.resolved.selected_source_kind.as_str();
    if kind == "git"
        || kind == "nix_flake"
        || kind == "gentoo_overlay"
        || kind == "aur_pkgbuild"
        || kind == "xbps_template"
    {
        emit_step_done(
            sink,
            frame,
            "fetch-source",
            "fetch source",
            action.resolved.source_ref.clone(),
        );
        emit_step_done(
            sink,
            frame,
            "build-source",
            "build source payload",
            Some(build_summary(action)),
        );
    } else if matches!(
        kind,
        "url_archive" | "github_release" | "release_asset" | "appimage"
    ) {
        emit_step_done(
            sink,
            frame,
            "fetch-binary",
            "fetch binary",
            action.resolved.source_ref.clone(),
        );
        if let Some(verification) = &action.resolved.binary_source_verification {
            emit_step_done(
                sink,
                frame,
                "verify-binary-source",
                "verify binary source",
                Some(format!("remote `{}`", verification.remote_name)),
            );
        }
    } else {
        emit_step_done(
            sink,
            frame,
            "prepare-source",
            format!("prepare {kind} source"),
            None,
        );
    }
    emit_step_done(
        sink,
        frame,
        "stage-payload",
        "stage payload",
        Some(format!(
            "{} bytes payload manifest",
            package.manifest_hash.len()
        )),
    );
    emit_step_done(
        sink,
        frame,
        "analyze-staged-objects",
        "analyze staged objects",
        Some(analyze_summary(package)),
    );
}

pub(crate) fn emit_install_completed(
    sink: &dyn ProgressSink,
    frame: FrameId,
    action: &PlannedInstallAction,
    install: &InstallReport,
) {
    emit_step_done(
        sink,
        frame,
        "activate",
        "activate",
        Some(format!("backend {}", install.activation_backend)),
    );
    emit_step_done(
        sink,
        frame,
        "record-installed-state",
        "record installed state",
        Some(format!(
            "state {} ({} path(s))",
            install.state_id, install.installed_paths
        )),
    );
    sink.emit(ProgressEvent::FrameEnd {
        frame,
        outcome: FrameOutcome::Ok,
        summary: Some(format!("{} ok", action.package_name)),
    });
}

pub(crate) fn emit_step_started(
    sink: &dyn ProgressSink,
    frame: FrameId,
    step: &'static str,
    label: impl Into<String>,
    detail: Option<String>,
    live_spinner: bool,
) {
    sink.emit(ProgressEvent::StepStarted {
        frame,
        step,
        label: label.into(),
        detail,
        live_spinner,
    });
}

fn emit_step_done(
    sink: &dyn ProgressSink,
    frame: FrameId,
    step: &'static str,
    label: impl Into<String>,
    summary: Option<String>,
) {
    sink.emit(ProgressEvent::StepDone {
        frame,
        step,
        label: label.into(),
        summary,
    });
}

pub(crate) fn emit_frame_blocked(
    sink: &dyn ProgressSink,
    frame: FrameId,
    step: &'static str,
    label: impl Into<String>,
    reason: String,
) {
    sink.emit(ProgressEvent::StepBlocked {
        frame,
        step,
        label: label.into(),
        reason: reason.clone(),
        action: None,
    });
    sink.emit(ProgressEvent::FrameEnd {
        frame,
        outcome: FrameOutcome::Blocked,
        summary: Some(reason),
    });
}

fn build_summary(action: &PlannedInstallAction) -> String {
    if let Some(build) = &action.resolved.recipe.package.build {
        return format!("{} pipeline", build.system);
    }
    if matches!(
        action.resolved.recipe.package.kind.as_str(),
        "meta" | "profile"
    ) {
        return format!(
            "{} package, no payload build",
            action.resolved.recipe.package.kind
        );
    }
    if action.resolved.ad_hoc_git {
        return "auto-detected build system".to_owned();
    }
    "build pipeline".to_owned()
}

fn analyze_summary(package: &BuiltPackage) -> String {
    let requires = package.object_metadata.shlib_requires.len();
    let provides = package.object_metadata.shlib_provides.len();
    if requires == 0 && provides == 0 {
        return "no shared-library metadata".to_owned();
    }
    format!("{requires} require(s), {provides} provide(s)")
}
