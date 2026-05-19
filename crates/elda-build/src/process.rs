use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

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

pub fn run_command_streamed(
    program: &'static str,
    mut command: Command,
    context: &str,
    line_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<(), BuildError> {
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let output = Arc::new(Mutex::new(String::new()));
    let stdout_thread = child.stdout.take().map(|stdout| {
        let output = Arc::clone(&output);
        let hook = line_hook.clone();
        thread::spawn(move || read_stream(program, stdout, output, hook))
    });
    let stderr_thread = child.stderr.take().map(|stderr| {
        let output = Arc::clone(&output);
        let hook = line_hook.clone();
        thread::spawn(move || read_stream(program, stderr, output, hook))
    });

    let status = child.wait()?;
    if let Some(handle) = stdout_thread {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_thread {
        let _ = handle.join();
    }
    if status.success() {
        return Ok(());
    }

    let captured = output.lock().map(|guard| guard.clone()).unwrap_or_default();
    Err(BuildError::CommandFailed {
        program,
        context: context.to_owned(),
        stderr: command_failure_message(program, &captured, ""),
    })
}

fn read_stream<R>(
    program: &str,
    reader: R,
    output: Arc<Mutex<String>>,
    line_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) where
    R: std::io::Read,
{
    for line in BufReader::new(reader).lines().map_while(Result::ok) {
        if let Ok(mut captured) = output.lock() {
            captured.push_str(&line);
            captured.push('\n');
        }
        if let Some(rendered) = streamed_line(program, &line) {
            if let Some(hook) = &line_hook {
                hook(&rendered);
            } else {
                eprintln!("│  {rendered}");
            }
        }
    }
}

fn streamed_line(program: &str, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if program == "make" {
        if trimmed.starts_with("make[")
            || trimmed.contains("Entering directory")
            || trimmed.starts_with("Leaving directory")
            || trimmed.starts_with("Error")
            || trimmed.starts_with("error:")
        {
            return Some(format!("[Make] {trimmed}"));
        }
        return None;
    }

    if program == "cmake" || program == "ctest" {
        if trimmed.starts_with("[") && trimmed.contains('%')
            || trimmed.starts_with("Building CXX")
            || trimmed.starts_with("Linking CXX")
            || trimmed.starts_with("Scanning ")
            || trimmed.starts_with("FAILED:")
            || trimmed.starts_with("ninja:")
            || trimmed.starts_with("-- ")
            || trimmed.contains("Configuring done")
            || trimmed.contains("Generating done")
            || trimmed.starts_with("Test project ")
            || trimmed.starts_with("    Start ")
        {
            return Some(format!("[CMake] {trimmed}"));
        }
        return None;
    }

    if program == "meson" {
        if trimmed.starts_with("[") && trimmed.contains('/')
            || trimmed.starts_with("ninja:")
            || trimmed.starts_with("Linking ")
            || trimmed.starts_with("Compiling ")
            || trimmed.starts_with("FAILED:")
        {
            return Some(format!("[Ninja] {trimmed}"));
        }
        if trimmed.starts_with("meson ") || trimmed.starts_with("The Meson") {
            return Some(format!("[Meson] {trimmed}"));
        }
        return None;
    }

    if program != "cargo" {
        return Some(trimmed.to_owned());
    }

    if trimmed.starts_with("Downloaded ") {
        return None;
    }
    if trimmed == "Downloading crates ..." || trimmed.starts_with("Updating ") {
        return Some(format!("[Cargo] {trimmed}"));
    }
    if trimmed.starts_with("Compiling ")
        || trimmed.starts_with("Finished ")
        || trimmed.starts_with("Checking ")
        || trimmed.starts_with("Building ")
        || trimmed.starts_with("error:")
        || trimmed.starts_with("help:")
        || trimmed.starts_with("warning:")
    {
        return Some(format!("[Cargo] {trimmed}"));
    }
    None
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

    #[test]
    fn cargo_stream_suppresses_downloaded_spam() {
        assert_eq!(streamed_line("cargo", "Downloaded libc v0.2"), None);
        assert_eq!(
            streamed_line("cargo", "Compiling libc v0.2").as_deref(),
            Some("[Cargo] Compiling libc v0.2")
        );
    }
}
