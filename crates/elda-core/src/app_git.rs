use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::app::AppContext;
use crate::{CommandReport, CommandRequest, CoreError, ExitStatus};

impl AppContext {
    pub(crate) fn handle_git_tags(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.git_tag_report(request, "git tags")
    }

    pub(crate) fn handle_git_versions(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.git_tag_report(request, "versions")
    }

    pub(crate) fn handle_git_releases(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let target = request.operands.first().ok_or_else(|| {
            CoreError::Operator(
                "git releases requires a forge repository URL or owner/repo".to_owned(),
            )
        })?;
        let release_args = parse_release_args(&request.operands)?;
        let mut report = elda_git::inspect_releases(target, release_args.max_releases)
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        if let Some(tag) = &release_args.tag {
            report.releases.retain(|release| release.tag == *tag);
        }
        let release_count = report.releases.len();
        let repo = report.repo.clone();

        Ok(CommandReport {
            area: "git",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("found {release_count} release candidate(s) for `{repo}`."),
            details: Some(json!({
                "git_releases": report,
                "max_releases": release_args.max_releases,
                "tag": release_args.tag,
            })),
        })
    }

    fn git_tag_report(
        &self,
        request: CommandRequest,
        command_label: &str,
    ) -> Result<CommandReport, CoreError> {
        let target = request.operands.first().ok_or_else(|| {
            CoreError::Operator(format!(
                "{command_label} requires a repository URL or local path"
            ))
        })?;
        let tag_args = parse_tag_args(&request.operands, command_label, self.config.git.max_tags)?;
        let max_tags = tag_args.max_tags;
        let tag_options = elda_git::GitTagOptions {
            max_tags,
            include_prereleases: self.config.git.include_prereleases,
            strip_v_prefix: self.config.git.strip_v_prefix,
            allow_date_versions: self.config.git.allow_date_versions,
            allow_raw_versions: self.config.git.tag_policy != "semver",
        };
        let mut report = elda_git::list_remote_tags_with_options(target, tag_options.clone())
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let release_join = if tag_args.with_releases {
            Some(join_releases_to_tags(target, &mut report)?)
        } else {
            None
        };
        let tag_count = report.tags.len();
        let target = report.target.clone();

        Ok(CommandReport {
            area: "git",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("found {tag_count} version candidate(s) for `{target}`."),
            details: Some(json!({
                "git_tags": report,
                "max_tags": max_tags,
                "tag_options": tag_options,
                "release_join": release_join,
            })),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TagArgs {
    max_tags: usize,
    with_releases: bool,
}

fn parse_tag_args(
    operands: &[String],
    command_label: &str,
    default_max_tags: usize,
) -> Result<TagArgs, CoreError> {
    let mut max_tags = default_max_tags;
    let mut with_releases = false;
    let mut index = 1;
    while index < operands.len() {
        match operands[index].as_str() {
            "--max-tags" => {
                let Some(value) = operands.get(index + 1) else {
                    return Err(CoreError::Operator(
                        format!("{command_label} --max-tags requires a value").to_owned(),
                    ));
                };
                max_tags = value.parse::<usize>().map_err(|_| {
                    CoreError::Operator(format!(
                        "{command_label} --max-tags requires a non-negative integer, got `{value}`"
                    ))
                })?;
                index += 2;
            }
            "--with-releases" => {
                with_releases = true;
                index += 1;
            }
            value => {
                return Err(CoreError::Operator(format!(
                    "unsupported {command_label} operand `{value}`"
                )));
            }
        }
    }
    Ok(TagArgs {
        max_tags,
        with_releases,
    })
}

fn join_releases_to_tags(
    target: &str,
    report: &mut elda_git::GitTagReport,
) -> Result<Value, CoreError> {
    let release_report = elda_git::inspect_releases(target, report.tags.len())
        .map_err(|error| CoreError::Operator(error.to_string()))?;
    let releases_by_tag = release_report
        .releases
        .iter()
        .map(|release| (release.tag.clone(), release))
        .collect::<BTreeMap<_, _>>();
    let mut joined = Vec::with_capacity(report.tags.len());
    for tag in &report.tags {
        let release = releases_by_tag.get(&tag.tag);
        joined.push(json!({
            "tag": tag.tag,
            "has_release": release.is_some(),
            "recommended_asset": release.and_then(|release| release.recommended_asset.clone()),
            "asset_count": release.map_or(0, |release| release.assets.len()),
        }));
    }

    Ok(json!({
        "source": release_report.source,
        "repo": release_report.repo,
        "joined_tags": joined,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseArgs {
    max_releases: usize,
    tag: Option<String>,
}

fn parse_release_args(operands: &[String]) -> Result<ReleaseArgs, CoreError> {
    let mut max_releases = 10;
    let mut tag = None;
    let mut index = 1;
    while index < operands.len() {
        match operands[index].as_str() {
            "--max-releases" => {
                let Some(value) = operands.get(index + 1) else {
                    return Err(CoreError::Operator(
                        "git releases --max-releases requires a value".to_owned(),
                    ));
                };
                max_releases = value.parse::<usize>().map_err(|_| {
                    CoreError::Operator(format!(
                        "git releases --max-releases requires a non-negative integer, got `{value}`"
                    ))
                })?;
                index += 2;
            }
            "--tag" => {
                let Some(value) = operands.get(index + 1) else {
                    return Err(CoreError::Operator(
                        "git releases --tag requires a value".to_owned(),
                    ));
                };
                tag = Some(value.clone());
                index += 2;
            }
            value => {
                return Err(CoreError::Operator(format!(
                    "unsupported git releases operand `{value}`"
                )));
            }
        }
    }
    Ok(ReleaseArgs { max_releases, tag })
}

#[cfg(test)]
mod tests {
    use super::{ReleaseArgs, parse_release_args};

    #[test]
    fn parse_release_args_accepts_tag_filter() {
        let args = parse_release_args(&[
            "Mjoyufull/fsel".to_owned(),
            "--max-releases".to_owned(),
            "3".to_owned(),
            "--tag".to_owned(),
            "v1.2.3".to_owned(),
        ])
        .expect("release args should parse");

        assert_eq!(
            args,
            ReleaseArgs {
                max_releases: 3,
                tag: Some("v1.2.3".to_owned()),
            }
        );
    }
}
