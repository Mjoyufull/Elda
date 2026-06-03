use std::path::Path;

use super::build_intent::detect_build_intent;
use super::metadata::read_generated_metadata;
use super::model::{ImportOptions, LegacyPkgdep};
use super::render::{PkgLuaRender, render_pkg_lua};
use super::strategy::srcinfo_binary_strategy;
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
    let binary_strategy = binary_lane_strategy(source_url, source_dir, selected_strategy, options);
    let build_intent = detect_build_intent(source_dir, recipe_name, &source_strategy);

    if let Some(binary_strategy) = binary_strategy.as_ref() {
        return render_pkg_lua(PkgLuaRender {
            recipe_name,
            source_url,
            legacy_pkgdeps,
            recipe_kind,
            source_strategy: &source_strategy,
            binary_strategy: Some(binary_strategy),
            default_lane: default_lane(selected_strategy, options),
            metadata: &metadata,
            build_intent: build_intent.as_ref(),
            git_ref: options.git_ref.as_ref(),
        });
    }

    render_pkg_lua(PkgLuaRender {
        recipe_name,
        source_url,
        legacy_pkgdeps,
        recipe_kind,
        source_strategy: selected_strategy,
        binary_strategy: None,
        default_lane: "source",
        metadata: &metadata,
        build_intent: build_intent.as_ref(),
        git_ref: options.git_ref.as_ref(),
    })
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
    source_dir: Option<&Path>,
    selected_strategy: &SourceStrategy,
    options: &ImportOptions,
) -> Option<SourceStrategy> {
    if selected_strategy.is_binary_lane() {
        return Some(selected_strategy.clone());
    }

    srcinfo_binary_strategy(source_dir)
        .or_else(|| release_binary_strategy(source_url, &options.release_binary_format_priority))
}

fn default_lane(selected_strategy: &SourceStrategy, options: &ImportOptions) -> &'static str {
    if selected_strategy.is_binary_lane() || options.selected_source_option.is_none() {
        "binary"
    } else {
        "source"
    }
}

#[cfg(test)]
mod tests {
    use super::default_lane;
    use crate::import::{ImportOptions, strategy::SourceStrategy};

    #[test]
    fn generated_dual_lane_recipe_defaults_to_binary_without_explicit_source_choice() {
        assert_eq!(
            default_lane(&SourceStrategy::Git, &ImportOptions::default()),
            "binary"
        );
    }

    #[test]
    fn explicit_source_option_keeps_generated_dual_lane_recipe_on_source() {
        let options = ImportOptions {
            selected_source_option: Some(2),
            ..ImportOptions::default()
        };
        assert_eq!(default_lane(&SourceStrategy::Git, &options), "source");
    }
}
