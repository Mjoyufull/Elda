use std::collections::HashMap;
use std::fs;
use std::path::Path;

use elda_recipe::RecipeDocument;

use crate::error::BuildError;
use crate::interbuild::shell::{self, ShellSafetyVerdict};
use crate::interbuild::{ArchSourceReport, AurReport, InterbuildReport, PhaseCommandReport};

pub fn validate_pkgbuild(
    recipe: &RecipeDocument,
    source_dir: &Path,
) -> Result<InterbuildReport, BuildError> {
    let path = source_dir.join("PKGBUILD");
    let contents = fs::read_to_string(&path).map_err(|error| {
        BuildError::Invalid(format!(
            "aur_pkgbuild source for `{}` does not contain a readable PKGBUILD: {error}",
            recipe.package.name
        ))
    })?;

    let vars = shell::extract_bash_variables(
        &path,
        &[
            "pkgname",
            "pkgver",
            "pkgrel",
            "epoch",
            "pkgdesc",
            "url",
            "license",
            "source",
            "depends",
            "makedepends",
            "checkdepends",
            "optdepends",
            "provides",
            "conflicts",
            "replaces",
            "sha256sums",
            "b2sums",
            "sha512sums",
            "sha384sums",
            "sha224sums",
            "sha1sums",
            "md5sums",
        ],
    )?;

    validate_required_metadata(recipe, &vars)?;
    validate_source_integrity(recipe, &vars)?;

    let functions = function_names(&contents);
    let phase_commands = validate_supported_functions(recipe, &contents)?;

    let pkgname = vars
        .get("pkgname")
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_else(|| recipe.package.name.clone());

    let report = AurReport {
        pkgname,
        pkgver: first_var(&vars, "pkgver").unwrap_or_default(),
        pkgrel: first_var(&vars, "pkgrel").unwrap_or_default(),
        epoch: first_var(&vars, "epoch"),
        pkgdesc: first_var(&vars, "pkgdesc").unwrap_or_default(),
        url: first_var(&vars, "url").unwrap_or_default(),
        license: vars.get("license").cloned().unwrap_or_default(),
        source: vars.get("source").cloned().unwrap_or_default(),
        arch_sources: arch_source_reports(&vars),
        vcs_sources: vcs_sources(&vars),
        pkgver_function: functions.iter().any(|function| function == "pkgver"),
        depends: vars.get("depends").cloned().unwrap_or_default(),
        makedepends: vars.get("makedepends").cloned().unwrap_or_default(),
        checkdepends: vars.get("checkdepends").cloned().unwrap_or_default(),
        optdepends: vars.get("optdepends").cloned().unwrap_or_default(),
        provides: vars.get("provides").cloned().unwrap_or_default(),
        conflicts: vars.get("conflicts").cloned().unwrap_or_default(),
        replaces: vars.get("replaces").cloned().unwrap_or_default(),
        functions,
        phase_commands,
    };

    Ok(InterbuildReport::aur_pkgbuild(report))
}

fn first_var(vars: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    vars.get(key).and_then(|v| v.first()).cloned()
}

fn validate_required_metadata(
    recipe: &RecipeDocument,
    vars: &HashMap<String, Vec<String>>,
) -> Result<(), BuildError> {
    for key in ["pkgver", "pkgrel", "pkgdesc", "url", "license", "source"] {
        if !vars.contains_key(key) || vars.get(key).map(|v| v.is_empty()).unwrap_or(true) {
            // Note: If an AUR package purely uses arch-specific sources (e.g. source_x86_64),
            // this validation will fail if `source` is entirely missing. This matches prior behavior.
            if key == "source" && vars.keys().any(|k| k.starts_with("source_")) {
                continue;
            }
            return Err(BuildError::Invalid(format!(
                "aur_pkgbuild for `{}` is missing `{key}` metadata",
                recipe.package.name
            )));
        }
    }
    Ok(())
}

