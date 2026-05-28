//! Minimal `.desktop` parsing and rewriting for Elda-managed launchers.

use std::collections::BTreeMap;

use crate::error::AppImageError;

pub(crate) const DESKTOP_EXEC_CODES: &[&str] = &["%f", "%F", "%u", "%U", "%i", "%c", "%k"];

#[derive(Debug, Clone)]
pub(crate) struct DesktopFile {
    pub(crate) sections: Vec<(String, BTreeMap<String, String>)>,
}

pub(crate) fn parse_desktop(raw: &str) -> Result<DesktopFile, AppImageError> {
    let mut sections: Vec<(String, BTreeMap<String, String>)> = Vec::new();
    let mut current: Option<(String, BTreeMap<String, String>)> = None;

    for raw_line in raw.lines() {
        let line = raw_line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            current = Some((line[1..line.len() - 1].trim().to_owned(), BTreeMap::new()));
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let Some(ref mut sec) = current else {
            continue;
        };
        sec.1.insert(key.trim().to_owned(), value.trim().to_owned());
    }

    if let Some(sec) = current {
        sections.push(sec);
    }

    Ok(DesktopFile { sections })
}

pub(crate) fn serialize_desktop(file: &DesktopFile) -> String {
    let mut out = String::new();
    for (name, map) in &file.sections {
        out.push('[');
        out.push_str(name);
        out.push_str("]\n");
        for (k, v) in map {
            out.push_str(k);
            out.push('=');
            out.push_str(v);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

pub(crate) fn desktop_entry_section(file: &DesktopFile) -> Option<&BTreeMap<String, String>> {
    file.sections
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Desktop Entry"))
        .map(|(_, map)| map)
}

pub(crate) fn rewrite_launcher_exec(
    exec: &str,
    launcher_binary: &str,
) -> Result<String, AppImageError> {
    let tokens = shell_split(exec)?;
    if tokens.is_empty() {
        return Err(AppImageError::DesktopParse(
            "empty Exec line after parsing".to_owned(),
        ));
    }

    let rest: Vec<&str> = tokens[1..]
        .iter()
        .copied()
        .filter(|token| !DESKTOP_EXEC_CODES.contains(token))
        .collect();

    let suffix = if rest.is_empty() {
        String::new()
    } else {
        format!(" {}", rest.join(" "))
    };

    Ok(format!("/usr/bin/{launcher_binary}{suffix}"))
}

fn shell_split(line: &str) -> Result<Vec<&str>, AppImageError> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }

        match bytes[idx] {
            b'"' => {
                idx += 1;
                let start = idx;
                while idx < bytes.len() && bytes[idx] != b'"' {
                    idx += 1;
                }
                if idx >= bytes.len() {
                    return Err(AppImageError::DesktopParse(
                        "unbalanced double quote in Exec".to_owned(),
                    ));
                }
                let token = line.get(start..idx).ok_or_else(|| {
                    AppImageError::DesktopParse("invalid UTF-8 slice in Exec".to_owned())
                })?;
                out.push(token);
                idx += 1;
            }
            b'\'' => {
                idx += 1;
                let start = idx;
                while idx < bytes.len() && bytes[idx] != b'\'' {
                    idx += 1;
                }
                if idx >= bytes.len() {
                    return Err(AppImageError::DesktopParse(
                        "unbalanced single quote in Exec".to_owned(),
                    ));
                }
                let token = line.get(start..idx).ok_or_else(|| {
                    AppImageError::DesktopParse("invalid UTF-8 slice in Exec".to_owned())
                })?;
                out.push(token);
                idx += 1;
            }
            _ => {
                let start = idx;
                while idx < bytes.len() && !bytes[idx].is_ascii_whitespace() {
                    idx += 1;
                }
                let token = line.get(start..idx).ok_or_else(|| {
                    AppImageError::DesktopParse("invalid UTF-8 slice in Exec".to_owned())
                })?;
                out.push(token);
            }
        }
    }

    Ok(out)
}

pub(crate) fn rewrite_desktop_exec_sections(
    raw: &str,
    launcher_binary: &str,
) -> Result<String, AppImageError> {
    let mut desktop = parse_desktop(raw)?;

    for (section_name, map) in &mut desktop.sections {
        let is_main = section_name.eq_ignore_ascii_case("Desktop Entry");
        let is_action = section_name
            .to_ascii_lowercase()
            .starts_with("desktop action ");

        if !(is_main || is_action) {
            continue;
        }

        if is_main {
            match map.get("Exec").cloned() {
                Some(exec) => {
                    let new_exec = rewrite_launcher_exec(&exec, launcher_binary)?;
                    map.insert("Exec".to_owned(), new_exec);
                    map.insert("TryExec".to_owned(), format!("/usr/bin/{launcher_binary}"));
                }
                None => {
                    map.insert("Exec".to_owned(), format!("/usr/bin/{launcher_binary}"));
                    map.insert("TryExec".to_owned(), format!("/usr/bin/{launcher_binary}"));
                }
            }
            continue;
        }

        if let Some(exec) = map.get("Exec").cloned() {
            let new_exec = rewrite_launcher_exec(&exec, launcher_binary)?;
            map.insert("Exec".to_owned(), new_exec);
        }
    }

    Ok(serialize_desktop(&desktop))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_rewrite_preserves_non_code_arguments() {
        let out = rewrite_launcher_exec(r#"AppRun --enable-features=Foo %U"#, "demo")
            .expect("launcher exec rewrite should succeed");
        assert_eq!(out, "/usr/bin/demo --enable-features=Foo");
    }
}
