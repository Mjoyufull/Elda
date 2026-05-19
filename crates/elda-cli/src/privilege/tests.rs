use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use elda_core::{PrivilegeProvider, PrivilegeRequest};
use tempfile::TempDir;

use super::command::render_privilege_frame;
use super::provider::resolve_provider;
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

    assert!(frame.contains("Privilege Escalation"));
    assert!(frame.contains("selected:  sudo"));
    assert!(frame.contains(&sudo.display().to_string()));
    assert!(frame.contains("non-interactive"));
    assert!(frame.contains("preserve selected environment"));
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
