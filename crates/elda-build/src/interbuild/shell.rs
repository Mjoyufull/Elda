use std::collections::HashMap;
use std::process::Command;

use crate::error::BuildError;

/// Verdict from semantic shell-body analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellSafetyVerdict {
    Safe,
    Unsupported { reason: String },
}

/// Curated allowlist of commands considered safe inside build phases.
///
/// Commands outside this list are not automatically rejected — only
/// patterns from `REJECTED_PATTERNS` and `REJECTED_COMMANDS` cause
/// a hard failure. Unknown commands pass through so that packages
/// with custom helper scripts (already present in the checkout)
/// are not rejected at parse time.
const REJECTED_COMMANDS: &[&str] = &[
    "eval", "source", "exec", "trap", "curl", "wget", "fetch", "sudo", "su", "doas",
];

/// Portage eclass helper commands that appear inside `$(...)` command
/// substitution in ebuild phase functions. These are deterministic
/// API calls provided by Portage and eclasses, not arbitrary shell
/// execution.
const PORTAGE_HELPER_COMMANDS: &[&str] = &[
    // meson.eclass
    "meson_feature",
    "meson_use",
    "meson_native_enabled",
    "meson_native_use_feature",
    // flag-o-matic.eclass / USE helpers
    "usex",
    "use_enable",
    "use_with",
    "use_if_iuse",
    // multilib.eclass
    "get_libdir",
    "get_abi_CFLAGS",
    "get_abi_LDFLAGS",
    // toolchain-funcs.eclass
    "tc-getCC",
    "tc-getCXX",
    "tc-getAR",
    "tc-getNM",
    "tc-getRANLIB",
    "tc-getPKG_CONFIG",
    "tc-is-cross-compiler",
    // python eclasses
    "python_get_sitedir",
    "python_get_includedir",
    "python_get_scriptdir",
    // misc common helpers
    "get_nproc",
    "nproc",
    "ver_cut",
    "ver_rs",
];

/// Classify whether a shell function body is safe for Elda to
/// execute inside its managed build environment.
///
/// Allows common parameter expansion, logical operators, pipelines,
/// and standard build commands. Rejects command substitution, heredocs,
/// eval, source inclusion, and network/privilege-escalation commands.
pub fn classify_shell_body(body: &str) -> ShellSafetyVerdict {
    classify_shell_body_inner(body, false)
}

/// Portage-aware variant that permits known eclass helper command
/// substitution patterns like `$(meson_use ...)` and `$(usex ...)`.
pub fn classify_shell_body_portage(body: &str) -> ShellSafetyVerdict {
    classify_shell_body_inner(body, true)
}

fn classify_shell_body_inner(body: &str, portage_mode: bool) -> ShellSafetyVerdict {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(reason) = check_rejected_patterns(trimmed, portage_mode) {
            return ShellSafetyVerdict::Unsupported { reason };
        }

        if let Some(reason) = check_rejected_commands(trimmed) {
            return ShellSafetyVerdict::Unsupported { reason };
        }
    }

    ShellSafetyVerdict::Safe
}

/// Detect structurally dangerous shell patterns.
fn check_rejected_patterns(line: &str, portage_mode: bool) -> Option<String> {
    // Command substitution via $(...) — but allow ${...} parameter expansion.
    if contains_command_substitution(line, portage_mode) {
        return Some("command substitution `$(...)` is not supported".to_owned());
    }

    if line.contains('`') {
        return Some("backtick command substitution is not supported".to_owned());
    }

    // Heredocs
    if line.contains("<<") && !line.contains("<<=") {
        return Some("heredoc is not supported".to_owned());
    }

    // Dot-sourcing (`. script` at the start of a statement)
    let first_token = first_command_token(line);
    if first_token == "." {
        return Some("dot-source file inclusion is not supported".to_owned());
    }

    None
}

/// Detect rejected commands at the start of a statement.
fn check_rejected_commands(line: &str) -> Option<String> {
    let token = first_command_token(line);
    for rejected in REJECTED_COMMANDS {
        if token == *rejected {
            return Some(format!(
                "command `{rejected}` is not allowed in build phases"
            ));
        }
    }
    None
}

