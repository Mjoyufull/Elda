use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::CoreError;

const FALLBACK_EDITORS: &[&str] = &["nvim", "vim", "nano", "vi", "hx", "micro", "emacs"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorSelection {
    program: PathBuf,
    args: Vec<OsString>,
    source: &'static str,
}

impl EditorSelection {
    pub(crate) fn source(&self) -> &'static str {
        self.source
    }

    pub(crate) fn display_program(&self) -> String {
        self.program.display().to_string()
    }
}

pub(crate) fn select_editor() -> Result<EditorSelection, CoreError> {
    for (source, value) in [
        ("VISUAL", env::var_os("VISUAL")),
        ("EDITOR", env::var_os("EDITOR")),
    ] {
        if let Some(value) = value {
            return parse_editor_spec(&value, source);
        }
    }

    for candidate in FALLBACK_EDITORS {
        if let Some(program) = lookup_program(candidate) {
            return Ok(EditorSelection {
                program,
                args: Vec::new(),
                source: "fallback",
            });
        }
    }

    Err(CoreError::Operator(
        "no editor is configured; set `VISUAL` or `EDITOR`, or install `nvim`, `vim`, `nano`, or `vi`".to_owned(),
    ))
}

pub(crate) fn open_path_in_editor(path: &Path) -> Result<EditorSelection, CoreError> {
    let selection = select_editor()?;
    let status = Command::new(&selection.program)
        .args(&selection.args)
        .arg(path)
        .status()?;

    if status.success() {
        return Ok(selection);
    }

    Err(CoreError::Operator(format!(
        "editor `{}` exited unsuccessfully while opening `{}`",
        selection.display_program(),
        path.display(),
    )))
}

fn parse_editor_spec(value: &OsString, source: &'static str) -> Result<EditorSelection, CoreError> {
    let value = value.to_string_lossy();
    let mut parts = value.split_whitespace();
    let program_name = parts.next().ok_or_else(|| {
        CoreError::Operator(format!(
            "{source} is set but does not contain a runnable editor command"
        ))
    })?;
    let program = resolve_program(program_name).ok_or_else(|| {
        CoreError::Operator(format!(
            "{source} points to `{program_name}`, but that editor is not available in PATH"
        ))
    })?;

    Ok(EditorSelection {
        program,
        args: parts.map(OsString::from).collect(),
        source,
    })
}

fn resolve_program(program_name: &str) -> Option<PathBuf> {
    let program_path = Path::new(program_name);
    if program_path.components().count() > 1 {
        return program_path.is_file().then(|| program_path.to_path_buf());
    }

    lookup_program(program_name)
}

fn lookup_program(program_name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(program_name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{EditorSelection, parse_editor_spec};

    #[test]
    fn parse_editor_spec_accepts_program_and_args() {
        let selection = parse_editor_spec(&"/bin/sh -c".into(), "EDITOR")
            .expect("editor selection should parse");

        assert_eq!(selection.source(), "EDITOR");
        assert_eq!(selection.display_program(), "/bin/sh");
        assert_eq!(selection.args, vec![OsString::from("-c")]);
    }

    #[test]
    fn parse_editor_spec_rejects_missing_binary() {
        let error = parse_editor_spec(&"/definitely/missing/editor".into(), "VISUAL")
            .expect_err("missing editor should be rejected");

        assert!(error.to_string().contains("not available"));
    }

    #[test]
    fn editor_selection_display_program_uses_path_display() {
        let selection = EditorSelection {
            program: PathBuf::from("/usr/bin/vim"),
            args: Vec::new(),
            source: "fallback",
        };

        assert_eq!(selection.display_program(), "/usr/bin/vim");
    }
}
