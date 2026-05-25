use std::env;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use anyhow::bail;
use elda_core::{PrivilegeProvider, PrivilegeRequest};

use super::provider::{ResolvedProvider, provider_label, resolve_provider};
use super::shell::render_shell_command;

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

fn configure_provider_command(
    command: &mut Command,
    resolved: &ResolvedProvider,
    request: &PrivilegeRequest,
    current_exe: &Path,
    forwarded_args: &[std::ffi::OsString],
) -> anyhow::Result<()> {
    match resolved.effective {
        PrivilegeProvider::Doas | PrivilegeProvider::Sudo => {
            configure_sudo_like_command(command, request, current_exe, forwarded_args);
        }
        PrivilegeProvider::Run0 => {
            forward_operator_context(command);
            command.arg(current_exe).args(forwarded_args);
        }
        PrivilegeProvider::Su => {
            configure_su_command(command, request, current_exe, forwarded_args)?;
        }
        PrivilegeProvider::Auto | PrivilegeProvider::None => unreachable!(),
    }

    Ok(())
}

fn configure_sudo_like_command(
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
    forward_operator_context(command);
    command.arg("--").arg(current_exe).args(forwarded_args);
}

/// Preserve invoking-operator paths through sudo/doas clean-env re-exec.
fn forward_operator_context(command: &mut Command) {
    command.arg(format!("ELDA_AFTER_PRIVILEGE=1"));
    append_operator_context_args(command);
}

fn append_operator_context_args(command: &mut Command) {
    if let Ok(home) = env::var("HOME") {
        command.arg(format!("ELDA_OPERATOR_HOME={home}"));
    }
    if let Ok(uid) = env::var("UID") {
        command.arg(format!("ELDA_OPERATOR_UID={uid}"));
    } else if let Ok(uid) = env::var("SUDO_UID") {
        command.arg(format!("ELDA_OPERATOR_UID={uid}"));
    }
}

fn operator_context_prefix() -> String {
    let mut parts = vec!["ELDA_AFTER_PRIVILEGE=1".to_owned()];
    if let Ok(home) = env::var("HOME") {
        parts.push(format!("ELDA_OPERATOR_HOME={home}"));
    }
    if let Ok(uid) = env::var("UID") {
        parts.push(format!("ELDA_OPERATOR_UID={uid}"));
    } else if let Ok(uid) = env::var("SUDO_UID") {
        parts.push(format!("ELDA_OPERATOR_UID={uid}"));
    }
    parts.join(" ")
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
    shell = format!("{} {}", operator_context_prefix(), shell);
    command.arg("-c").arg(shell).arg("root");

    Ok(())
}