fn validate_source_integrity(
    recipe: &RecipeDocument,
    vars: &HashMap<String, Vec<String>>,
) -> Result<(), BuildError> {
    for source_key in vars
        .keys()
        .filter(|k| *k == "source" || k.starts_with("source_"))
    {
        let sources = &vars[source_key];
        let suffix = source_key.strip_prefix("source").unwrap_or_default();
        let checksums = first_checksum_array(vars, suffix);

        if sources.is_empty() || checksums.len() == sources.len() {
            continue;
        }

        return Err(BuildError::Invalid(format!(
            "aur_pkgbuild for `{}` has {} `{source_key}` source entry(s) but {} checksum entry(s)",
            recipe.package.name,
            sources.len(),
            checksums.len()
        )));
    }

    Ok(())
}

fn arch_source_reports(vars: &HashMap<String, Vec<String>>) -> Vec<ArchSourceReport> {
    vars.keys()
        .filter(|k| k.starts_with("source_"))
        .map(|source_key| {
            let suffix = source_key.strip_prefix("source").unwrap_or_default();
            ArchSourceReport {
                arch: suffix.trim_start_matches('_').to_owned(),
                source: vars.get(source_key).cloned().unwrap_or_default(),
                checksum: first_checksum_array(vars, suffix),
            }
        })
        .collect()
}

fn vcs_sources(vars: &HashMap<String, Vec<String>>) -> Vec<String> {
    vars.keys()
        .filter(|k| *k == "source" || k.starts_with("source_"))
        .flat_map(|source_key| vars.get(source_key).cloned().unwrap_or_default())
        .filter(|source| is_vcs_source(source))
        .collect()
}

fn is_vcs_source(source: &str) -> bool {
    let url = source.rsplit_once("::").map_or(source, |(_, url)| url);
    let lower = url.to_ascii_lowercase();
    lower.starts_with("git+")
        || lower.starts_with("hg+")
        || lower.starts_with("svn+")
        || lower.starts_with("bzr+")
        || lower.starts_with("fossil+")
        || lower.ends_with(".git")
        || lower.contains(".git#")
}

fn first_checksum_array(vars: &HashMap<String, Vec<String>>, suffix: &str) -> Vec<String> {
    [
        "sha256sums",
        "b2sums",
        "sha512sums",
        "sha384sums",
        "sha224sums",
        "sha1sums",
        "md5sums",
    ]
    .into_iter()
    .map(|key| format!("{key}{suffix}"))
    .find_map(|key| {
        let values = vars.get(&key).cloned().unwrap_or_default();
        (!values.is_empty()).then_some(values)
    })
    .unwrap_or_default()
}

fn validate_supported_functions(
    recipe: &RecipeDocument,
    contents: &str,
) -> Result<Vec<PhaseCommandReport>, BuildError> {
    let mut reports = Vec::new();
    for function in function_names(contents) {
        if !["pkgver", "prepare", "build", "check", "package"].contains(&function.as_str()) {
            return Err(BuildError::Unsupported(format!(
                "aur_pkgbuild for `{}` defines unsupported function `{function}`",
                recipe.package.name
            )));
        }
        let body = function_body(contents, &function).unwrap_or_default();
        // pkgver() commonly uses command substitution to derive the
        // version from VCS state — bash evaluation already extracts
        // its output, so it is exempt from shell safety analysis.
        if function != "pkgver"
            && let ShellSafetyVerdict::Unsupported { reason } = shell::classify_shell_body(body)
        {
            return Err(BuildError::Unsupported(format!(
                "aur_pkgbuild for `{}` uses unsupported shell in \
                 `{function}`: {reason}",
                recipe.package.name
            )));
        }
        reports.push(PhaseCommandReport {
            phase: function,
            commands: command_lines(body),
        });
    }
    Ok(reports)
}

fn command_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn function_names(contents: &str) -> Vec<String> {
    contents
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_suffix("() {").map(str::trim))
        .map(ToOwned::to_owned)
        .collect()
}

fn function_body<'a>(contents: &'a str, function: &str) -> Option<&'a str> {
    let start = contents.find(&format!("{function}()"))?;
    let after_start = &contents[start..];
    let open = after_start.find('{')? + start + 1;
    let close = contents[open..].find(
        "
}",
    )? + open;
    Some(&contents[open..close])
}
