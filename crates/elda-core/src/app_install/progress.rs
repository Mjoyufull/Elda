use serde_json::json;

use crate::app::PlannedInstallAction;
use elda_build::BuiltPackage;
use elda_db::InstallationMode;
use elda_install::{InstallReport, SnapshotRecord};

#[derive(Debug, Clone, Copy)]
enum ProgressStatus {
    Planned,
    Done,
    Skipped,
}

impl ProgressStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Done => "done",
            Self::Skipped => "skipped",
        }
    }
}

pub(crate) fn planned_activation_backend(mode: InstallationMode) -> &'static str {
    match mode {
        InstallationMode::Prefix => "prefix-copy",
        InstallationMode::System => "linux-copy",
    }
}

pub(crate) fn install_progress_for_plan(
    action: &PlannedInstallAction,
    activation_backend: &str,
) -> Vec<serde_json::Value> {
    install_progress_steps(
        action,
        ProgressStatus::Planned,
        analyze_detail(None),
        format!("backend {activation_backend}"),
        None,
    )
}

pub(crate) fn install_progress_for_existing(
    action: &PlannedInstallAction,
    activation_backend: &str,
) -> Vec<serde_json::Value> {
    let mut steps = vec![progress_step(
        "reuse-installed-state",
        ProgressStatus::Done,
        Some("requested package already matches the selected version and variant".to_owned()),
    )];
    steps.extend(install_progress_steps(
        action,
        ProgressStatus::Skipped,
        analyze_detail(None),
        format!("backend {activation_backend}"),
        None,
    ));
    steps
}

pub(crate) fn install_progress_for_completed(
    action: &PlannedInstallAction,
    package: &BuiltPackage,
    install: &InstallReport,
) -> Vec<serde_json::Value> {
    let mut steps = install_progress_steps(
        action,
        ProgressStatus::Done,
        analyze_detail(Some(package)),
        format!("backend {}", install.activation_backend),
        replacement_detail(action),
    );
    steps.push(progress_step(
        "record-installed-state",
        ProgressStatus::Done,
        Some(format!(
            "state {} with {} managed path(s)",
            install.state_id, install.installed_paths
        )),
    ));
    let (snapshot_status, snapshot_detail) = snapshot_progress(&install.snapshots);
    steps.push(progress_step(
        "snapshot-hooks",
        snapshot_status,
        Some(snapshot_detail),
    ));
    steps
}

fn install_progress_steps(
    action: &PlannedInstallAction,
    status: ProgressStatus,
    analyze_detail: String,
    activation_detail: String,
    replacement_detail: Option<String>,
) -> Vec<serde_json::Value> {
    let mut steps = Vec::new();

    if let Some(path) = &action.resolved.generated_recipe_dir {
        steps.push(progress_step(
            "review-generated-metadata",
            status,
            Some(format!("recipe tree {}", path.display())),
        ));
    }

    if let Some(source) = &action.resolved.remote_recipe_source {
        steps.push(progress_step(
            "fetch-package-definition",
            status,
            Some(format!(
                "remote `{}` at commit {}",
                source.remote_name,
                short_commit(&source.repo_commit)
            )),
        ));
    }

    match action.resolved.selected_source_kind.as_str() {
        "git" => {
            steps.push(progress_step(
                "fetch-source",
                status,
                source_fetch_detail(action),
            ));
            steps.push(progress_step(
                "build-source",
                status,
                Some(source_build_detail(action)),
            ));
        }
        "url_archive" | "github_release" => {
            steps.push(progress_step(
                "fetch-binary",
                status,
                Some(binary_fetch_detail(action)),
            ));
            steps.push(progress_step(
                "verify-binary-source",
                status,
                binary_verify_detail(action),
            ));
        }
        other => {
            steps.push(progress_step(
                "prepare-source",
                status,
                Some(format!("source kind {other}")),
            ));
        }
    }

    if let Some(detail) = upgrade_detail(action) {
        steps.push(progress_step(
            "replace-installed-version",
            status,
            Some(detail),
        ));
    }
    if let Some(detail) = replacement_detail {
        steps.push(progress_step(
            "remove-replaced-packages",
            status,
            Some(detail),
        ));
    }

    steps.push(progress_step(
        "stage-payload",
        status,
        Some(format!("assemble payload for {}", action.package_name)),
    ));
    steps.push(progress_step(
        "analyze-staged-objects",
        status,
        Some(analyze_detail),
    ));
    steps.push(progress_step("activate", status, Some(activation_detail)));

    steps
}

