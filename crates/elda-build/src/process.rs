use std::process::Command;

use crate::error::BuildError;

pub fn run_command(
    program: &'static str,
    mut command: Command,
    context: &str,
) -> Result<(), BuildError> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Err(BuildError::CommandFailed {
        program,
        context: context.to_owned(),
        stderr: if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "command exited with a non-zero status".to_owned()
        },
    })
}
