//! Survives privilege re-exec so dispatch-level mutation prompts are not replayed.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::CommandRequest;
use crate::error::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct DispatchConfirmStamp {
    command_path: Vec<String>,
    operands: Vec<String>,
}

fn dispatch_confirm_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("dispatch-confirm.json")
}

pub(crate) fn dispatch_confirmation_matches(
    data_dir: &Path,
    request: &CommandRequest,
) -> Result<bool, CoreError> {
    let path = dispatch_confirm_path(data_dir);
    if !path.is_file() {
        return Ok(false);
    }
    let stamp: DispatchConfirmStamp = serde_json::from_slice(&std::fs::read(&path)?)?;
    Ok(stamp.command_path == request.command_path && stamp.operands == request.operands)
}

pub(crate) fn write_dispatch_confirmation(
    data_dir: &Path,
    request: &CommandRequest,
) -> Result<(), CoreError> {
    let path = dispatch_confirm_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let stamp = DispatchConfirmStamp {
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
    };
    std::fs::write(path, serde_json::to_vec_pretty(&stamp)?)?;
    Ok(())
}

pub(crate) fn clear_dispatch_confirmation(data_dir: &Path) -> Result<(), CoreError> {
    let path = dispatch_confirm_path(data_dir);
    if path.is_file() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