fn analyze_detail(package: Option<&BuiltPackage>) -> String {
    let Some(package) = package else {
        return "pending staged object scan".to_owned();
    };

    let requires = package.object_metadata.shlib_requires.len();
    let provides = package.object_metadata.shlib_provides.len();
    if requires == 0 && provides == 0 {
        return "no shared-library metadata detected".to_owned();
    }

    format!("{requires} shlib require(s), {provides} shlib provide(s)")
}

fn source_fetch_detail(action: &PlannedInstallAction) -> Option<String> {
    action
        .resolved
        .source_ref
        .clone()
        .or_else(|| {
            action.resolved.remote_recipe_source.as_ref().map(|source| {
                format!(
                    "{}#{}",
                    source.packages_url,
                    short_commit(&source.repo_commit)
                )
            })
        })
        .or_else(|| Some("resolved source input".to_owned()))
}

fn source_build_detail(action: &PlannedInstallAction) -> String {
    if matches!(
        action.resolved.recipe.package.kind.as_str(),
        "meta" | "profile"
    ) {
        return format!(
            "{} package; no compiled payload build is required",
            action.resolved.recipe.package.kind
        );
    }

    if let Some(build) = &action.resolved.recipe.package.build {
        return format!("{} pipeline", build.system);
    }

    if action.resolved.ad_hoc_git {
        return "auto-detect build system for ad hoc git source".to_owned();
    }

    "auto-detect build system for the selected source lane".to_owned()
}

fn binary_fetch_detail(action: &PlannedInstallAction) -> String {
    if let Some(remote_name) = &action.resolved.remote_name {
        return format!("remote `{remote_name}` binary lane");
    }
    if let Some(source_ref) = &action.resolved.source_ref {
        return source_ref.clone();
    }

    format!("{} binary lane", action.package_name)
}

fn binary_verify_detail(action: &PlannedInstallAction) -> Option<String> {
    let verification = action.resolved.binary_source_verification.as_ref()?;
    if verification.payload_signature.is_some() {
        return Some(format!(
            "verified payload signature for remote `{}`",
            verification.remote_name
        ));
    }

    Some(format!(
        "verified payload origin for remote `{}`",
        verification.remote_name
    ))
}

fn upgrade_detail(action: &PlannedInstallAction) -> Option<String> {
    let installed = action.already_installed.as_ref()?;
    let next = format!(
        "{}:{}-{}",
        action.resolved.recipe.package.epoch,
        action.resolved.recipe.package.version,
        action.resolved.recipe.package.rel
    );
    let current = format!(
        "{}:{}-{}",
        installed.epoch, installed.pkgver, installed.pkgrel
    );
    (current != next).then_some(format!("upgrade from {current} to {next}"))
}

fn replacement_detail(action: &PlannedInstallAction) -> Option<String> {
    (!action.replaced_packages.is_empty()).then(|| action.replaced_packages.join(", "))
}

fn snapshot_progress(snapshots: &[SnapshotRecord]) -> (ProgressStatus, String) {
    if snapshots.is_empty() {
        return (
            ProgressStatus::Skipped,
            "no configured activation snapshots".to_owned(),
        );
    }

    let tool = snapshots
        .first()
        .map(|snapshot| snapshot.tool.as_str())
        .unwrap_or("unknown");
    let captured = snapshots
        .iter()
        .filter(|snapshot| snapshot.status == "captured")
        .count();
    let failed = snapshots
        .iter()
        .filter(|snapshot| snapshot.status == "failed")
        .count();

    let mut detail = format!("{} request(s) via {tool}", snapshots.len());
    if captured > 0 {
        detail.push_str(&format!(", {captured} captured"));
    }
    if failed > 0 {
        detail.push_str(&format!(", {failed} failed"));
    }

    (ProgressStatus::Done, detail)
}

fn progress_step(step: &str, status: ProgressStatus, detail: Option<String>) -> serde_json::Value {
    json!({
        "step": step,
        "status": status.as_str(),
        "detail": detail,
    })
}

fn short_commit(commit: &str) -> &str {
    let end = commit.len().min(12);
    &commit[..end]
}
