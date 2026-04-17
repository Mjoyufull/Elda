use std::path::Path;
use std::process::Command;

use super::super::{CommandRequest, OutputMode, run_from_root};
use super::fixture_remote_key_fingerprint;

pub(in crate::tests) fn run_installed_binary(root: &Path, binary_path: &str) -> String {
    let resolved_path = root.join(binary_path.trim_start_matches('/'));
    let output = Command::new(&resolved_path)
        .output()
        .expect("installed binary should launch");
    assert!(
        output.status.success(),
        "installed binary should succeed: {} stderr={}",
        resolved_path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub(in crate::tests) fn current_state_id(root: &Path) -> String {
    run_from_root(
        root,
        CommandRequest::new(
            vec!["state".to_owned(), "show".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("state show should succeed")
    .details
    .as_ref()
    .and_then(|details| details.get("active_state"))
    .and_then(|state| state.as_str())
    .map(ToOwned::to_owned)
    .expect("active state should exist")
}

pub(in crate::tests) fn register_fixture_remote(root: &Path, name: &str, index_path: &Path) {
    run_from_root(
        root,
        CommandRequest::new(
            vec!["rmt".to_owned(), "add".to_owned()],
            vec![
                format!("{name}=file://{}", index_path.display()),
                "--trust".to_owned(),
                "pinned".to_owned(),
                "--trusted-key".to_owned(),
                fixture_remote_key_fingerprint(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("fixture remote add should succeed");
}
