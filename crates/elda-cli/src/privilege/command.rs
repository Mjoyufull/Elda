use std::env;
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::bail;
use elda_core::{PrivilegeProvider, PrivilegeRequest};

use super::provider::{ResolvedProvider, provider_label, resolve_provider};
use super::shell::{render_shell_command, shell_quote};

pub fn reexec_with_provider(request: &PrivilegeRequest) -> anyhow::Result<()> {
    if request.provider == PrivilegeProvider::None {
        bail!(
            "superuser access is required for this live host operation, but `privilege.provider = \"none\"`"
        );
    }

    let resolved = resolve_provider(request, env::var_os("PATH"))?;
    report_provider_fallback(request, &resolved);
    eprintln!("{}", render_privilege_frame(request, &resolved));

    let current_exe = env::current_exe()?;
    let forwarded_args = env::args_os().skip(1).collect::<Vec<_>>();
    let mut command = Command::new(&resolved.binary_path);
    configure_provider_command(
        &mut command,
        &resolved,
        request,
        &current_exe,
        &forwarded_args,
    )?;

    let error = command.exec();
    Err(error.into())
}

pub(super) fn render_privilege_frame(
    request: &PrivilegeRequest,
    resolved: &ResolvedProvider,
) -> String {
    let selected = provider_label(resolved.effective);
    let prompt = if request.interactive {
        "may prompt"
    } else {
        "non-interactive"
    };
    let env_policy = if request.preserve_env {
        "preserve env"
    } else {
        "clean env"
    };

    format!(
        ":: privilege {selected} ({prompt}, {env_policy}) via {}",
        resolved.binary_path.display()
    )
}

fn report_provider_fallback(request: &PrivilegeRequest, resolved: &ResolvedProvider) {
    if request.provider != PrivilegeProvider::Auto && resolved.requested != resolved.effective {
        eprintln!(
            "Configured privilege provider `{}` was unavailable; falling back to `{}`.",
            provider_label(resolved.requested),
            provider_label(resolved.effective)
        );
    }
}

pub(super) fn configure_provider_command(
    command: &mut Command,
    resolved: &ResolvedProvider,
    request: &PrivilegeRequest,
    current_exe: &Path,
    forwarded_args: &[std::ffi::OsString],
) -> anyhow::Result<()> {
    match resolved.effective {
        PrivilegeProvider::Sudo => {
            configure_sudo_command(command, request, current_exe, forwarded_args);
        }
        PrivilegeProvider::Doas => {
            configure_doas_command(command, request, current_exe, forwarded_args);
        }
        PrivilegeProvider::Run0 => {
            configure_run0_policy(command, request);
            command.arg(current_exe).args(forwarded_args);
        }
        PrivilegeProvider::Su => {
            configure_su_command(command, request, current_exe, forwarded_args)?;
        }
        PrivilegeProvider::Auto | PrivilegeProvider::None => unreachable!(),
    }

    Ok(())
}

fn configure_sudo_command(
    command: &mut Command,
    request: &PrivilegeRequest,
    current_exe: &Path,
    forwarded_args: &[std::ffi::OsString],
) {
    if request.preserve_env {
        command.arg("-E");
    }
    if !request.interactive {
        command.arg("-n");
    }
    command.arg("--");
    append_env_exec(command, request.preserve_env, current_exe, forwarded_args);
}

fn configure_doas_command(
    command: &mut Command,
    request: &PrivilegeRequest,
    current_exe: &Path,
    forwarded_args: &[OsString],
) {
    if !request.interactive {
        command.arg("-n");
    }
    append_env_exec(command, request.preserve_env, current_exe, forwarded_args);
}

fn configure_run0_policy(command: &mut Command, request: &PrivilegeRequest) {
    if !request.interactive {
        command.arg("--no-ask-password");
    }
    for (name, value) in env_assignments(request.preserve_env) {
        command.arg(format!("--setenv={name}={value}"));
    }
}

fn append_env_exec(
    command: &mut Command,
    preserve_env: bool,
    current_exe: &Path,
    forwarded_args: &[OsString],
) {
    command.arg("/usr/bin/env");
    for (name, value) in env_assignments(preserve_env) {
        command.arg(format!("{name}={value}"));
    }
    command.arg(current_exe).args(forwarded_args);
}

fn env_assignments(preserve_env: bool) -> Vec<(String, String)> {
    let mut assignments = if preserve_env {
        env::vars()
            .filter(|(name, _)| valid_env_name(name))
            .collect()
    } else {
        Vec::new()
    };
    upsert_assignment(&mut assignments, "ELDA_AFTER_PRIVILEGE", "1".to_owned());
    if let Ok(home) = env::var("HOME") {
        upsert_assignment(&mut assignments, "ELDA_OPERATOR_HOME", home);
    }
    if let Ok(uid) = env::var("UID").or_else(|_| env::var("SUDO_UID")) {
        upsert_assignment(&mut assignments, "ELDA_OPERATOR_UID", uid);
    }
    assignments
}

fn upsert_assignment(assignments: &mut Vec<(String, String)>, name: &str, value: String) {
    if let Some((_, existing)) = assignments
        .iter_mut()
        .find(|(existing, _)| existing == name)
    {
        *existing = value;
    } else {
        assignments.push((name.to_owned(), value));
    }
}

fn valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(ch) if ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn operator_context_prefix() -> anyhow::Result<String> {
    let mut parts = Vec::new();
    for (name, value) in env_assignments(false) {
        parts.push(format!(
            "{name}={}",
            shell_quote(std::ffi::OsStr::new(&value))?
        ));
    }
    Ok(parts.join(" "))
}

fn configure_su_command(
    command: &mut Command,
    request: &PrivilegeRequest,
    current_exe: &Path,
    forwarded_args: &[std::ffi::OsString],
) -> anyhow::Result<()> {
    if !request.interactive {
        bail!(
            "the detected privilege provider `su` does not support non-interactive elevation in the current frontend; install `sudo`, `doas`, or `run0`, or enable interactive escalation"
        );
    }
    if request.preserve_env {
        command.arg("-m");
    }
    let mut shell = render_shell_command(current_exe, forwarded_args)?;
    shell = format!("{} {shell}", operator_context_prefix()?);
    command.arg("-c").arg(shell).arg("root");

    Ok(())
}
