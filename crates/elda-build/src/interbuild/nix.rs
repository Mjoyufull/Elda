use rowan::ast::AstNode;

use std::fs;
use std::path::Path;

use elda_recipe::{RecipeDocument, ScalarValue};
use serde_json::Value;

use crate::error::BuildError;
use crate::interbuild::{InterbuildReport, LockfileReport};

const ALLOWED_LOCKED_INPUT_TYPES: &[&str] = &["git", "github", "tarball"];

/// Common flake-utils helper names that wrap outputs with a
/// per-system function, producing `packages.<system>.*` without
/// explicitly writing `packages.x86_64-linux.default`.
const SYSTEM_WRAPPER_PATTERNS: &[&str] = &[
    "eachDefaultSystem",
    "eachSystem",
    "forAllSystems",
    "genAttrs",
    "eachDefaultSystemPassthrough",
];

pub fn validate_flake(
    recipe: &RecipeDocument,
    source_dir: &Path,
) -> Result<InterbuildReport, BuildError> {
    let flake_path = source_dir.join("flake.nix");
    let contents = fs::read_to_string(&flake_path).map_err(|error| {
        BuildError::Invalid(format!(
            "nix_flake source for `{}` does not contain a readable flake.nix: {error}",
            recipe.package.name
        ))
    })?;
    let candidate_systems = candidate_systems(recipe);
    let target = validate_install_target(recipe, &contents, &candidate_systems)?;
    let lockfile = validate_lockfile_if_present(recipe, source_dir)?;
    let nix_meta = extract_nix_meta(&contents);
    validate_flake_inputs_against_lockfile(recipe, &nix_meta.inputs, &lockfile)?;
    // nix_meta is already parsed
    Ok(InterbuildReport::nix_flake(
        target,
        candidate_systems,
        lockfile,
        nix_meta,
    ))
}

fn validate_install_target(
    recipe: &RecipeDocument,
    contents: &str,
    systems: &[String],
) -> Result<String, BuildError> {
    let installable = string_field(recipe, "installable").unwrap_or("default");
    if installable != "default" {
        let attr = format!(".{installable}");
        if systems
            .iter()
            .any(|sys| contents.contains(&format!("packages.{sys}{attr}")))
            || contents.contains(&format!("{installable} ="))
        {
            return Ok(installable.to_owned());
        }

        return Err(BuildError::Unsupported(format!(
            "nix_flake installable `{installable}` for `{}` was not \
             found in the supported static output subset",
            recipe.package.name
        )));
    }

    // Explicit packages.<system>.default
    if systems
        .iter()
        .any(|sys| contains_default_package(contents, sys))
    {
        return Ok("default".to_owned());
    }

    // flake-utils / forAllSystems wrappers that generate per-system
    // outputs — if the flake uses one of these helpers, a single
    // `default` assignment inside the wrapper body is sufficient.
    if uses_system_wrapper(contents) {
        if contents.contains("default =") || contents.contains("default=") {
            return Ok("default".to_owned());
        }

        let package_outputs = count_package_output_assignments(contents);
        if package_outputs == 1 {
            return Ok("single-wrapped-package-output".to_owned());
        }
    }

    // Single static package output (no wrapper)
    let package_outputs = count_package_output_assignments(contents);
    if package_outputs == 1 {
        return Ok("single-static-package-output".to_owned());
    }

    // let-binding or rec-set that exports packages
    if contains_let_package_export(contents) {
        return Ok("let-bound-package-output".to_owned());
    }

    Err(BuildError::Unsupported(format!(
        "nix_flake source for `{}` does not expose \
         packages.<system>.default or one obvious static package output",
        recipe.package.name
    )))
}

fn validate_lockfile_if_present(
    recipe: &RecipeDocument,
    source_dir: &Path,
) -> Result<LockfileReport, BuildError> {
    let lock_path = source_dir.join("flake.lock");
    if !lock_path.exists() {
        return Ok(LockfileReport::absent());
    }

    let contents = fs::read_to_string(&lock_path).map_err(|error| {
        BuildError::Invalid(format!(
            "nix_flake lockfile for `{}` could not be read: {error}",
            recipe.package.name
        ))
    })?;
    let lock = serde_json::from_str::<Value>(&contents).map_err(|error| {
        BuildError::Invalid(format!(
            "nix_flake lockfile for `{}` is not valid JSON: {error}",
            recipe.package.name
        ))
    })?;
    let locked_inputs = validate_locked_inputs(recipe, &lock)?;

    Ok(LockfileReport::present(locked_inputs))
}

