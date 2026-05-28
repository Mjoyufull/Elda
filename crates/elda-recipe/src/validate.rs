mod metadata;
mod source;

use std::collections::BTreeSet;

use crate::model::{
    DependencyBody, DependencyEntry, IssueSeverity, LuaValue, PackageDefinition, RecipeDocument,
    ValidationIssue,
};
use elda_types::NamedConstraint;

const CANONICAL_ARCHES: &[&str] = &["amd64", "i386", "arm64", "armhf", "riscv64", "ppc64le"];
const PACKAGE_KINDS: &[&str] = &["normal", "meta", "profile"];
const SOURCE_KINDS: &[&str] = &[
    "url_archive",
    "github_release",
    "release_asset",
    "appimage",
    "git",
    "nix_flake",
    "gentoo_overlay",
    "aur_pkgbuild",
    "xbps_template",
];

pub fn validate_recipe(document: &RecipeDocument) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    validate_package(&document.package, &mut issues);
    issues
}

fn validate_package(package: &PackageDefinition, issues: &mut Vec<ValidationIssue>) {
    if package.name.trim().is_empty() {
        issues.push(error("pkg.name must not be empty"));
    }
    if package.version.trim().is_empty() {
        issues.push(error("pkg.version must not be empty"));
    }
    if package.arch.is_empty() {
        issues.push(error(
            "pkg.arch must contain at least one canonical architecture label",
        ));
    }

    let mut seen_arches = BTreeSet::new();
    for arch in &package.arch {
        if !CANONICAL_ARCHES.contains(&arch.as_str()) {
            issues.push(error(format!(
                "pkg.arch contains unsupported architecture label `{arch}`"
            )));
        }
        if !seen_arches.insert(arch) {
            issues.push(warning(format!(
                "pkg.arch contains duplicate entry `{arch}`"
            )));
        }
    }

    if !PACKAGE_KINDS.contains(&package.kind.as_str()) {
        issues.push(error(format!(
            "pkg.kind must be one of: {}",
            PACKAGE_KINDS.join(", ")
        )));
    }

    source::validate_source(package, issues, CANONICAL_ARCHES, SOURCE_KINDS);
    let allowed_flags = collect_allowed_flag_names(package);
    validate_dependencies("depends", &package.depends, &allowed_flags, issues);
    validate_dependencies("makedepends", &package.makedepends, &allowed_flags, issues);
    validate_dependencies(
        "checkdepends",
        &package.checkdepends,
        &allowed_flags,
        issues,
    );
    validate_dependencies("recommends", &package.recommends, &allowed_flags, issues);
    validate_dependencies("suggests", &package.suggests, &allowed_flags, issues);
    validate_dependencies("supplements", &package.supplements, &allowed_flags, issues);
    validate_dependencies("enhances", &package.enhances, &allowed_flags, issues);
    validate_provides(&package.provides, issues);
    validate_named_constraints("conflicts", &package.conflicts, issues);
    validate_named_constraints("replaces", &package.replaces, issues);
    metadata::validate_metadata(package, issues);
    validate_profile_policy(package, issues);
    validate_build(package, issues);

    for conffile in &package.conffiles {
        if !conffile.starts_with("/etc/") {
            issues.push(error(format!(
                "conffile `{conffile}` must be an absolute path under /etc"
            )));
        }
    }
}

fn validate_profile_policy(package: &PackageDefinition, issues: &mut Vec<ValidationIssue>) {
    let Some(profile) = &package.profile else {
        return;
    };

    if package.kind != "profile" {
        issues.push(error(
            "pkg.profile metadata is allowed only when pkg.kind = `profile`",
        ));
    }

    if profile
        .native_arch
        .as_deref()
        .is_some_and(|arch| arch.trim().is_empty())
    {
        issues.push(error("profile.native_arch must not be empty"));
    }
    if let Some(native_arch) = &profile.native_arch
        && !CANONICAL_ARCHES.contains(&native_arch.as_str())
    {
        issues.push(error(format!(
            "profile.native_arch must use a canonical Elda architecture label, got `{native_arch}`"
        )));
    }
    if profile
        .init
        .as_deref()
        .is_some_and(|provider| provider.trim().is_empty())
    {
        issues.push(error("profile.init must not be empty"));
    }

    let mut seen_arches = BTreeSet::new();
    for arch in &profile.foreign_arches {
        if arch.trim().is_empty() {
            issues.push(error(
                "profile.foreign_arches must not contain empty architecture labels",
            ));
            continue;
        }
        if !CANONICAL_ARCHES.contains(&arch.as_str()) {
            issues.push(error(format!(
                "profile.foreign_arches contains unsupported architecture label `{arch}`"
            )));
        }
        if !seen_arches.insert(arch) {
            issues.push(warning(format!(
                "profile.foreign_arches contains duplicate entry `{arch}`"
            )));
        }
    }
}

