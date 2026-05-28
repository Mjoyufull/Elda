use std::io::Write;

use crate::CommandRequest;
use crate::error::CoreError;
use crate::run_log::is_mutating_command;

pub(crate) fn install_handler_if_needed(request: &CommandRequest) -> Result<(), CoreError> {
    if cfg!(test) {
        return Ok(());
    }
    if request.output_mode != crate::OutputMode::Human {
        return Ok(());
    }
    if !is_mutating_command(&request.command_path) {
        return Ok(());
    }

    ctrlc::set_handler(handle_interrupt).map_err(|error| {
        CoreError::Operator(format!("could not install interrupt handler: {error}"))
    })
}

fn handle_interrupt() {
    restore_terminal();
    let _ = writeln!(std::io::stderr(), "\n:: transaction interrupted");
    let _ = writeln!(
        std::io::stderr(),
        ":: partial build trees may remain under Elda tmp; run `elda recover` if a file mutation was in progress"
    );
    std::process::exit(130);
}

fn restore_terminal() {
    let _ = write!(std::io::stderr(), "\x1b[?25h");
}
