use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};
use elda_repo::search_packages;

impl AppContext {
    pub(crate) fn handle_search(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let parsed = self.parse_search_request(&request)?;
        let mut results = search_packages(&self.repo_snapshot_path(), &parsed.query, parsed.regex)?;
        if !parsed.regex {
            let query_lower = parsed.query.to_ascii_lowercase();
            let local_entries = crate::recipe_catalog::list_local_recipe_entries(
                &self.database.layout().recipes_dir,
            )?;
            for entry in local_entries {
                if entry.pkgname.to_ascii_lowercase().contains(&query_lower)
                    || entry.description.as_deref().is_some_and(|description| {
                        description.to_ascii_lowercase().contains(&query_lower)
                    })
                {
                    results.push(elda_repo::SyncedPackageRecord {
                        remote_name: "local".to_owned(),
                        remote_priority: 0,
                        pkgname: entry.pkgname,
                        epoch: 0,
                        pkgver: entry
                            .version
                            .as_deref()
                            .and_then(|version| version.split(':').nth(1))
                            .and_then(|value| value.split('-').next())
                            .unwrap_or("0.1.0")
                            .to_owned(),
                        pkgrel: 1,
                        arch: vec!["amd64".to_owned()],
                        package_kind: "normal".to_owned(),
                        variant_id: None,
                        summary: None,
                        description: entry.description,
                        homepage: entry.upstream,
                        license: entry.licenses.first().cloned(),
                        channel: None,
                        asset_url: None,
                        sha256: None,
                        size: None,
                        payload_sig: None,
                        sbom_url: None,
                        attestation_url: None,
                        source_kind: Some("local_recipe".to_owned()),
                        source_ref: None,
                        fallback_git_url: None,
                        repo_commit: None,
                        release_tag: None,
                        pkg_lua: String::new(),
                    });
                }
            }
        }

        if parsed.interactive && request.output_mode == OutputMode::Human {
            if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
                return Err(CoreError::Operator(
                    "interactive search requires a TTY; use `search` without `--interactive` or pass explicit package names to `i`".to_owned(),
                ));
            }
            if let Some(selected) = interactive_select_packages(&parsed.query, &results)? {
                if selected.is_empty() {
                    return Ok(CommandReport {
                        area: "search",
                        status: "ok",
                        exit_status: ExitStatus::Success,
                        command_path: request.command_path,
                        operands: request.operands,
                        output_mode: request.output_mode,
                        dry_run: request.dry_run,
                        summary: "no package selections entered; search completed.".to_owned(),
                        details: Some(json!({
                            "query": parsed.query,
                            "regex": parsed.regex,
                            "interactive": false,
                            "results": results,
                            "selected": [],
                        })),
                    });
                }
                let install_request = CommandRequest::new(
                    vec!["i".to_owned()],
                    selected,
                    request.output_mode,
                    request.dry_run,
                )
                .with_system_mode(request.system_mode)
                .with_offline(request.offline)
                .with_log_level(request.log_level)
                .with_accepted_rotated_keys(request.accept_rotated_keys.clone());
                return self.handle_install(install_request);
            }
        }

        Ok(CommandReport {
            area: "search",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("found {} synced package match(es).", results.len()),
            details: Some(json!({
                "query": parsed.query,
                "regex": parsed.regex,
                "interactive": parsed.interactive,
                "results": results,
            })),
        })
    }
}

fn interactive_select_packages(
    query: &str,
    results: &[elda_repo::SyncedPackageRecord],
) -> Result<Option<Vec<String>>, CoreError> {
    if results.is_empty() {
        return Ok(None);
    }
    eprintln!("search: ok");
    eprintln!("found {} synced package match(es).", results.len());
    eprintln!();
    eprintln!("Matches for `{query}`:");
    for (idx, result) in results.iter().enumerate() {
        let version = format!("{}:{}-{}", result.epoch, result.pkgver, result.pkgrel);
        eprintln!(
            "{} {}/{} {}",
            idx + 1,
            result.remote_name,
            result.pkgname,
            version
        );
        let description = result
            .description
            .as_deref()
            .or(result.summary.as_deref())
            .unwrap_or("No description available.");
        eprintln!("    {description}");
    }
    eprintln!();
    eprintln!(":: Packages to install (eg: 1 2 3, 1-3):");
    eprint!(":: ");
    io::stderr().flush().map_err(CoreError::Io)?;

    let mut selection = String::new();
    io::stdin()
        .read_line(&mut selection)
        .map_err(CoreError::Io)?;
    let selection = selection.trim();
    if selection.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let indexes = parse_selection_indexes(selection, results.len())?;
    let mut selected = BTreeSet::new();
    for index in indexes {
        selected.insert(results[index].pkgname.clone());
    }
    Ok(Some(selected.into_iter().collect()))
}

fn parse_selection_indexes(input: &str, max: usize) -> Result<Vec<usize>, CoreError> {
    let mut indexes = Vec::new();
    for token in input.split_whitespace() {
        if let Some((left, right)) = token.split_once('-') {
            let start = parse_one_based_index(left, max)?;
            let end = parse_one_based_index(right, max)?;
            let (start, end) = if start <= end {
                (start, end)
            } else {
                (end, start)
            };
            indexes.extend(start..=end);
        } else {
            indexes.push(parse_one_based_index(token, max)?);
        }
    }
    Ok(indexes)
}

fn parse_one_based_index(token: &str, max: usize) -> Result<usize, CoreError> {
    let value = token.parse::<usize>().map_err(|_| {
        CoreError::Operator(format!(
            "invalid selection `{token}`; expected one-based indexes or ranges"
        ))
    })?;
    if value == 0 || value > max {
        return Err(CoreError::Operator(format!(
            "selection `{token}` is out of range; expected 1..={max}"
        )));
    }
    Ok(value - 1)
}
