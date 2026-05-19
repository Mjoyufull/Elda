use serde_json::json;

use crate::{CommandReport, CommandRequest, ExitStatus};

use super::adapters::ForeignPackage;

pub(crate) fn adopt_report(
    request: CommandRequest,
    package: &ForeignPackage,
    status: &'static str,
    committed: bool,
) -> CommandReport {
    CommandReport {
        area: "migration",
        status,
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: if committed {
            format!(
                "adopted `{}` from `{}` without modifying live files.",
                package.name, package.source_pm
            )
        } else {
            format!(
                "planned adoption of `{}` from `{}`.",
                package.name, package.source_pm
            )
        },
        details: Some(json!({
            "adoption": package_json(package),
            "committed": committed,
        })),
    }
}

pub(crate) fn migration_from_report(
    request: CommandRequest,
    source_pm: &str,
    packages: &[ForeignPackage],
) -> CommandReport {
    CommandReport {
        area: "migration",
        status: if request.dry_run { "planned" } else { "ok" },
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: if request.dry_run {
            format!(
                "planned migration of {} package(s) from `{source_pm}`.",
                packages.len()
            )
        } else {
            format!(
                "migrated {} package(s) from `{source_pm}` as adopted state.",
                packages.len()
            )
        },
        details: Some(json!({
            "migration": {
                "source_pm": source_pm,
                "package_count": packages.len(),
                "packages": packages.iter().map(package_json).collect::<Vec<_>>(),
            }
        })),
    }
}

pub(crate) fn migration_lock_blocked_report(request: CommandRequest) -> CommandReport {
    CommandReport {
        area: "migration",
        status: "blocked",
        exit_status: ExitStatus::OperatorFailure,
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: "migration lock/unlock needs live package-manager binary takeover policy."
            .to_owned(),
        details: Some(json!({
            "kind": "external-policy",
            "blocked": "renaming a foreign package-manager binary is not safe in this disposable-root migration slice",
            "next_action": "use `elda adopt --from <pm> <pkg>` or `elda mg from <pm>` for database-backed adoption; freeze live lock policy before binary takeover",
            "command_path": request.command_path,
            "operands": request.operands,
            "dry_run": request.dry_run,
        })),
    }
}

fn package_json(package: &ForeignPackage) -> serde_json::Value {
    json!({
        "source_pm": package.source_pm,
        "pkgname": package.name,
        "version": {
            "epoch": package.version.epoch,
            "pkgver": package.version.pkgver,
            "pkgrel": package.version.pkgrel,
            "raw": package.version.raw,
        },
        "arch": package.arch,
        "source_repo": package.source_repo,
        "source_channel": package.source_channel,
        "files": package.files,
        "dependencies": package.dependencies,
    })
}