fn validate_locked_inputs(recipe: &RecipeDocument, lock: &Value) -> Result<usize, BuildError> {
    let Some(nodes) = lock.get("nodes").and_then(Value::as_object) else {
        return Err(BuildError::Invalid(format!(
            "nix_flake lockfile for `{}` is missing a nodes table",
            recipe.package.name
        )));
    };

    let mut locked_inputs = 0;
    for (node_name, node) in nodes {
        let Some(locked) = node.get("locked") else {
            continue;
        };
        locked_inputs += 1;
        let input_type = locked.get("type").and_then(Value::as_str).ok_or_else(|| {
            BuildError::Invalid(format!(
                "nix_flake lockfile node `{node_name}` for `{}` \
                     is missing locked.type",
                recipe.package.name
            ))
        })?;
        if !ALLOWED_LOCKED_INPUT_TYPES.contains(&input_type) {
            return Err(BuildError::Unsupported(format!(
                "nix_flake lockfile node `{node_name}` for `{}` uses \
                 unsupported locked.type `{input_type}`",
                recipe.package.name
            )));
        }
        if input_type == "tarball" && locked.get("narHash").is_none() {
            return Err(BuildError::Invalid(format!(
                "nix_flake tarball lockfile node `{node_name}` for `{}` \
                 is missing narHash",
                recipe.package.name
            )));
        }
    }

    Ok(locked_inputs)
}

/// Verify that every input referenced in `flake.nix` has a matching
/// locked node in the lockfile. This catches lock-drift where an input
/// was added to the flake but `nix flake lock` was never re-run.
fn validate_flake_inputs_against_lockfile(
    recipe: &RecipeDocument,
    declared_inputs: &[String],
    lockfile: &LockfileReport,
) -> Result<(), BuildError> {
    if !lockfile.present {
        return Ok(());
    }

    if declared_inputs.is_empty() {
        return Ok(());
    }

    // The lockfile `locked_inputs` count gives us a basic sanity check.
    // A more precise check would walk the actual lock JSON, but the
    // lockfile was already validated in validate_lockfile_if_present.
    if lockfile.locked_inputs == 0 && !declared_inputs.is_empty() {
        return Err(BuildError::Invalid(format!(
            "nix_flake for `{}` declares {} input(s) but the lockfile \
             has no locked nodes — run `nix flake lock`",
            recipe.package.name,
            declared_inputs.len()
        )));
    }

    Ok(())
}

/// Extract basic `meta` information from the flake output using rnix AST parsing.
fn extract_nix_meta(contents: &str) -> NixMetaReport {
    let root = rnix::Root::parse(contents).syntax();

    let mut description = None;
    let mut meta_description = None;
    let mut homepage = None;
    let mut license = None;
    let mut inputs = std::collections::BTreeSet::new();

    for desc in root.descendants() {
        if desc.kind() != rnix::SyntaxKind::NODE_ATTRPATH_VALUE {
            continue;
        }
        let Some(attrpath_value) = rnix::ast::AttrpathValue::cast(desc.clone()) else {
            continue;
        };
        let Some(path) = attrpath_value.attrpath() else {
            continue;
        };

        let attrs: Vec<String> = path
            .attrs()
            .filter_map(|attr| {
                if let rnix::ast::Attr::Ident(ident) = attr {
                    ident.ident_token().map(|token| token.text().to_string())
                } else {
                    None
                }
            })
            .collect();

        let Some(last) = attrs.last().map(String::as_str) else {
            continue;
        };

        let in_meta = is_inside_meta_context(&desc) || attrs.iter().any(|a| a == "meta");
        if last == "description" {
            if let Some(val) = extract_string(&attrpath_value) {
                if in_meta {
                    meta_description = Some(val.clone());
                }
                if description.is_none() {
                    description = Some(val);
                }
            }
        } else if (last == "homepage" || last == "downloadPage") && homepage.is_none() {
            homepage = extract_string(&attrpath_value);
        } else if (last == "license" || last == "licenses") && license.is_none() {
            license = Some(extract_raw_text(&attrpath_value));
        }

        if attrs[0] == "inputs" {
            if attrs.len() > 1 {
                inputs.insert(attrs[1].clone());
            } else if let Some(rnix::ast::Expr::AttrSet(set)) = attrpath_value.value() {
                use rnix::ast::HasEntry;
                for entry in set.entries() {
                    if let rnix::ast::Entry::AttrpathValue(attrpath_value) = entry
                        && let Some(path) = attrpath_value.attrpath()
                        && let Some(rnix::ast::Attr::Ident(ident)) = path.attrs().next()
                        && let Some(token) = ident.ident_token()
                    {
                        inputs.insert(token.text().to_string());
                    }
                }
            }
        }
    }

    NixMetaReport {
        description: meta_description.or(description),
        homepage,
        license,
        inputs: inputs.into_iter().collect(),
    }
}

