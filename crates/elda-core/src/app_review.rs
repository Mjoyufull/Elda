use std::path::Path;

use serde_json::json;

use crate::app::AppContext;
use crate::app_review_memory::{
    ReviewStamp, forget_review_stamp, list_review_history, list_review_stamps, load_review_stamp,
    review_is_unchanged,
};
use crate::editor::{open_path_in_pager, open_paths_in_diff_pager, open_text_in_pager};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_review_list(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let stamps = list_review_stamps(&self.database.layout().data_dir)?;

        Ok(CommandReport {
            area: "review",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} review stamp(s).", stamps.len()),
            details: Some(json!({ "stamps": stamps })),
        })
    }

    pub(crate) fn handle_review_info(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package = review_package_operand(&request)?.to_owned();
        let data_dir = &self.database.layout().data_dir;
        let stamps = stamps_for_package(data_dir, &package)?;
        let history = list_review_history(data_dir, Some(&package))?;

        Ok(CommandReport {
            area: "review",
            status: if stamps.is_empty() { "missing" } else { "ok" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if stamps.is_empty() {
                format!("no review stamp is recorded for `{package}`.")
            } else {
                format!("reported {} review stamp(s) for `{package}`.", stamps.len())
            },
            details: Some(json!({
                "package": package,
                "stamps": stamps,
                "history": history,
            })),
        })
    }

    pub(crate) fn handle_review_forget(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package = review_package_operand(&request)?.to_owned();
        let review_kind = review_kind_operand(&request)?.to_owned();
        let data_dir = &self.database.layout().data_dir;
        let removed = forget_review_stamp(data_dir, &package, &review_kind)?;

        Ok(CommandReport {
            area: "review",
            status: if removed { "ok" } else { "missing" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if removed {
                format!("forgot review stamp for `{package}` ({review_kind}).")
            } else {
                format!("no `{review_kind}` review stamp exists for `{package}`.")
            },
            details: Some(json!({
                "package": package,
                "review_kind": review_kind,
                "removed": removed,
            })),
        })
    }

    pub(crate) fn handle_review_diff(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package = review_package_operand(&request)?.to_owned();
        let review_kind = review_kind_operand(&request)?.to_owned();
        let data_dir = &self.database.layout().data_dir;
        let stamp = load_review_stamp(data_dir, &package, &review_kind)?;
        let recipes_dir = &self.database.layout().recipes_dir;
        let recipe_path = review_recipe_path(
            &request,
            recipes_dir,
            &package,
            &review_kind,
            stamp.as_ref(),
        )?;
        let unchanged = review_is_unchanged(data_dir, &package, &review_kind, &recipe_path)?;
        let opened_pager = if request.output_mode == crate::OutputMode::Human
            && !request.dry_run
            && recipe_path.is_file()
        {
            let label = format!("{package} ({review_kind})");
            let previous = stamp
                .as_ref()
                .map(|entry| Path::new(&entry.recipe_path))
                .filter(|path| path.is_file() && *path != recipe_path);
            Some(if let Some(previous) = previous {
                open_paths_in_diff_pager(previous, &recipe_path, &label)?
            } else {
                open_path_in_pager(
                    &recipe_path,
                    &format!("{label} — {}", recipe_path.display()),
                )?
            })
        } else {
            None
        };

        Ok(CommandReport {
            area: "review",
            status: if stamp.is_some() {
                if unchanged { "current" } else { "changed" }
            } else {
                "new"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: review_diff_summary(&package, &review_kind, stamp.as_ref(), unchanged),
            details: Some(json!({
                "package": package,
                "review_kind": review_kind,
                "recipe_path": recipe_path.display().to_string(),
                "stamp": stamp,
                "unchanged": unchanged,
                "opened_pager": opened_pager,
            })),
        })
    }
}

pub(crate) fn preview_recipe_for_review(recipe_path: &Path, title: &str) -> Result<(), CoreError> {
    if !recipe_path.is_file() {
        return Ok(());
    }
    let content = std::fs::read_to_string(recipe_path)?;
    let header = format!(
        "{title}\npath: {}\n\nPress `q` to return to Elda.\n\n",
        recipe_path.display()
    );
    open_text_in_pager(&format!("{header}{content}"), title)
}

fn review_package_operand(request: &CommandRequest) -> Result<&str, CoreError> {
    request
        .operands
        .first()
        .map(String::as_str)
        .ok_or_else(|| CoreError::Operator("review command requires a package name".to_owned()))
}

fn review_kind_operand(request: &CommandRequest) -> Result<&str, CoreError> {
    for index in 0..request.operands.len() {
        if request.operands[index] == "--kind"
            && let Some(kind) = request.operands.get(index + 1)
        {
            return Ok(kind.as_str());
        }
    }
    Ok("interbuild")
}

fn stamps_for_package(data_dir: &Path, package: &str) -> Result<Vec<ReviewStamp>, CoreError> {
    Ok(list_review_stamps(data_dir)?
        .into_iter()
        .filter(|stamp| stamp.package == package)
        .collect())
}

fn review_recipe_path(
    request: &CommandRequest,
    recipes_dir: &Path,
    package: &str,
    review_kind: &str,
    stamp: Option<&ReviewStamp>,
) -> Result<std::path::PathBuf, CoreError> {
    for index in 0..request.operands.len() {
        if request.operands[index] == "--recipe"
            && let Some(path) = request.operands.get(index + 1)
        {
            return Ok(std::path::PathBuf::from(path));
        }
    }

    if let Some(stamp) = stamp {
        return Ok(std::path::PathBuf::from(&stamp.recipe_path));
    }

    let _ = review_kind;
    let default = recipes_dir.join(package).join("pkg.lua");
    if default.is_file() {
        return Ok(default);
    }

    Err(CoreError::Operator(format!(
        "review diff for `{package}` requires an existing stamp or `--recipe <path>`"
    )))
}

fn review_diff_summary(
    package: &str,
    review_kind: &str,
    stamp: Option<&ReviewStamp>,
    unchanged: bool,
) -> String {
    match stamp {
        Some(_stamp) if unchanged => {
            format!("review memory for `{package}` ({review_kind}) is current.")
        }
        Some(_) => format!(
            "reviewed recipe for `{package}` ({review_kind}) changed since the last acceptance."
        ),
        None => format!("no review stamp is recorded for `{package}` ({review_kind})."),
    }
}
