use std::path::Path;

use super::metadata::read_generated_metadata;
use super::model::{ImportOptions, LegacyPkgdep};
use super::render::{render_pkg_lua, render_pkg_lua_with_binary_lane};
use super::strategy::{SourceStrategy, metadata_strategy_for_source, release_binary_strategy};

pub(super) fn render_generated_pkg_lua(
    recipe_name: &str,
    source_url: Option<&str>,
    source_dir: Option<&Path>,
    legacy_pkgdeps: &[LegacyPkgdep],
    recipe_kind: &str,
    selected_strategy: &SourceStrategy,
    options: &ImportOptions,
) -> String {
    let source_strategy = source_lane_strategy(source_dir, selected_strategy, options);
    let metadata_strategy = metadata_strategy_for_source(source_dir, &options.strategy_priority)
        .unwrap_or_else(|| source_strategy.clone());
    let metadata = read_generated_metadata(source_dir, &metadata_strategy);
    let binary_strategy = binary_lane_strategy(source_url, selected_strategy, options);

    if let Some(binary_strategy) = binary_strategy.as_ref() {
        return render_pkg_lua_with_binary_lane(
            recipe_name,
            source_url,
            legacy_pkgdeps,
            recipe_kind,
            &source_strategy,
            Some(binary_strategy),
            default_lane(selected_strategy),
            &metadata,
            options.git_ref.as_ref(),
        );
    }

    render_pkg_lua(
        recipe_name,
        source_url,
        legacy_pkgdeps,
        recipe_kind,
        selected_strategy,
        &metadata,
        options.git_ref.as_ref(),
    )
}

fn source_lane_strategy(
    source_dir: Option<&Path>,
    selected_strategy: &SourceStrategy,
    options: &ImportOptions,
) -> SourceStrategy {
    if !selected_strategy.is_binary_lane() {
        return selected_strategy.clone();
    }

    metadata_strategy_for_source(source_dir, &options.strategy_priority)
        .unwrap_or(SourceStrategy::Git)
}

fn binary_lane_strategy(
    source_url: Option<&str>,
    selected_strategy: &SourceStrategy,
    options: &ImportOptions,
) -> Option<SourceStrategy> {
    if selected_strategy.is_binary_lane() {
        return Some(selected_strategy.clone());
    }

    release_binary_strategy(source_url, &options.release_binary_format_priority)
}

fn default_lane(selected_strategy: &SourceStrategy) -> &'static str {
    if selected_strategy.is_binary_lane() {
        "binary"
    } else {
        "source"
    }
}
