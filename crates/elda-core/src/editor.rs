use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::CoreError;

const FALLBACK_EDITORS: &[&str] = &["nvim", "vim", "nano", "vi", "hx", "micro", "emacs"];
const FALLBACK_PAGERS: &[&str] = &["less", "most", "more"];

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PagerSelection {
    program: PathBuf,
    args: Vec<OsString>,
    source: &'static str,
}

impl PagerSelection {
    pub(crate) fn source(&self) -> &'static str {
        self.source
    }

    pub(crate) fn display_program(&self) -> String {
        self.program.display().to_string()
    }
}

pub(crate) fn select_pager() -> Result<PagerSelection, CoreError> {
    for (source, value) in [
        ("ELDA_PAGER", env::var_os("ELDA_PAGER")),
        ("PAGER", env::var_os("PAGER")),
    ] {
        if let Some(value) = value {
            return parse_pager_spec(&value, source);
        }
    }

    for candidate in FALLBACK_PAGERS {
        if let Some(program) = lookup_program(candidate) {
            return Ok(PagerSelection {
                program,
                args: vec![OsString::from("-R")],
                source: "fallback",
            });
        }
    }

    Err(CoreError::Operator(
        "no pager is configured; set `ELDA_PAGER` or `PAGER`, or install `less`".to_owned(),
    ))
}

pub(crate) fn open_path_in_pager(path: &Path, title: &str) -> Result<String, CoreError> {
    let selection = select_pager()?;
    let status = Command::new(&selection.program)
        .args(&selection.args)
        .arg(path)
        .status()?;

    if status.success() {
        return Ok(format!(
            "{} via {} ({})",
            title,
            selection.display_program(),
            selection.source()
        ));
    }

    Err(CoreError::Operator(format!(
        "pager `{}` exited unsuccessfully while opening `{}`",
        selection.display_program(),
        path.display(),
    )))
}

pub(crate) fn open_paths_in_diff_pager(
    left: &Path,
    right: &Path,
    title: &str,
) -> Result<String, CoreError> {
    for (program, args) in [("diff", vec!["-y", "--"]), ("colordiff", vec!["-y", "--"])] {
        if let Some(binary) = lookup_program(program) {
            let status = Command::new(&binary)
                .args(args)
                .arg(left)
                .arg(right)
                .status()?;
            if status.success() {
                return Ok(format!(
                    "{title} side-by-side via {program} ({})",
                    left.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("left"),
                ));
            }
        }
    }

    let unified = Command::new("diff")
        .arg("-u")
        .arg("--label")
        .arg(format!("previous ({})", left.display()))
        .arg("--label")
        .arg(format!("current ({})", right.display()))
        .arg(left)
        .arg(right)
        .output();
    match unified {
        Ok(output) if output.status.success() || output.status.code() == Some(1) => {
            let body = String::from_utf8_lossy(&output.stdout);
            open_text_in_pager(&body, title)?;
            Ok(format!("{title} unified diff via diff -u"))
        }
        _ => open_path_in_pager(right, title),
    }
}

pub(crate) fn open_text_in_pager(content: &str, title: &str) -> Result<(), CoreError> {
    let temp_dir = env::temp_dir().join(format!("elda-review-{}", std::process::id()));
    fs::create_dir_all(&temp_dir)?;
    let path = temp_dir.join("review.txt");
    fs::write(&path, content)?;
    open_path_in_pager(&path, title)?;
    let _ = fs::remove_dir_all(temp_dir);
    Ok(())
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

fn parse_pager_spec(value: &OsString, source: &'static str) -> Result<PagerSelection, CoreError> {
    let value = value.to_string_lossy();
    let mut parts = value.split_whitespace();
    let program_name = parts.next().ok_or_else(|| {
        CoreError::Operator(format!(
            "{source} is set but does not contain a runnable pager command"
        ))
    })?;
    let program = resolve_program(program_name).ok_or_else(|| {
        CoreError::Operator(format!(
            "{source} points to `{program_name}`, but that pager is not available in PATH"
        ))
    })?;

    Ok(PagerSelection {
        program,
        args: parts.map(OsString::from).collect(),
        source,
    })
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