const IMPLEMENTED_BUILD_SYSTEMS: &[&str] = &[
    "cargo", "cmake", "go", "make", "meson", "nim", "nimble", "python", "zig",
];

fn validate_build(package: &PackageDefinition, issues: &mut Vec<ValidationIssue>) {
    let Some(build) = &package.build else {
        return;
    };

    if build.system.trim().is_empty() {
        issues.push(error("build.system must not be empty"));
        return;
    }

    if !IMPLEMENTED_BUILD_SYSTEMS.contains(&build.system.as_str()) {
        issues.push(warning(format!(
            "build.system `{}` is not implemented by the current execution slice yet",
            build.system
        )));
    }

    if build.bins.iter().any(|bin| bin.trim().is_empty()) {
        issues.push(error("build.bins must not contain empty binary names"));
    }
    if build
        .features
        .iter()
        .any(|feature| feature.trim().is_empty())
    {
        issues.push(error("build.features must not contain empty feature names"));
    }
}

fn validate_dependencies(
    field: &str,
    entries: &[DependencyEntry],
    allowed_flags: &Option<BTreeSet<String>>,
    issues: &mut Vec<ValidationIssue>,
) {
    for entry in entries {
        match &entry.body {
            DependencyBody::Constraint(value) => {
                if value.trim().is_empty() {
                    issues.push(error(format!(
                        "{field} contains an empty dependency constraint"
                    )));
                    continue;
                }
                if let Err(parse_error) = NamedConstraint::parse_dependency(value) {
                    issues.push(error(format!(
                        "{field} contains invalid dependency constraint `{value}`: {parse_error}"
                    )));
                }
            }
            DependencyBody::AnyOf(providers) => {
                if providers.is_empty() {
                    issues.push(error(format!(
                        "{field} contains an empty any-of provider table"
                    )));
                }
                if providers.iter().any(|provider| provider.trim().is_empty()) {
                    issues.push(error(format!(
                        "{field} contains an empty provider in an any-of table"
                    )));
                }
                for provider in providers {
                    if provider.trim().is_empty() {
                        continue;
                    }
                    if let Err(parse_error) = NamedConstraint::parse_dependency(provider) {
                        issues.push(error(format!(
                            "{field} contains invalid any-of entry `{provider}`: {parse_error}"
                        )));
                    }
                }
            }
        }
        if let (Some(predicate), Some(allowed)) = (entry.when.as_ref(), allowed_flags.as_ref()) {
            for atom in &predicate.atoms {
                if !allowed.contains(&atom.flag) {
                    issues.push(error(format!(
                        "{field} `when` predicate references undeclared flag `{}`",
                        atom.flag
                    )));
                }
            }
        }
    }
}

fn collect_allowed_flag_names(package: &PackageDefinition) -> Option<BTreeSet<String>> {
    let mut allowed = BTreeSet::new();
    let mut populated = false;

    for source in [
        package.flags_default.as_ref(),
        package.flags_allowed.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let LuaValue::Table(table) = source {
            populated = true;
            for flag in table.keys() {
                allowed.insert(flag.clone());
            }
        }
    }

    for source in [
        package.flags_implies.as_ref(),
        package.flags_conflicts.as_ref(),
        package.flags_required_one_of.as_ref(),
        package.flags_required_at_most_one.as_ref(),
        package.flags_required_any_of.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if let LuaValue::Table(table) = source {
            populated = true;
            for (group, entry) in table {
                allowed.insert(group.clone());
                if let LuaValue::Array(entries) = entry {
                    for member in entries {
                        if let LuaValue::String(name) = member {
                            allowed.insert(name.clone());
                        }
                    }
                }
            }
        }
    }

    populated.then_some(allowed)
}

fn validate_provides(provides: &[String], issues: &mut Vec<ValidationIssue>) {
    for provide in provides {
        if provide.trim().is_empty() {
            issues.push(error("provides contains an empty entry"));
            continue;
        }
        if let Err(parse_error) = NamedConstraint::parse_provide(provide) {
            issues.push(error(format!(
                "provides contains invalid provide `{provide}`: {parse_error}"
            )));
        }
    }
}

fn validate_named_constraints(field: &str, values: &[String], issues: &mut Vec<ValidationIssue>) {
    for value in values {
        if value.trim().is_empty() {
            issues.push(error(format!("{field} contains an empty constraint")));
            continue;
        }
        if let Err(parse_error) = NamedConstraint::parse_dependency(value) {
            issues.push(error(format!(
                "{field} contains invalid constraint `{value}`: {parse_error}"
            )));
        }
    }
}

fn error(message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: IssueSeverity::Error,
        message: message.into(),
    }
}

fn warning(message: impl Into<String>) -> ValidationIssue {
    ValidationIssue {
        severity: IssueSeverity::Warning,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
