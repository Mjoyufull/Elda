use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use elda_core::{PrivilegeProvider, PrivilegeRequest};
use tempfile::TempDir;

use super::command::{configure_provider_command, render_privilege_frame};
use super::provider::{ResolvedProvider, resolve_provider};
use super::shell::{render_shell_command, shell_quote};

#[test]
fn auto_provider_resolution_uses_documented_detection_order() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    create_executable(tempdir.path().join("run0"));
    create_executable(tempdir.path().join("sudo"));

    let resolved = resolve_provider(
        &PrivilegeRequest {
            provider: PrivilegeProvider::Auto,
            preserve_env: false,
            interactive: true,
        },
        Some(OsString::from(tempdir.path().as_os_str())),
    )
    .expect("provider should resolve");

    assert_eq!(resolved.effective, PrivilegeProvider::Sudo);
    assert_eq!(resolved.binary_name, "sudo");
}

#[test]
fn explicit_provider_falls_back_to_detected_provider_when_missing() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    create_executable(tempdir.path().join("run0"));

    let resolved = resolve_provider(
        &PrivilegeRequest {
            provider: PrivilegeProvider::Doas,
            preserve_env: false,
            interactive: true,
        },
        Some(OsString::from(tempdir.path().as_os_str())),
    )
    .expect("provider should resolve");

    assert_eq!(resolved.requested, PrivilegeProvider::Doas);
    assert_eq!(resolved.effective, PrivilegeProvider::Run0);
}

#[test]
fn none_provider_returns_error() {
    let error = resolve_provider(
        &PrivilegeRequest {
            provider: PrivilegeProvider::None,
            preserve_env: false,
            interactive: true,
        },
        None,
    )
    .expect_err("none should not resolve");

    assert!(error.to_string().contains("disabled"));
}

#[test]
fn shell_quote_escapes_single_quotes() {
    let quoted = shell_quote(OsStr::new("a'b")).expect("shell quote should succeed");

    assert_eq!(quoted, "'a'\\''b'");
}

#[test]
fn render_shell_command_quotes_forwarded_arguments() {
    let command = render_shell_command(
        Path::new("/tmp/elda"),
        &[OsString::from("ls"), OsString::from("path with space")],
    )
    .expect("shell command should render");

    assert_eq!(command, "'/tmp/elda' 'ls' 'path with space'");
}

#[test]
fn privilege_frame_names_selected_provider_and_policy() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let sudo = tempdir.path().join("sudo");
    create_executable(sudo.clone());
    let request = PrivilegeRequest {
        provider: PrivilegeProvider::Auto,
        preserve_env: true,
        interactive: false,
    };
    let resolved = resolve_provider(&request, Some(OsString::from(tempdir.path().as_os_str())))
        .expect("provider should resolve");

    let frame = render_privilege_frame(&request, &resolved);

    assert_eq!(
        frame,
        format!(
            ":: privilege sudo (non-interactive, preserve env) via {}",
            sudo.display()
        )
    );
}

#[test]
fn run0_policy_uses_provider_flags_and_setenv() {
    let mut command = std::process::Command::new("run0");
    let request = PrivilegeRequest {
        provider: PrivilegeProvider::Run0,
        preserve_env: false,
        interactive: false,
    };
    configure_provider_command(
        &mut command,
        &resolved_provider(PrivilegeProvider::Run0),
        &request,
        Path::new("/tmp/elda"),
        &[OsString::from("ls")],
    )
    .expect("run0 command should configure");

    let args = command_args(&command);
    assert!(args.contains(&"--no-ask-password".to_owned()));
    assert!(args.contains(&"--setenv=ELDA_AFTER_PRIVILEGE=1".to_owned()));
    assert!(!args.iter().any(|arg| arg == "ELDA_AFTER_PRIVILEGE=1"));
}

#[test]
fn doas_uses_env_command_for_operator_context() {
    let mut command = std::process::Command::new("doas");
    let request = PrivilegeRequest {
        provider: PrivilegeProvider::Doas,
        preserve_env: false,
        interactive: false,
    };
    configure_provider_command(
        &mut command,
        &resolved_provider(PrivilegeProvider::Doas),
        &request,
        Path::new("/tmp/elda"),
        &[OsString::from("ls")],
    )
    .expect("doas command should configure");

    let args = command_args(&command);
    assert_eq!(args.first().map(String::as_str), Some("-n"));
    assert!(args.contains(&"/usr/bin/env".to_owned()));
    assert!(args.contains(&"ELDA_AFTER_PRIVILEGE=1".to_owned()));
}

#[test]
fn sudo_preserve_env_uses_sudo_policy_without_serializing_full_environment() {
    let mut command = std::process::Command::new("sudo");
    let request = PrivilegeRequest {
        provider: PrivilegeProvider::Sudo,
        preserve_env: true,
        interactive: true,
    };
    configure_provider_command(
        &mut command,
        &resolved_provider(PrivilegeProvider::Sudo),
        &request,
        Path::new("/tmp/elda"),
        &[OsString::from("ls")],
    )
    .expect("sudo command should configure");

    let args = command_args(&command);
    assert!(args.contains(&"-E".to_owned()));
    assert!(args.contains(&"/usr/bin/env".to_owned()));
    assert!(args.contains(&"ELDA_AFTER_PRIVILEGE=1".to_owned()));
    assert!(
        args.iter()
            .filter(|arg| arg.contains('='))
            .all(|arg| arg.starts_with("ELDA_"))
    );
}

fn resolved_provider(provider: PrivilegeProvider) -> ResolvedProvider {
    ResolvedProvider {
        requested: provider,
        effective: provider,
        binary_name: "fixture",
        binary_path: PathBuf::from("fixture"),
    }
}

fn command_args(command: &std::process::Command) -> Vec<String> {
    command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn create_executable(path: PathBuf) {
    fs::write(&path, "#!/bin/sh\nexit 0\n").expect("fixture executable should be written");
    let mut permissions = fs::metadata(&path)
        .expect("fixture executable metadata should exist")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions)
        .expect("fixture executable permissions should be updated");
}