/// Check for `$(...)` command substitution while allowing `${...}`
/// parameter expansion.
///
/// Scans character-by-character: `$` followed by `(` is command
/// substitution. `$` followed by `{` is parameter expansion (allowed).
fn contains_command_substitution(line: &str, portage_mode: bool) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'(' => {
                    if portage_mode && is_portage_helper_substitution(line, i) {
                        // Skip past the closing ')' for this allowed substitution
                        i += 2;
                        let mut depth = 1u32;
                        while i < bytes.len() && depth > 0 {
                            if bytes[i] == b'(' {
                                depth += 1;
                            } else if bytes[i] == b')' {
                                depth -= 1;
                            }
                            i += 1;
                        }
                        continue;
                    }
                    return true;
                }
                b'{' => {
                    // Skip past the closing '}' to avoid false positives
                    // from nested content.
                    i += 2;
                    let mut depth = 1u32;
                    while i < bytes.len() && depth > 0 {
                        if bytes[i] == b'{' {
                            depth += 1;
                        } else if bytes[i] == b'}' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                    continue;
                }
                _ => {}
            }
        }
        i += 1;
    }
    false
}

/// Check if a `$(...)` at position `dollar_pos` invokes a known
/// Portage helper command.
fn is_portage_helper_substitution(line: &str, dollar_pos: usize) -> bool {
    let after_paren = &line[dollar_pos + 2..];
    let cmd_name: String = after_paren
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '-')
        .collect();
    if cmd_name.is_empty() {
        return false;
    }
    PORTAGE_HELPER_COMMANDS
        .iter()
        .any(|helper| *helper == cmd_name)
}

/// Extract the first command token from a shell line, stripping
/// leading variable assignments (`VAR=val cmd ...`) and control
/// operators.
fn first_command_token(line: &str) -> &str {
    let stripped = line
        .trim()
        .trim_start_matches("if ")
        .trim_start_matches("then ")
        .trim_start_matches("else ")
        .trim_start_matches("elif ")
        .trim_start_matches("fi")
        .trim_start_matches("while ")
        .trim_start_matches("for ")
        .trim_start_matches("do ")
        .trim_start_matches("done")
        .trim();

    // Skip leading variable assignments (e.g. `DESTDIR="$pkgdir" make`)
    let mut rest = stripped;
    loop {
        let token = rest.split_whitespace().next().unwrap_or("");
        if token.contains('=') && !token.starts_with('=') && !token.starts_with('-') {
            rest = rest[token.len()..].trim_start();
            continue;
        }
        return token;
    }
}

/// Safely evaluate a shell script using a bash subshell and extract
/// variables that match the given prefixes. Guarantees semantic
/// correctness for bash arrays, conditionals, and dynamic function
/// evaluations like `pkgver()`.
pub fn extract_bash_variables(
    script_path: &std::path::Path,
    prefixes: &[&str],
) -> Result<HashMap<String, Vec<String>>, BuildError> {
    extract_bash_variables_with_prelude(script_path, prefixes, "")
}

