//! Shared interactive confirmation prompts for mutating operator commands.

use std::io::{self, IsTerminal, Write};

use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfirmResponse {
    Accept,
    Decline,
    Edit,
    Invalid,
}

#[must_use]
pub(crate) fn interactive_session(request: &CommandRequest) -> bool {
    request.output_mode == OutputMode::Human
        && !request.dry_run
        && io::stdout().is_terminal()
        && io::stdin().is_terminal()
}

pub(crate) fn parse_yne_response(input: &str) -> ConfirmResponse {
    match input.trim().to_ascii_lowercase().as_str() {
        "" | "y" | "yes" => ConfirmResponse::Accept,
        "n" | "no" => ConfirmResponse::Decline,
        "e" | "edit" => ConfirmResponse::Edit,
        _ => ConfirmResponse::Invalid,
    }
}

pub(crate) fn prompt_yne(prompt: &str) -> Result<ConfirmResponse, CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    loop {
        write!(stdout, "{prompt} [Y/n/e] ")?;
        stdout.flush()?;
        let mut answer = String::new();
        stdin.read_line(&mut answer)?;
        match parse_yne_response(&answer) {
            ConfirmResponse::Invalid => {
                writeln!(stdout, "Enter `Y`, `n`, or `e`.")?;
            }
            response => return Ok(response),
        }
    }
}

/// Read stdin after a frame footer already printed `[Y/n/e]`; do not emit a second prompt line.
pub(crate) fn read_yne_after_frame() -> Result<ConfirmResponse, CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    let mut answer = String::new();
    stdin.read_line(&mut answer)?;
    let response = parse_yne_response(&answer);
    if response == ConfirmResponse::Invalid {
        writeln!(stdout, "invalid response: use Y, n, or e")?;
    }
    Ok(response)
}

pub(crate) fn prompt_yn(prompt: &str, default_yes: bool) -> Result<bool, CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };

    loop {
        write!(stdout, "{prompt} {hint} ")?;
        stdout.flush()?;
        let mut answer = String::new();
        stdin.read_line(&mut answer)?;
        let normalized = answer.trim().to_ascii_lowercase();
        let decision = match normalized.as_str() {
            "" => default_yes,
            "y" | "yes" => true,
            "n" | "no" => false,
            _ => {
                writeln!(stdout, "Enter `Y` or `n`.")?;
                continue;
            }
        };
        return Ok(decision);
    }
}

/// Require an interactive terminal for destructive mutations when not in dry-run mode.
pub(crate) fn require_interactive_confirmation(
    request: &CommandRequest,
    action: &str,
) -> Result<(), CoreError> {
    if request.dry_run || interactive_session(request) {
        return Ok(());
    }
    Err(CoreError::Operator(format!(
        "{action} requires an interactive terminal; run with `--dry-run` first or use a TTY"
    )))
}

/// Human interactive confirmation before a mutating action. JSON and dry-run skip the prompt.
pub(crate) fn confirm_mutation(request: &CommandRequest, prompt: &str) -> Result<(), CoreError> {
    if request.dry_run || request.output_mode != OutputMode::Human {
        return Ok(());
    }
    if !interactive_session(request) {
        if cfg!(test) {
            return Ok(());
        }
        return require_interactive_confirmation(request, prompt);
    }
    match prompt_yne(prompt)? {
        ConfirmResponse::Accept => Ok(()),
        ConfirmResponse::Decline => Err(CoreError::Operator(
            "operation cancelled by operator".to_owned(),
        )),
        ConfirmResponse::Edit => Err(CoreError::Operator(
            "operation cancelled; edit config or recipe metadata, then retry".to_owned(),
        )),
        ConfirmResponse::Invalid => Err(CoreError::Operator(
            "operation cancelled: confirmation was not accepted".to_owned(),
        )),
    }
}
