use std::io::{self, IsTerminal, Write};

use crate::app::{ParsedInstallRequest, PlannedInstallAction};
use crate::config::LinkOptionMode;
use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};

pub(crate) fn apply_interactive_source_option_selection(
    request: &CommandRequest,
    parsed: &ParsedInstallRequest,
    actions: &[PlannedInstallAction],
    link_option_mode: LinkOptionMode,
) -> Result<Option<ParsedInstallRequest>, CoreError> {
    if link_option_mode != LinkOptionMode::ListOptions
        || request.dry_run
        || request.output_mode != OutputMode::Human
        || parsed.source_option.is_some()
    {
        return Ok(None);
    }

    let Some(action) = selectable_action(actions) else {
        return Ok(None);
    };
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(None);
    }

    eprintln!("Source options for `{}`:", action.target);
    for option in &action.resolved.source_options {
        let selected = if option.selected { "*" } else { " " };
        let checksum = if option.checksum_available {
            " checksum=yes"
        } else {
            " checksum=no"
        };
        eprintln!(
            "{selected} {}. {} [{} / {} / {}]{}",
            option.index,
            option.strategy,
            option.lane,
            option.source_kind,
            option.confidence,
            checksum
        );
        eprintln!("    {}", option.summary);
        if let Some(tag) = &option.tag {
            eprintln!("    tag: {tag}");
        }
        if let Some(asset) = &option.asset {
            eprintln!("    asset: {asset}");
        }
        if let Some(compatibility) = &option.compatibility {
            eprintln!("    compatibility: {compatibility}");
        }
    }
    eprintln!();
    eprintln!(":: Source option (blank keeps selected default):");
    eprint!(":: ");
    io::stderr().flush().map_err(CoreError::Io)?;

    let mut selection = String::new();
    io::stdin()
        .read_line(&mut selection)
        .map_err(CoreError::Io)?;
    let selection = selection.trim();
    if selection.is_empty() {
        return Ok(None);
    }
    let index = selection.parse::<usize>().map_err(|_| {
        CoreError::Operator("source option selection must be one listed number".to_owned())
    })?;
    if !action
        .resolved
        .source_options
        .iter()
        .any(|option| option.index == index)
    {
        return Err(CoreError::Operator(format!(
            "source option `{index}` is not available for `{}`",
            action.target
        )));
    }

    let mut selected = parsed.clone();
    selected.source_option = Some(index);
    Ok(Some(selected))
}

fn selectable_action(actions: &[PlannedInstallAction]) -> Option<&PlannedInstallAction> {
    actions
        .iter()
        .find(|action| action.resolved.source_options.len() > 1)
}
