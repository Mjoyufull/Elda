use std::io::{self, Write};

use crate::CommandRequest;
use crate::app::DependencyCandidate;
use crate::app_confirm::interactive_session;
use crate::error::CoreError;

pub(crate) fn parse_provider_assignment(value: &str) -> Result<(String, String), CoreError> {
    let (virtual_name, provider) = value.split_once('=').ok_or_else(|| {
        CoreError::Operator(
            "provider assignment must be `virtual=package` (for example `phonon-backend=phonon-qt5`)"
                .to_owned(),
        )
    })?;
    let virtual_name = virtual_name.trim();
    let provider = provider.trim();
    if virtual_name.is_empty() || provider.is_empty() {
        return Err(CoreError::Operator(
            "provider assignment must include both a virtual name and a provider package"
                .to_owned(),
        ));
    }
    Ok((virtual_name.to_owned(), provider.to_owned()))
}

pub(crate) fn prompt_virtual_provider_selection(
    virtual_name: &str,
    candidates: &[DependencyCandidate],
) -> Result<String, CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    writeln!(stdout)?;
    writeln!(stdout, "┌─ Ambiguous Virtual Provider Resolution")?;
    writeln!(stdout, "├─ Required virtual")?;
    writeln!(stdout, "│  requires: {virtual_name}")?;
    writeln!(stdout, "│")?;
    writeln!(stdout, "├─ Available Providers")?;
    for (index, candidate) in candidates.iter().enumerate() {
        writeln!(
            stdout,
            "│  {}) {} ({})",
            index + 1,
            candidate.target,
            provider_origin_label(candidate)
        )?;
    }
    writeln!(stdout, "│")?;
    let upper = candidates.len();
    writeln!(
        stdout,
        "└─ Select provider to fulfill dependency [1-{upper}, abort]: "
    )?;
    stdout.flush().map_err(CoreError::Io)?;

    loop {
        let mut selection = String::new();
        stdin.read_line(&mut selection).map_err(CoreError::Io)?;
        let selection = selection.trim();
        if selection.eq_ignore_ascii_case("abort")
            || selection.eq_ignore_ascii_case("a")
            || selection.eq_ignore_ascii_case("q")
        {
            return Err(CoreError::Operator(
                "install cancelled: virtual provider was not selected".to_owned(),
            ));
        }
        if selection.is_empty() {
            writeln!(stdout, ":: Enter a number from the list, or `abort`.")?;
            stdout.flush().map_err(CoreError::Io)?;
            continue;
        }
        let Ok(index) = selection.parse::<usize>() else {
            writeln!(stdout, ":: Selection must be a number from the list.")?;
            stdout.flush().map_err(CoreError::Io)?;
            continue;
        };
        if !(1..=candidates.len()).contains(&index) {
            writeln!(stdout, ":: Selection must be between 1 and {upper}.")?;
            stdout.flush().map_err(CoreError::Io)?;
            continue;
        }
        return Ok(candidates[index - 1].target.clone());
    }
}

pub(crate) fn provider_origin_label(candidate: &DependencyCandidate) -> &'static str {
    match candidate.source_priority {
        None => "native",
        Some(_) => "remote",
    }
}

pub(crate) fn install_allows_provider_prompt(command: Option<&CommandRequest>) -> bool {
    let Some(request) = command else {
        return false;
    };
    interactive_session(request)
}

pub(crate) fn reorder_provider_candidates(
    ranked: &mut Vec<DependencyCandidate>,
    chosen: &str,
) -> Result<(), CoreError> {
    let Some(position) = ranked
        .iter()
        .position(|candidate| candidate.target == chosen)
    else {
        return Err(CoreError::Operator(format!(
            "provider `{chosen}` is not one of the available candidates"
        )));
    };
    if position == 0 {
        return Ok(());
    }
    let selected = ranked.remove(position);
    ranked.insert(0, selected);
    Ok(())
}
