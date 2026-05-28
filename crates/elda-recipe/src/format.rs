//! Canonical `pkg.lua` formatting from parsed recipe documents.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::RecipeError;
use crate::model::{
    DependencyBody, DependencyEntry, GitHubReleaseAssetDefinition, PackageDefinition, ScalarValue,
    SourceDefinition, SourceLaneDefinition,
};
use crate::validate::validate_recipe;

pub fn format_recipe_file(path: &Path) -> Result<String, RecipeError> {
    let content = fs::read_to_string(path)?;
    let document = crate::parse_pkg_lua(path, &content)?;
    Ok(render_pkg_lua(&document.package))
}

pub fn normalize_recipe_file(path: &Path) -> Result<String, RecipeError> {
    let content = fs::read_to_string(path)?;
    let document = crate::parse_pkg_lua(path, &content)?;
    let issues = validate_recipe(&document);
    if issues
        .iter()
        .any(|issue| issue.severity == crate::model::IssueSeverity::Error)
    {
        return Err(RecipeError::InvalidInput(
            "recipe has validation errors; run `elda rc check` first".to_owned(),
        ));
    }
    Ok(render_pkg_lua(&document.package))
}

pub fn write_formatted_recipe(path: &Path, content: &str) -> Result<(), RecipeError> {
    fs::write(path, content)?;
    Ok(())
}

pub fn render_pkg_lua(package: &PackageDefinition) -> String {
    let mut out = String::from("pkg = {\n");
    out.push_str(&format!("    name = {},\n", lua_string(&package.name)));
    if let Some(description) = &package.description {
        out.push_str(&format!("    description = {},\n", lua_string(description)));
    }
    if !package.licenses.is_empty() {
        out.push_str("    licenses = {\n");
        for license in &package.licenses {
            out.push_str(&format!("        {},\n", lua_string(license)));
        }
        out.push_str("    },\n");
    }
    if let Some(upstream) = &package.upstream {
        out.push_str(&format!("    upstream = {},\n", lua_string(upstream)));
    }
    if package.epoch > 0 {
        out.push_str(&format!("    epoch = {},\n", package.epoch));
    }
    out.push_str(&format!(
        "    version = {},\n",
        lua_string(&package.version)
    ));
    out.push_str(&format!("    rel = {},\n", package.rel));
    if !package.arch.is_empty() {
        out.push_str("    arch = {\n");
        for arch in &package.arch {
            out.push_str(&format!("        {},\n", lua_string(arch)));
        }
        out.push_str("    },\n");
    }
    out.push_str(&format!("    kind = {},\n", lua_string(&package.kind)));
    out.push_str("    source = ");
    out.push_str(&render_source_definition(&package.source));
    out.push_str(",\n");
    render_dependency_list(&mut out, "depends", &package.depends);
    render_dependency_list(&mut out, "makedepends", &package.makedepends);
    render_string_list(&mut out, "provides", &package.provides);
    render_string_list(&mut out, "conflicts", &package.conflicts);
    render_string_list(&mut out, "replaces", &package.replaces);
    render_string_list(&mut out, "conffiles", &package.conffiles);
    if let Some(build) = &package.build {
        out.push_str("    build = {\n");
        out.push_str(&format!(
            "        system = {},\n",
            lua_string(&build.system)
        ));
        if !build.bins.is_empty() {
            out.push_str("        bins = {\n");
            for bin in &build.bins {
                out.push_str(&format!("            {},\n", lua_string(bin)));
            }
            out.push_str("        },\n");
        }
        out.push_str(&format!("        tests = {},\n", build.tests));
        out.push_str("    },\n");
    }
    out.push_str("}\n");
    out
}

fn render_source_definition(source: &SourceDefinition) -> String {
    if source.lanes.is_empty() {
        return render_lane_block(&SourceLaneDefinition {
            kind: source.kind.clone(),
            fields: source.fields.clone(),
            github_release_assets: source.github_release_assets.clone(),
        });
    }
    let mut out = String::from("{\n");
    if let Some(default_lane) = &source.default_lane {
        out.push_str(&format!(
            "        default_lane = {},\n",
            lua_string(default_lane)
        ));
    }
    out.push_str("        lanes = {\n");
    for (name, lane) in &source.lanes {
        out.push_str(&format!("            {} = ", lua_string(name)));
        out.push_str(&render_lane_block(lane));
        out.push_str(",\n");
    }
    out.push_str("        },\n    }");
    out
}

fn render_lane_block(lane: &SourceLaneDefinition) -> String {
    let mut out = String::from("{\n");
    out.push_str(&format!("            kind = {},\n", lua_string(&lane.kind)));
    for (key, value) in &lane.fields {
        out.push_str(&format!(
            "            {} = {},\n",
            key,
            render_scalar(value)
        ));
    }
    if !lane.github_release_assets.is_empty() {
        out.push_str(&render_release_assets(&lane.github_release_assets));
    }
    out.push_str("        }");
    out
}

fn render_release_assets(assets: &BTreeMap<String, GitHubReleaseAssetDefinition>) -> String {
    let mut out = String::from("            github_release_assets = {\n");
    for (arch, asset) in assets {
        out.push_str(&format!("                {} = {{\n", lua_string(arch)));
        out.push_str(&format!(
            "                    asset = {},\n",
            lua_string(&asset.asset)
        ));
        out.push_str(&format!(
            "                    sha256 = {},\n",
            lua_string(&asset.sha256)
        ));
        if let Some(signature) = &asset.signature {
            out.push_str(&format!(
                "                    signature = {},\n",
                lua_string(signature)
            ));
        }
        out.push_str("                },\n");
    }
    out.push_str("            },\n");
    out
}

fn render_dependency_list(out: &mut String, key: &str, entries: &[DependencyEntry]) {
    if entries.is_empty() {
        return;
    }
    out.push_str(&format!("    {key} = {{\n"));
    for entry in entries {
        out.push_str(&format!("        {},\n", render_dependency(entry)));
    }
    out.push_str("    },\n");
}

fn render_string_list(out: &mut String, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    out.push_str(&format!("    {key} = {{\n"));
    for value in values {
        out.push_str(&format!("        {},\n", lua_string(value)));
    }
    out.push_str("    },\n");
}

fn render_scalar(value: &ScalarValue) -> String {
    match value {
        ScalarValue::String(text) => lua_string(text),
        ScalarValue::Integer(number) => number.to_string(),
        ScalarValue::Boolean(flag) => flag.to_string(),
    }
}

fn render_dependency(entry: &DependencyEntry) -> String {
    match &entry.body {
        DependencyBody::Constraint(value) => lua_string(value),
        DependencyBody::AnyOf(providers) => {
            let inner = providers
                .iter()
                .map(|provider| lua_string(provider))
                .collect::<Vec<_>>()
                .join(", ");
            format!("any = {{ {inner} }}")
        }
    }
}

fn lua_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests;
