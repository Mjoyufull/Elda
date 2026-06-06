mod cargo;
mod static_files;

use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::strategy::SourceStrategy;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuildIntent {
    pub(super) system: String,
    pub(super) bins: Vec<String>,
}

pub(super) fn detect_build_intent(
    source_dir: Option<&Path>,
    recipe_name: &str,
    strategy: &SourceStrategy,
) -> Option<BuildIntent> {
    if strategy.kind() != "git" {
        return None;
    }

    let source_dir = source_dir?;
    cargo::intent(source_dir)
        .or_else(|| static_files::go_intent(source_dir, recipe_name))
        .or_else(|| static_files::meson_intent(source_dir))
        .or_else(|| static_files::cmake_intent(source_dir))
        .or_else(|| static_files::python_intent(source_dir))
        .or_else(|| static_files::nimble_intent(source_dir, recipe_name))
        .or_else(|| static_files::zig_intent(source_dir))
        .or_else(|| static_files::make_intent(source_dir))
}

fn read_toml<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, toml::de::Error> {
    let contents = fs::read_to_string(path).unwrap_or_default();
    toml::from_str(&contents)
}

fn clean_bin_name(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()
        && !value.contains('/')
        && !value.contains('\\')
        && value != "."
        && value != "..")
        .then(|| value.to_owned())
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
