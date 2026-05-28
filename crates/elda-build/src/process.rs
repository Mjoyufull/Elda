use std::process::{Command, Stdio};

use crate::error::BuildError;

use crate::BuildLineHook;

pub(crate) fn emit_build_line(line_hook: &Option<BuildLineHook>, line: impl AsRef<str>) {
    if let Some(hook) = line_hook {
        hook(line.as_ref());
    }
}

pub fn run_command_inherited(
    program: &'static str,
    mut command: Command,
    context: &str,
) -> Result<(), BuildError> {
    command.stdin(Stdio::null());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    let status = command.status()?;
    if status.success() {
        return Ok(());
    }

    Err(BuildError::CommandFailed {
        program,
        context: context.to_owned(),
        stderr: format!("{program} exited with status {status}"),
    })
}

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
        stderr: command_failure_message(program, &stderr, &stdout),
    })
}

pub fn command_failure_message(program: &str, stderr: &str, stdout: &str) -> String {
    let output = if !stderr.is_empty() { stderr } else { stdout };
    if output.is_empty() {
        return "command exited with a non-zero status".to_owned();
    }
    if program == "cargo" {
        return cargo_failure_summary(output);
    }
    truncate_output(output, 12)
}

fn cargo_failure_summary(output: &str) -> String {
    let lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let Some(error_index) = lines.iter().rposition(|line| line.starts_with("error:")) else {
        return truncate_output(output, 12);
    };

    let mut selected = vec![lines[error_index].to_owned()];
    for line in lines.iter().skip(error_index + 1) {
        if line.starts_with("help:") || line.starts_with("note:") {
            selected.push((*line).to_owned());
        }
        if selected.len() >= 4 {
            break;
        }
    }
    selected.join("; ")
}

fn truncate_output(output: &str, max_lines: usize) -> String {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() <= max_lines {
        return lines.join("\n");
    }

    lines.truncate(max_lines);
    format!("{}\n... output truncated", lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_failure_summary_keeps_error_and_hint() {
        let output = "Updating crates.io index\n Downloading crates ...\nerror: no bin target named `CuTTY` in default-run packages\n\nhelp: a target with a similar name exists: `cutty`\n";

        assert_eq!(
            command_failure_message("cargo", output, ""),
            "error: no bin target named `CuTTY` in default-run packages; help: a target with a similar name exists: `cutty`"
        );
    }
}
