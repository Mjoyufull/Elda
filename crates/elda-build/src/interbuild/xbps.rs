use std::collections::HashMap;
use std::fs;
use std::path::Path;

use elda_recipe::{RecipeDocument, ScalarValue};

use crate::error::BuildError;
use crate::interbuild::shell::{self, ShellSafetyVerdict};
use crate::interbuild::source::InterbuildSource;
use crate::interbuild::{InterbuildReport, PhaseCommandReport, XbpsReport};

pub struct ValidatedTemplate {
    pub upstream_source: Option<InterbuildSource>,
    pub report: InterbuildReport,
}

pub fn validate_template(
    recipe: &RecipeDocument,
    source_dir: &Path,
) -> Result<ValidatedTemplate, BuildError> {
    let package = string_field(recipe, "package").unwrap_or("");
    let path = source_dir.join(package).join("template");
    let contents = fs::read_to_string(&path).map_err(|error| {
        BuildError::Invalid(format!(
            "xbps_template source for `{}` (package: `{package}`) does not contain a readable template: {error}",
            recipe.package.name
        ))
    })?;

    let vars = shell::extract_bash_variables(
        &path,
        &[
            "pkgname",
            "version",
            "revision",
            "short_desc",
            "homepage",
            "license",
            "distfiles",
            "checksum",
            "depends",
            "makedepends",
            "hostmakedepends",
            "checkdepends",
            "provides",
            "conflicts",
            "archs",
            "build_style",
            "configure_args",
            "make_build_args",
            "make_install_args",
            "make_check",
        ],
    )?;

    validate_required_metadata(recipe, &vars)?;
    validate_distfile_integrity(recipe, &vars)?;
    let phase_commands = validate_supported_functions(recipe, &contents)?;
    let upstream_source = xbps_upstream_source(&vars);

    let pkgname = first_var(&vars, "pkgname").unwrap_or_else(|| recipe.package.name.clone());

    let report = XbpsReport {
        pkgname,
        version: first_var(&vars, "version").unwrap_or_default(),
        revision: first_var(&vars, "revision").unwrap_or_default(),
        short_desc: first_var(&vars, "short_desc").unwrap_or_default(),
        homepage: first_var(&vars, "homepage").unwrap_or_default(),
        license: split_first_var(&vars, "license"),
        distfiles: split_first_var(&vars, "distfiles"),
        checksum: split_first_var(&vars, "checksum"),
        depends: split_first_var(&vars, "depends"),
        makedepends: split_first_var(&vars, "makedepends"),
        hostmakedepends: split_first_var(&vars, "hostmakedepends"),
        checkdepends: split_first_var(&vars, "checkdepends"),
        provides: split_first_var(&vars, "provides"),
        conflicts: split_first_var(&vars, "conflicts"),
        archs: split_first_var(&vars, "archs"),
        build_style: first_var(&vars, "build_style"),
        configure_args: first_var(&vars, "configure_args"),
        functions: function_names(&contents),
        phase_commands,
    };

    Ok(ValidatedTemplate {
        upstream_source,
        report: InterbuildReport::xbps_template(report),
    })
}

fn first_var(vars: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    vars.get(key).and_then(|v| v.first()).cloned()
}

fn split_first_var(vars: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    first_var(vars, key)
        .map(|v| split_words(&v))
        .unwrap_or_default()
}

fn xbps_upstream_source(vars: &HashMap<String, Vec<String>>) -> Option<InterbuildSource> {
    let distfile = split_first_var(vars, "distfiles").into_iter().next()?;
    let sha256 = split_first_var(vars, "checksum").into_iter().next();
    Some(InterbuildSource::Archive {
        url: distfile,
        sha256,
    })
}

fn string_field<'a>(recipe: &'a RecipeDocument, key: &str) -> Option<&'a str> {
    match recipe.package.source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn validate_required_metadata(
    recipe: &RecipeDocument,
    vars: &HashMap<String, Vec<String>>,
) -> Result<(), BuildError> {
    for key in [
        "version",
        "revision",
        "short_desc",
        "homepage",
        "license",
        "distfiles",
    ] {
        if !vars.contains_key(key) || vars.get(key).map(|v| v.is_empty()).unwrap_or(true) {
            return Err(BuildError::Invalid(format!(
                "xbps_template for `{}` is missing `{key}` metadata",
                recipe.package.name
            )));
        }
    }
    Ok(())
}

fn validate_distfile_integrity(
    recipe: &RecipeDocument,
    vars: &HashMap<String, Vec<String>>,
) -> Result<(), BuildError> {
    let distfiles = split_first_var(vars, "distfiles");
    let checksums = split_first_var(vars, "checksum");
    if distfiles.len() == checksums.len() {
        return Ok(());
    }

    Err(BuildError::Invalid(format!(
        "xbps_template for `{}` has {} distfile entry(s) but {} checksum entry(s)",
        recipe.package.name,
        distfiles.len(),
        checksums.len()
    )))
}

/// All standard xbps-src phase hooks that can appear in templates.
const ALLOWED_XBPS_FUNCTIONS: &[&str] = &[
    "pre_fetch",
    "do_fetch",
    "post_fetch",
    "pre_extract",
    "do_extract",
    "post_extract",
    "pre_configure",
    "do_configure",
    "post_configure",
    "pre_build",
    "do_build",
    "post_build",
    "pre_check",
    "do_check",
    "post_check",
    "pre_install",
    "do_install",
    "post_install",
];

fn validate_supported_functions(
    recipe: &RecipeDocument,
    contents: &str,
) -> Result<Vec<PhaseCommandReport>, BuildError> {
    let mut reports = Vec::new();
    for function in function_names(contents) {
        // Subpackage declaration functions (e.g. "hyprland-devel_package")
        // follow the `{name}_package` naming convention. They are
        // declarative metadata blocks, not build phases — skip them.
        if function.ends_with("_package") {
            continue;
        }

        if !ALLOWED_XBPS_FUNCTIONS.contains(&function.as_str()) {
            return Err(BuildError::Unsupported(format!(
                "xbps_template for `{}` defines unsupported function `{function}`",
                recipe.package.name
            )));
        }
        let body = function_body(contents, &function).unwrap_or_default();
        if let ShellSafetyVerdict::Unsupported { reason } = shell::classify_shell_body(body) {
            return Err(BuildError::Unsupported(format!(
                "xbps_template for `{}` uses unsupported shell in \
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

fn split_words(value: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }
        match (quote, ch) {
            (Some(active), _) if ch == active => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, _) if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }

    words
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_words_handles_quoted_values() {
        assert_eq!(split_words("'MIT' 'Apache-2.0'"), vec!["MIT", "Apache-2.0"]);
    }

    #[test]
    fn split_words_preserves_spaces_inside_quotes() {
        assert_eq!(
            split_words("'foo: runtime library' 'bar: optional tool'"),
            vec!["foo: runtime library", "bar: optional tool"]
        );
    }
}
