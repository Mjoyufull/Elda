use std::io::{self, Write};

use crate::app::{AppContext, ParsedInstallRequest};
use crate::app_confirm::interactive_session;
use crate::app_install::set_git_ref;
use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};
use elda_recipe::{GitRefKind, GitRefRequest, is_git_like_target};

use crate::app::ParsedUpgradeRequest;

pub(crate) fn apply_pick_tag_selection(
    app: &AppContext,
    request: &CommandRequest,
    parsed: &mut ParsedInstallRequest,
) -> Result<(), CoreError> {
    if !operands_contain_flag(&request.operands, "--pick-tag")
        && !operands_contain_flag(&request.operands, "--pick")
    {
        return Ok(());
    }
    if request.dry_run || request.output_mode != OutputMode::Human || !interactive_session(request)
    {
        return Err(CoreError::Operator(
            "`--pick-tag` requires an interactive human TTY; use `--to-tag <name>` in non-interactive mode"
                .to_owned(),
        ));
    }
    if parsed.git_ref.is_some() {
        return Ok(());
    }

    let targets: Vec<String> = parsed.targets.clone();
    for target in targets {
        if !is_git_like_target(&target) {
            continue;
        }
        let tag = prompt_tag_for_target(app, &target)?;
        set_git_ref(&mut parsed.git_ref, GitRefKind::Tag, &tag)?;
        parsed
            .git_source_refs
            .insert(target.clone(), target.clone());
        parsed.git_ref_overrides.insert(
            target.clone(),
            GitRefRequest {
                kind: GitRefKind::Tag,
                value: tag,
            },
        );
    }
    Ok(())
}

fn prompt_tag_for_target(app: &AppContext, target: &str) -> Result<String, CoreError> {
    let tag_options = elda_git::GitTagOptions {
        max_tags: app.config.git.max_tags,
        include_prereleases: app.config.git.include_prereleases,
        strip_v_prefix: app.config.git.strip_v_prefix,
        allow_date_versions: app.config.git.allow_date_versions,
        allow_raw_versions: app.config.git.tag_policy != "semver",
    };
    let report = elda_git::list_remote_tags_with_options(target, tag_options)
        .map_err(|error| CoreError::Operator(error.to_string()))?;
    if report.tags.is_empty() {
        return Err(CoreError::Operator(format!("no tags found for `{target}`")));
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    writeln!(stdout)?;
    writeln!(stdout, "┌─ Git Tag Selection")?;
    writeln!(stdout, "├─ Target")?;
    writeln!(stdout, "│  {target}")?;
    writeln!(stdout, "│")?;
    writeln!(stdout, "├─ Available tags")?;
    for (index, tag) in report.tags.iter().take(20).enumerate() {
        writeln!(
            stdout,
            "│  {}) {} ({})",
            index + 1,
            tag.tag,
            format!("{:?}", tag.version_confidence).to_ascii_lowercase()
        )?;
    }
    if report.tags.len() > 20 {
        writeln!(
            stdout,
            "│  ... {} more tag(s) omitted",
            report.tags.len() - 20
        )?;
    }
    writeln!(stdout, "│")?;
    let upper = report.tags.len().min(20);
    writeln!(stdout, "└─ Select tag [1-{upper}, abort]: ")?;
    stdout.flush().map_err(CoreError::Io)?;

    loop {
        let mut selection = String::new();
        stdin.read_line(&mut selection).map_err(CoreError::Io)?;
        let selection = selection.trim();
        if selection.eq_ignore_ascii_case("abort") {
            return Err(CoreError::Operator(
                "operation cancelled: git tag was not selected".to_owned(),
            ));
        }
        let Ok(index) = selection.parse::<usize>() else {
            writeln!(stdout, ":: Enter a number from the list, or `abort`.")?;
            stdout.flush().map_err(CoreError::Io)?;
            continue;
        };
        if !(1..=upper).contains(&index) {
            writeln!(stdout, ":: Selection must be between 1 and {upper}.")?;
            stdout.flush().map_err(CoreError::Io)?;
            continue;
        }
        return Ok(report.tags[index - 1].tag.clone());
    }
}

pub(crate) fn apply_upgrade_pick_tag_selection(
    app: &AppContext,
    request: &CommandRequest,
    parsed: &mut ParsedUpgradeRequest,
) -> Result<(), CoreError> {
    if !operands_contain_flag(&request.operands, "--pick-tag")
        && !operands_contain_flag(&request.operands, "--pick")
    {
        return Ok(());
    }
    if request.dry_run || request.output_mode != OutputMode::Human || !interactive_session(request)
    {
        return Err(CoreError::Operator(
            "`--pick-tag` requires an interactive human TTY; use `--to-tag <name>` in non-interactive mode"
                .to_owned(),
        ));
    }
    if parsed.git_ref.is_some() {
        return Ok(());
    }
    let target = parsed.targets.first().cloned().ok_or_else(|| {
        CoreError::Operator("upgrade --pick-tag requires one package name".to_owned())
    })?;
    let installed = app.ensure_installed(&target)?;
    let source_ref = installed.source_ref.clone().ok_or_else(|| {
        CoreError::Operator(format!(
            "`{target}` does not have a persisted git source ref"
        ))
    })?;
    let tag = prompt_tag_for_target(app, &source_ref)?;
    crate::app_install::set_git_ref(&mut parsed.git_ref, GitRefKind::Tag, &tag)?;
    Ok(())
}

fn operands_contain_flag(operands: &[String], flag: &str) -> bool {
    operands.iter().any(|operand| operand == flag)
}
