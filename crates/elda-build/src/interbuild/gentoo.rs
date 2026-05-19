use std::fs;
use std::path::{Path, PathBuf};

use elda_recipe::{RecipeDocument, ScalarValue};

use crate::error::BuildError;
use crate::interbuild::shell::{self, ShellSafetyVerdict};
use crate::interbuild::source::InterbuildSource;
use crate::interbuild::{GentooReport, InterbuildReport, PhaseCommandReport};

const SUPPORTED_EAPI: &str = "8";
const CURATED_ECLASSES: &[&str] = &[
    "autotools",
    "cargo",
    "cmake",
    "desktop",
    "flag-o-matic",
    "git-r3",
    "go-module",
    "linux-info",
    "meson",
    "multilib",
    "pam",
    "prefix",
    "python-any-r1",
    "python-r1",
    "python-single-r1",
    "readme.gentoo-r1",
    "systemd",
    "toolchain-funcs",
    "vcs-clean",
    "verify-sig",
    "xdg",
    "zig",
];
const PHASES: &[&str] = &[
    "src_unpack",
    "src_prepare",
    "src_configure",
    "src_compile",
    "src_install",
];
const GENTOO_METADATA_PRELUDE: &str = r#"
inherit() { :; }
EXPORT_FUNCTIONS() { :; }
die() { printf '%s\n' "$*" >&2; exit 1; }
use() { return 1; }
usex() { if use "$1"; then printf '%s' "${2-yes}"; else printf '%s' "${3-no}"; fi; }
use_enable() { use "$1" && printf '%s' "--enable-${2-$1}" || printf '%s' "--disable-${2-$1}"; }
use_with() { use "$1" && printf '%s' "--with-${2-$1}" || printf '%s' "--without-${2-$1}"; }
meson_feature() { use "$1" && printf '%s' "-D${2-$1}=enabled" || printf '%s' "-D${2-$1}=disabled"; }
meson_use() { use "$1" && printf '%s' "-D${2-$1}=true" || printf '%s' "-D${2-$1}=false"; }
"#;

pub struct ValidatedEbuild {
    pub package_dir: PathBuf,
    pub upstream_source: Option<InterbuildSource>,
    pub report: InterbuildReport,
}

pub fn validate_ebuild(
    recipe: &RecipeDocument,
    source_dir: &Path,
) -> Result<ValidatedEbuild, BuildError> {
    let ebuild_path = selected_ebuild_path(recipe, source_dir)?;
    let contents = fs::read_to_string(&ebuild_path).map_err(|error| {
        BuildError::Invalid(format!(
            "gentoo_overlay ebuild for `{}` could not be read: {error}",
            recipe.package.name
        ))
    })?;

    let vars = shell::extract_bash_variables_with_prelude(
        &ebuild_path,
        &[
            "EAPI",
            "DESCRIPTION",
            "HOMEPAGE",
            "LICENSE",
            "SRC_URI",
            "EGIT_REPO_URI",
            "SLOT",
            "DEPEND",
            "RDEPEND",
            "BDEPEND",
            "IUSE",
            "KEYWORDS",
        ],
        GENTOO_METADATA_PRELUDE,
    )?;

    let eapi = validate_eapi(recipe, &vars)?;
    let inherited_eclasses = validate_inherited_eclasses(recipe, &contents)?;
    validate_required_metadata(recipe, &vars)?;
    let upstream_source = gentoo_upstream_source(&vars);

    let phase_commands = validate_phase_subset(recipe, &contents)?;
    let phases = phase_commands
        .iter()
        .map(|phase| phase.phase.clone())
        .collect::<Vec<_>>();

    let package_dir = ebuild_path.parent().map(Path::to_path_buf).ok_or_else(|| {
        BuildError::Invalid(format!(
            "gentoo_overlay ebuild for `{}` has no package directory",
            recipe.package.name
        ))
    })?;

    let package = string_field(recipe, "package")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| recipe.package.name.clone());

    let report = GentooReport {
        package,
        ebuild: ebuild_path.display().to_string(),
        eapi,
        inherited_eclasses,
        description: first_var(&vars, "DESCRIPTION").unwrap_or_default(),
        homepage: first_var(&vars, "HOMEPAGE").unwrap_or_default(),
        license: split_first_var(&vars, "LICENSE"),
        src_uri: source_tokens(&vars),
        slot: first_var(&vars, "SLOT"),
        depend: dependency_tokens(&vars, "DEPEND"),
        rdepend: dependency_tokens(&vars, "RDEPEND"),
        bdepend: dependency_tokens(&vars, "BDEPEND"),
        iuse: split_first_var(&vars, "IUSE"),
        keywords: split_first_var(&vars, "KEYWORDS"),
        phases,
        phase_commands,
        gpkg_used: false,
        gpkg_use: Vec::new(),
    };

    Ok(ValidatedEbuild {
        package_dir,
        upstream_source,
        report: InterbuildReport::gentoo_overlay(report),
    })
}