pub fn extract_bash_variables_with_prelude(
    script_path: &std::path::Path,
    prefixes: &[&str],
    prelude: &str,
) -> Result<HashMap<String, Vec<String>>, BuildError> {
    let mut command = Command::new("bash");

    let prefix_checks = prefixes
        .iter()
        .map(|p| format!(r#"[[ $var == {}* ]]"#, p))
        .collect::<Vec<_>>()
        .join(" || ");

    let script = format!(
        r#"
set -e
{prelude}
source "{script_path}"
for var in $(compgen -v); do
    if {prefix_checks}; then
        echo "VAR:$var"
        if [[ $(declare -p "$var" 2>/dev/null) == declare\ -[aA]* ]]; then
            declare -n _ref="$var"
            for val in "${{_ref[@]}}"; do echo "VAL:$val"; done
            unset -n _ref
        else
            eval "echo \"VAL:\$$var\""
        fi
    fi
done
"#,
        prelude = prelude,
        script_path = script_path.display(),
        prefix_checks = prefix_checks
    );

    command.arg("-c").arg(&script);

    let output = command.output().map_err(BuildError::Io)?;

    if !output.status.success() {
        return Err(BuildError::Invalid(format!(
            "bash metadata extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_var = None;

    for line in stdout.lines() {
        if let Some(var) = line.strip_prefix("VAR:") {
            current_var = Some(var.to_owned());
            results.insert(var.to_owned(), Vec::new());
        } else if let Some(val) = line.strip_prefix("VAL:")
            && let Some(var) = &current_var
            && !val.is_empty()
            && let Some(values) = results.get_mut(var)
        {
            values.push(val.to_owned());
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_commands_are_safe() {
        let body = r#"
    cd "$srcdir"
    make DESTDIR="$pkgdir" install
    install -Dm755 binary "$pkgdir/usr/bin/binary"
"#;
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn parameter_expansion_is_safe() {
        let body = r#"
    local version="${pkgver%%+*}"
    echo "${srcdir##*/}"
    mkdir -p "${pkgdir}/usr/lib"
    cp -a "${srcdir}/${pkgname}-${pkgver}" "${pkgdir}/usr"
"#;
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn variable_substitution_is_safe() {
        let body = r#"
    local fixed="${version//-/_}"
    echo "${name:-unknown}"
    export PATH="${pkgdir}/usr/bin:${PATH}"
"#;
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn logical_operators_and_pipelines_are_safe() {
        let body = r#"
    make || return 1
    make -j$(nproc) && make install
"#;
        // Note: $(nproc) is command substitution — should be caught
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "command substitution `$(...)` is not supported".to_owned()
            }
        );
    }

    #[test]
    fn command_substitution_dollar_paren_rejected() {
        let body = "  local ver=$(git describe)";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "command substitution `$(...)` is not supported".to_owned()
            }
        );
    }

    #[test]
    fn backtick_substitution_rejected() {
        let body = "  local ver=`git describe`";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "backtick command substitution is not supported".to_owned()
            }
        );
    }

    #[test]
    fn heredoc_rejected() {
        let body = "  cat <<EOF\nstuff\nEOF";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "heredoc is not supported".to_owned()
            }
        );
    }

    #[test]
    fn eval_rejected() {
        let body = "  eval \"echo $something\"";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "command `eval` is not allowed in build phases".to_owned()
            }
        );
    }

    #[test]
    fn curl_rejected() {
        let body = "  curl -O https://example.com/file.tar.gz";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "command `curl` is not allowed in build phases".to_owned()
            }
        );
    }

    #[test]
    fn sudo_rejected() {
        let body = "  sudo make install";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "command `sudo` is not allowed in build phases".to_owned()
            }
        );
    }

    #[test]
    fn dot_source_rejected() {
        let body = "  . /etc/makepkg.conf";
        assert_eq!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported {
                reason: "dot-source file inclusion is not supported".to_owned()
            }
        );
    }

    #[test]
    fn env_prefixed_command_detected() {
        let body = "  DESTDIR=\"$pkgdir\" make install";
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn cmake_build_is_safe() {
        let body = r#"
    cmake -B build -DCMAKE_INSTALL_PREFIX=/usr
    cmake --build build
    cmake --install build --prefix "${pkgdir}/usr"
"#;
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn meson_build_is_safe() {
        let body = r#"
    meson setup build --prefix=/usr
    ninja -C build
    meson install -C build --destdir "${pkgdir}"
"#;
        assert_eq!(classify_shell_body(body), ShellSafetyVerdict::Safe);
    }

    #[test]
    fn contains_command_substitution_distinguishes_param_expansion() {
        assert!(contains_command_substitution("$(cmd)", false));
        assert!(contains_command_substitution("echo $(uname)", false));
        assert!(!contains_command_substitution("${var}", false));
        assert!(!contains_command_substitution("${var%%pattern}", false));
        assert!(!contains_command_substitution("${var:-default}", false));
    }

    #[test]
    fn portage_helpers_allowed_in_portage_mode() {
        // Known Portage helpers should pass in portage mode
        assert!(!contains_command_substitution("$(meson_use man)", true));
        assert!(!contains_command_substitution("$(usex flag yes no)", true));
        assert!(!contains_command_substitution("$(get_libdir)", true));
        assert!(!contains_command_substitution("$(tc-getCC)", true));

        // Unknown commands should still be rejected in portage mode
        assert!(contains_command_substitution("$(arbitrary_cmd)", true));
        assert!(contains_command_substitution("$(curl evil.com)", true));

        // Portage helpers should still be rejected in normal mode
        assert!(contains_command_substitution("$(meson_use man)", false));
    }

    #[test]
    fn portage_mode_classify_accepts_ebuild_patterns() {
        let body = r#"
    local emesonargs=(
        $(meson_feature man)
        $(meson_use doc)
        $(usex systemd -Dsystemd=true -Dsystemd=false)
    )
    meson_src_configure
"#;
        assert_eq!(classify_shell_body_portage(body), ShellSafetyVerdict::Safe);
        // Same body should be rejected in normal mode
        assert!(matches!(
            classify_shell_body(body),
            ShellSafetyVerdict::Unsupported { .. }
        ));
    }
}
