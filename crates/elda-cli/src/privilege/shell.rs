use std::ffi::{OsStr, OsString};
use std::path::Path;

use anyhow::anyhow;

pub(super) fn render_shell_command(
    current_exe: &Path,
    forwarded_args: &[OsString],
) -> anyhow::Result<String> {
    let mut parts = vec![shell_quote(current_exe.as_os_str())?];
    for argument in forwarded_args {
        parts.push(shell_quote(argument)?);
    }
    Ok(parts.join(" "))
}

pub(super) fn shell_quote(value: &OsStr) -> anyhow::Result<String> {
    let value = value.to_str().ok_or_else(|| {
        anyhow!("`su` privilege fallback requires UTF-8 command arguments in the current frontend")
    })?;
    if value.is_empty() {
        return Ok("''".to_owned());
    }

    let mut quoted = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(ch);
        }
    }
    quoted.push('\'');

    Ok(quoted)
}