fn first_var(vars: &std::collections::HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    vars.get(key).and_then(|v| v.first()).cloned()
}

fn split_first_var(
    vars: &std::collections::HashMap<String, Vec<String>>,
    key: &str,
) -> Vec<String> {
    first_var(vars, key)
        .map(|v| v.split_whitespace().map(ToOwned::to_owned).collect())
        .unwrap_or_default()
}

fn selected_ebuild_path(recipe: &RecipeDocument, source_dir: &Path) -> Result<PathBuf, BuildError> {
    if let Some(path) = string_field(recipe, "ebuild") {
        let ebuild_path = source_dir.join(path);
        if ebuild_path.is_file() {
            return Ok(ebuild_path);
        }

        return Err(BuildError::Invalid(format!(
            "gentoo_overlay ebuild `{path}` for `{}` was not found",
            recipe.package.name
        )));
    }

    let package = string_field(recipe, "package").ok_or_else(|| {
        BuildError::Invalid(format!(
            "gentoo_overlay source for `{}` is missing `package`",
            recipe.package.name
        ))
    })?;
    let package_dir = source_dir.join(package);
    let mut matches = fs::read_dir(&package_dir)
        .map_err(|error| {
            BuildError::Invalid(format!(
                "gentoo_overlay package directory `{package}` for `{}` could not be read: {error}",
                recipe.package.name
            ))
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    matches.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "ebuild")
    });
    matches.sort();

    match matches.as_slice() {
        [single] => Ok(single.clone()),
        [] => Err(BuildError::Invalid(format!(
            "gentoo_overlay package `{package}` for `{}` does not contain an ebuild",
            recipe.package.name
        ))),
        _ => Err(BuildError::Unsupported(format!(
            "gentoo_overlay package `{package}` for `{}` has multiple ebuilds; set source.ebuild explicitly",
            recipe.package.name
        ))),
    }
}

fn validate_eapi(
    recipe: &RecipeDocument,
    vars: &std::collections::HashMap<String, Vec<String>>,
) -> Result<String, BuildError> {
    let eapi = first_var(vars, "EAPI").unwrap_or_else(|| "0".to_owned());
    if eapi == SUPPORTED_EAPI {
        return Ok(eapi);
    }

    Err(BuildError::Unsupported(format!(
        "gentoo_overlay ebuild for `{}` uses EAPI `{eapi}`; supported EAPI is `{SUPPORTED_EAPI}`",
        recipe.package.name
    )))
}

fn validate_inherited_eclasses(
    recipe: &RecipeDocument,
    contents: &str,
) -> Result<Vec<String>, BuildError> {
    let inherited = inherited_eclasses(contents);
    for eclass in &inherited {
        if !CURATED_ECLASSES.contains(&eclass.as_str()) {
            return Err(BuildError::Unsupported(format!(
                "gentoo_overlay ebuild for `{}` inherits unsupported eclass `{eclass}`",
                recipe.package.name
            )));
        }
    }

    Ok(inherited)
}

fn validate_required_metadata(
    recipe: &RecipeDocument,
    vars: &std::collections::HashMap<String, Vec<String>>,
) -> Result<(), BuildError> {
    for key in ["DESCRIPTION", "HOMEPAGE", "LICENSE"] {
        if !vars.contains_key(key) || vars.get(key).map(|v| v.is_empty()).unwrap_or(true) {
            return Err(BuildError::Invalid(format!(
                "gentoo_overlay ebuild for `{}` is missing `{key}` metadata",
                recipe.package.name
            )));
        }
    }

    if first_var(vars, "SRC_URI").is_none() && first_var(vars, "EGIT_REPO_URI").is_none() {
        return Err(BuildError::Invalid(format!(
            "gentoo_overlay ebuild for `{}` is missing source metadata (`SRC_URI` or `EGIT_REPO_URI`)",
            recipe.package.name
        )));
    }

    Ok(())
}