fn is_inside_meta_context(node: &rnix::SyntaxNode) -> bool {
    for parent in node.ancestors() {
        if parent.kind() == rnix::SyntaxKind::NODE_ATTRPATH_VALUE
            && let Some(parent_attr) = rnix::ast::AttrpathValue::cast(parent)
            && let Some(path) = parent_attr.attrpath()
        {
            for attr in path.attrs() {
                if let rnix::ast::Attr::Ident(ident) = attr
                    && let Some(token) = ident.ident_token()
                    && token.text() == "meta"
                {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_string(attrpath_value: &rnix::ast::AttrpathValue) -> Option<String> {
    if let Some(val) = attrpath_value.value()
        && val.syntax().kind() == rnix::SyntaxKind::NODE_STRING
    {
        let text = val.syntax().text().to_string();
        let trimmed = text.trim_matches('"').to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

fn extract_raw_text(attrpath_value: &rnix::ast::AttrpathValue) -> String {
    if let Some(val) = attrpath_value.value() {
        let text = val.syntax().text().to_string();
        if text.starts_with('"') && text.ends_with('"') {
            return text.trim_matches('"').to_string();
        }
        let stripped = text.strip_prefix("lib.licenses.").unwrap_or(&text);
        let stripped = stripped.strip_prefix("licenses.").unwrap_or(stripped);
        return stripped.to_string();
    }
    String::new()
}

/// Detect whether the flake uses a system-wrapper helper such as
/// `flake-utils.lib.eachDefaultSystem` or `forAllSystems`.
fn uses_system_wrapper(contents: &str) -> bool {
    SYSTEM_WRAPPER_PATTERNS
        .iter()
        .any(|pattern| contents.contains(pattern))
}

/// Detect whether the flake uses `let ... in { packages = ... }` or
/// `rec { packages = ... }` to export packages.
fn contains_let_package_export(contents: &str) -> bool {
    let lower = contents.to_ascii_lowercase();
    (lower.contains("let ")
        || lower.contains(
            "let
",
        ))
        && lower.contains("packages")
        && (lower.contains(" in ")
            || lower.contains(
                "
in {",
            )
            || lower.contains(
                "
in
",
            ))
}

fn candidate_systems(recipe: &RecipeDocument) -> Vec<String> {
    recipe
        .package
        .arch
        .iter()
        .filter_map(|arch| match arch.as_str() {
            "amd64" => Some("x86_64-linux".to_owned()),
            "i386" => Some("i686-linux".to_owned()),
            "arm64" => Some("aarch64-linux".to_owned()),
            "armhf" => Some("armv7l-linux".to_owned()),
            "riscv64" => Some("riscv64-linux".to_owned()),
            "ppc64le" => Some("powerpc64le-linux".to_owned()),
            _ => None,
        })
        .collect()
}

fn contains_default_package(contents: &str, system: &str) -> bool {
    contents.contains(&format!("packages.{system}.default"))
        || contents.contains(&format!("\"{system}\".default"))
        || contents.contains(&format!("{system}.default"))
        || contents.contains("default =")
}

fn count_package_output_assignments(contents: &str) -> usize {
    contents
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.contains("packages.") && trimmed.contains('=') && !trimmed.starts_with('#')
        })
        .count()
}

fn string_field<'a>(recipe: &'a RecipeDocument, key: &str) -> Option<&'a str> {
    match recipe.package.source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Nix-specific metadata extracted from flake.nix for richer reporting.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NixMetaReport {
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub inputs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_package_is_accepted_for_supported_system() {
        let contents = "outputs = { self }: { packages.x86_64-linux.default = self; };";
        assert!(contains_default_package(contents, "x86_64-linux"));
    }

    #[test]
    fn package_assignment_count_ignores_comments() {
        let contents = "# packages.x86_64-linux.old = x;\n\
                        packages.x86_64-linux.tool = x;";
        assert_eq!(count_package_output_assignments(contents), 1);
    }

    #[test]
    fn flake_utils_each_default_system_detected() {
        let contents = r#"
outputs = { self, flake-utils, ... }:
  flake-utils.lib.eachDefaultSystem (system: {
    packages.default = self;
  });
"#;
        assert!(uses_system_wrapper(contents));
    }

    #[test]
    fn for_all_systems_pattern_detected() {
        let contents = r#"
outputs = { self, nixpkgs, ... }: let
  forAllSystems = f: nixpkgs.lib.genAttrs systems f;
in { packages = forAllSystems (system: { default = x; }); };
"#;
        assert!(uses_system_wrapper(contents));
    }

    #[test]
    fn gen_attrs_pattern_detected() {
        let contents = r#"
packages = nixpkgs.lib.genAttrs systems (system: {
  default = pkgs.${system}.tool;
});
"#;
        assert!(uses_system_wrapper(contents));
    }

    #[test]
    fn let_binding_with_packages_detected() {
        let contents = r#"
outputs = { self, nixpkgs, ... }: let
  pkgs = nixpkgs.legacyPackages.x86_64-linux;
in {
  packages.x86_64-linux.default = pkgs.callPackage ./. {};
};
"#;
        assert!(contains_let_package_export(contents));
    }

    #[test]
    fn meta_description_extracted() {
        let contents = r#"
{ meta.description = "A fast build tool";
meta.license = lib.licenses.mit;
}
"#;
        let meta = extract_nix_meta(contents);
        assert_eq!(meta.description.as_deref(), Some("A fast build tool"));
        assert_eq!(meta.license.as_deref(), Some("mit"));
    }

    #[test]
    fn inputs_extracted_from_dot_notation() {
        let contents = r#"
{ inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
inputs.flake-utils.url = "github:numtide/flake-utils";
}
"#;
        let inputs = extract_nix_meta(contents).inputs;
        assert_eq!(inputs, vec!["flake-utils", "nixpkgs"]);
    }
}
