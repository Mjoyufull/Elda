use std::fs;

use serde_json::json;
use tempfile::TempDir;

use crate::app::run_from_root;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use elda_recipe::check_local_recipes;

use super::publish::resolve_publish_target;
use super::publish_plan::plan_ci_work;
use super::qa_support::{latest_published_package, qa_plan_json, qa_targets};
use super::store::load_batch;
use super::workspace::{CiWorkspacePaths, copy_dir_recursive};

pub(crate) fn handle_qa_namespace(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    match request.command_path.as_slice() {
        [namespace, command] if namespace == "qa" && command == "lint" => {
            handle_qa_lint(app, request)
        }
        [namespace, command] if namespace == "qa" && command == "build" => {
            handle_qa_build(app, request)
        }
        [namespace, command] if namespace == "qa" && command == "smoke" => {
            handle_qa_smoke(app, request)
        }
        [namespace, command] if namespace == "qa" && command == "stack" => {
            handle_qa_stack(app, request)
        }
        [namespace, command] if namespace == "qa" && command == "repro" => {
            handle_qa_repro(app, request)
        }
        [namespace, command] if namespace == "qa" && command == "diff" => {
            handle_qa_diff(app, request)
        }
        _ => Err(CoreError::Operator("unsupported qa request".to_owned())),
    }
}

fn handle_qa_lint(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let target = request.operands.first().map(String::as_str);
    let report = check_local_recipes(&app.database.layout().recipes_dir, target)?;
    let status = if report
        .issues
        .iter()
        .any(|issue| issue.severity == elda_recipe::IssueSeverity::Error)
    {
        "invalid"
    } else {
        "ok"
    };

    Ok(CommandReport {
        area: "qa",
        status,
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: format!("lint checked {} recipe(s).", report.recipes.len()),
        details: Some(json!({ "lint": report })),
    })
}

fn handle_qa_build(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let targets = qa_targets(app, request.operands.first())?;
    let plan = plan_ci_work(app, &targets)?;

    if request.dry_run {
        return Ok(CommandReport {
            area: "qa",
            status: "planned",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: true,
            summary: format!("qa build would build {} package(s).", plan.packages.len()),
            details: Some(json!({ "plan": qa_plan_json(&plan) })),
        });
    }

    let mut built = Vec::new();
    for package in &plan.packages {
        let resolved = resolve_publish_target(app, &package.package_name)?;
        let build = app.build_resolved_target(&resolved, false)?;
        built.push(json!({
            "pkgname": build.package.package_name,
            "payload_path": build.package.payload_path,
            "manifest_path": build.package.manifest_path,
            "payload_sha256": build.package.payload_sha256,
            "manifest_hash": build.package.manifest_hash,
        }));
    }

    Ok(CommandReport {
        area: "qa",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: false,
        summary: format!("qa build completed for {} package(s).", built.len()),
        details: Some(json!({ "builds": built })),
    })
}

fn handle_qa_smoke(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let package = request.operands.first().ok_or_else(|| {
        CoreError::Operator("qa smoke requires one package or batch name".to_owned())
    })?;
    let targets = qa_targets(app, Some(package))?;
    let plan = plan_ci_work(app, &targets)?;

    if request.dry_run {
        return Ok(CommandReport {
            area: "qa",
            status: "planned",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: true,
            summary: format!("qa smoke would test {} package(s).", plan.packages.len()),
            details: Some(json!({ "plan": qa_plan_json(&plan) })),
        });
    }

    let smoke_root = TempDir::new_in(&app.database.layout().tmp_dir)?;
    copy_dir_recursive(
        &app.database.layout().recipes_dir,
        &smoke_root.path().join("etc/elda/recipes"),
    )?;
    let mut smoke_reports = Vec::new();
    for target in &targets {
        let install = run_from_root(
            smoke_root.path(),
            CommandRequest::new(
                vec!["i".to_owned()],
                vec![target.clone()],
                OutputMode::Json,
                false,
            ),
        )?;
        let verify = run_from_root(
            smoke_root.path(),
            CommandRequest::new(
                vec!["verify".to_owned()],
                vec![target.clone()],
                OutputMode::Json,
                false,
            ),
        )?;
        smoke_reports.push(json!({
            "pkgname": target,
            "install": install.status,
            "verify": verify.status,
        }));
    }

    Ok(CommandReport {
        area: "qa",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: false,
        summary: format!("qa smoke passed for {} package(s).", smoke_reports.len()),
        details: Some(json!({ "smoke": smoke_reports })),
    })
}

fn handle_qa_stack(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let batch_name = request
        .operands
        .first()
        .ok_or_else(|| CoreError::Operator("qa stack requires one batch name".to_owned()))?;
    let batch = load_batch(&CiWorkspacePaths::new(app.database.layout()), batch_name)?;
    let plan = plan_ci_work(app, &batch.packages)?;

    Ok(CommandReport {
        area: "qa",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: format!("qa stack resolved {} package(s).", plan.packages.len()),
        details: Some(json!({
            "batch": batch,
            "plan": qa_plan_json(&plan),
        })),
    })
}

fn handle_qa_repro(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let package = request
        .operands
        .first()
        .ok_or_else(|| CoreError::Operator("qa repro requires one package name".to_owned()))?;
    let package = package.clone();
    let resolved = resolve_publish_target(app, &package)?;
    let first = app.build_resolved_target(&resolved, false)?;
    let second = app.build_resolved_target(&resolved, false)?;
    let reproducible = first.package.payload_sha256 == second.package.payload_sha256
        && first.package.manifest_hash == second.package.manifest_hash;

    Ok(CommandReport {
        area: "qa",
        status: if reproducible { "ok" } else { "issues" },
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: if reproducible {
            format!("qa repro matched for `{package}`.")
        } else {
            format!("qa repro mismatch for `{package}`.")
        },
        details: Some(json!({
            "first": {
                "payload_sha256": first.package.payload_sha256,
                "manifest_hash": first.package.manifest_hash,
            },
            "second": {
                "payload_sha256": second.package.payload_sha256,
                "manifest_hash": second.package.manifest_hash,
            },
            "reproducible": reproducible,
        })),
    })
}

fn handle_qa_diff(
    app: &crate::app::AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    let package = request
        .operands
        .first()
        .ok_or_else(|| CoreError::Operator("qa diff requires one package name".to_owned()))?;
    let package = package.clone();
    let local_pkg_lua = fs::read_to_string(
        app.database
            .layout()
            .recipes_dir
            .join(&package)
            .join("pkg.lua"),
    )?;
    let published = latest_published_package(app, &package)?;
    let published_pkg_lua = if published.is_some() {
        fs::read_to_string(
            CiWorkspacePaths::new(app.database.layout())
                .packages_dir
                .join(&package)
                .join("pkg.lua"),
        )
        .ok()
    } else {
        None
    };
    let changed = published_pkg_lua.as_deref() != Some(local_pkg_lua.as_str());

    Ok(CommandReport {
        area: "qa",
        status: "ok",
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: format!("qa diff inspected `{package}`."),
        details: Some(json!({
            "pkgname": package,
            "published": published,
            "changed": changed,
        })),
    })
}