fn source_tokens(vars: &std::collections::HashMap<String, Vec<String>>) -> Vec<String> {
    let src_uri = split_first_var(vars, "SRC_URI");
    if !src_uri.is_empty() {
        return src_uri;
    }

    split_first_var(vars, "EGIT_REPO_URI")
}

fn gentoo_upstream_source(
    vars: &std::collections::HashMap<String, Vec<String>>,
) -> Option<InterbuildSource> {
    first_var(vars, "EGIT_REPO_URI")
        .and_then(|value| value.split_whitespace().next().map(ToOwned::to_owned))
        .map(|url| InterbuildSource::Git { url })
        .or_else(|| {
            first_var(vars, "SRC_URI")
                .and_then(|value| value.split_whitespace().next().map(ToOwned::to_owned))
                .map(|url| InterbuildSource::Archive { url, sha256: None })
        })
}

fn validate_phase_subset(
    recipe: &RecipeDocument,
    contents: &str,
) -> Result<Vec<PhaseCommandReport>, BuildError> {
    let mut phases = Vec::new();
    for phase in PHASES {
        let Some(body) = phase_body(contents, phase) else {
            continue;
        };
        let commands = phase_commands(recipe, phase, body)?;
        phases.push(PhaseCommandReport {
            phase: (*phase).to_owned(),
            commands,
        });
    }

    Ok(phases)
}

fn phase_commands(
    recipe: &RecipeDocument,
    phase: &str,
    body: &str,
) -> Result<Vec<String>, BuildError> {
    if let ShellSafetyVerdict::Unsupported { reason } = shell::classify_shell_body_portage(body) {
        return Err(BuildError::Unsupported(format!(
            "gentoo_overlay ebuild for `{}` uses unsupported shell in \
             `{phase}`: {reason}",
            recipe.package.name
        )));
    }

    Ok(command_lines(body))
}

fn command_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn inherited_eclasses(contents: &str) -> Vec<String> {
    contents
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("inherit "))
        .flat_map(str::split_whitespace)
        .map(ToOwned::to_owned)
        .collect()
}

fn dependency_tokens(
    vars: &std::collections::HashMap<String, Vec<String>>,
    key: &str,
) -> Vec<String> {
    split_first_var(vars, key)
        .into_iter()
        .filter(|token| !token.starts_with("${") && token != "||" && token != "(" && token != ")")
        .collect()
}

fn phase_body<'a>(contents: &'a str, phase: &str) -> Option<&'a str> {
    let start = contents.find(&format!("{phase}()"))?;
    let after_start = &contents[start..];
    let open = after_start.find('{')? + start + 1;
    let close = contents[open..].find("\n}")? + open;
    Some(&contents[open..close])
}

fn string_field<'a>(recipe: &'a RecipeDocument, key: &str) -> Option<&'a str> {
    match recipe.package.source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Extract the binhost URL from recipe source metadata.
/// Used by the GPKG fast-path to locate binary packages.
pub(crate) fn binhost_from_recipe(recipe: &RecipeDocument) -> Option<String> {
    string_field(recipe, "binhost").map(ToOwned::to_owned)
}

/// Extract the Gentoo category from recipe source metadata.
/// Defaults to "dev-util" when not explicitly set.
pub(crate) fn category_from_recipe(recipe: &RecipeDocument) -> String {
    string_field(recipe, "category")
        .unwrap_or("dev-util")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_eclasses_are_split() {
        assert_eq!(
            inherited_eclasses("inherit cmake xdg"),
            vec!["cmake", "xdg"]
        );
    }

    #[test]
    fn phase_body_extracts_simple_function() {
        let body = phase_body("src_compile() {\n    emake\n}\n", "src_compile");
        assert_eq!(body.map(str::trim), Some("emake"));
    }
}
