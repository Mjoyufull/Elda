use std::path::{Path, PathBuf};

use super::detected::DetectedStrategies;
use super::model::SourceOptionReport;
use super::release_options::{ReleaseOption, append_release_option, detect_release_option};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SourceStrategy {
    EldaNative { package: Option<String> },
    NixFlake,
    GentooEbuild { package: String },
    AurPkgbuild,
    XbpsTemplate { package: String },
    GithubRelease(ReleaseOption),
    Git,
}

impl SourceStrategy {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::EldaNative { .. } | Self::Git => "git",
            Self::GithubRelease(option) => option.source_kind(),
            Self::NixFlake => "nix_flake",
            Self::GentooEbuild { .. } => "gentoo_overlay",
            Self::AurPkgbuild => "aur_pkgbuild",
            Self::XbpsTemplate { .. } => "xbps_template",
        }
    }

    pub(super) fn is_binary_lane(&self) -> bool {
        matches!(self, Self::GithubRelease(_))
    }

    pub(super) fn extra_fields(&self) -> String {
        match self {
            Self::EldaNative {
                package: Some(package),
            } => {
                format!("    package = \"{}\",\n", escape(package))
            }
            Self::GentooEbuild { package } => format!("    package = \"{}\",\n", escape(package)),
            Self::XbpsTemplate { package } => format!("    package = \"{}\",\n", escape(package)),
            Self::GithubRelease(option) => option.extra_fields(),
            _ => String::new(),
        }
    }
}

pub(super) fn detect_source_strategy_for_source(
    source_dir: Option<&Path>,
    source_url: Option<&str>,
    strategy_priority: &[String],
    release_binary_format_priority: &[String],
) -> SourceStrategy {
    let detected = source_dir.map(DetectedStrategies::read);
    let priority = effective_priority(strategy_priority);
    for strategy in &priority {
        match normalize_strategy_name(strategy).as_str() {
            "git_release" | "github_release" => {
                if let Some(option) =
                    detect_release_option(source_url, release_binary_format_priority)
                    && option.sha256.is_some()
                {
                    return SourceStrategy::GithubRelease(option);
                }
            }
            _ => {
                if let Some(detected) = &detected
                    && let Some(strategy) = detected.strategy_for(strategy)
                {
                    return strategy;
                }
            }
        }
    }

    detected
        .as_ref()
        .map_or(SourceStrategy::Git, DetectedStrategies::default_strategy)
}

pub(super) fn metadata_strategy_for_source(
    source_dir: Option<&Path>,
    strategy_priority: &[String],
) -> Option<SourceStrategy> {
    let detected = source_dir.map(DetectedStrategies::read)?;
    for strategy in effective_priority(strategy_priority) {
        let normalized = normalize_strategy_name(&strategy);
        if normalized == "git_release" || normalized == "git_source" {
            continue;
        }
        if let Some(strategy) = detected.strategy_for(&strategy) {
            return Some(strategy);
        }
    }

    let fallback = detected.default_strategy();
    (!matches!(fallback, SourceStrategy::Git)).then_some(fallback)
}

pub(super) fn release_binary_strategy(
    source_url: Option<&str>,
    release_binary_format_priority: &[String],
) -> Option<SourceStrategy> {
    detect_release_option(source_url, release_binary_format_priority)
        .filter(|option| option.sha256.is_some())
        .map(SourceStrategy::GithubRelease)
}

pub(super) fn source_options_with_priority(
    source_dir: Option<&Path>,
    source_url: Option<&str>,
    options: &super::model::ImportOptions,
) -> Vec<SourceOptionReport> {
    let mut options_out = Vec::new();
    let detected = source_dir.map(DetectedStrategies::read);
    for strategy in effective_priority(&options.strategy_priority) {
        match normalize_strategy_name(&strategy).as_str() {
            "git_release" | "github_release" => append_release_option(
                source_url,
                &mut options_out,
                &options.release_binary_format_priority,
            ),
            "git_source" if source_url.is_some() => push_option(
                &mut options_out,
                &strategy,
                "git",
                "source",
                "derived",
                "Build from git source",
            ),
            _ => {
                if let Some(detected) = &detected {
                    detected.push_option_for(&strategy, &mut options_out);
                }
            }
        }
    }
    mark_selected_options(&mut options_out);
    options_out
}

pub(super) fn select_source_option_by_index(options: &mut [SourceOptionReport], index: usize) {
    for option in options {
        option.selected = option.index == index;
    }
}

pub(super) fn selected_source_option(options: &[SourceOptionReport]) -> Option<SourceOptionReport> {
    options.iter().find(|option| option.selected).cloned()
}

fn effective_priority(priority: &[String]) -> Vec<String> {
    if priority.is_empty() {
        return [
            "elda-native",
            "nix_flake",
            "gentoo_ebuild",
            "aur_pkgbuild",
            "xbps_template",
            "cargo",
            "cmake",
            "meson",
            "make",
            "go",
            "python",
            "zig",
            "nimble",
            "git_release",
            "git_source",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }

    priority.to_vec()
}

impl DetectedStrategies {
    fn strategy_for(&self, name: &str) -> Option<SourceStrategy> {
        match normalize_strategy_name(name).as_str() {
            "elda_native" if self.elda_native => Some(SourceStrategy::EldaNative { package: None }),
            "nix_flake" if self.nix_flake => Some(SourceStrategy::NixFlake),
            "gentoo_ebuild" => self
                .gentoo_package
                .clone()
                .map(|package| SourceStrategy::GentooEbuild { package }),
            "aur_pkgbuild" if self.aur_pkgbuild => Some(SourceStrategy::AurPkgbuild),
            "xbps_template" if self.xbps_template => {
                Some(SourceStrategy::XbpsTemplate {
                    package: String::new(), // Default to root
                })
            }
            "native_build" if !self.native_builds.is_empty() => Some(SourceStrategy::Git),
            native if self.native_builds.iter().any(|build| build == native) => {
                Some(SourceStrategy::Git)
            }
            _ => None,
        }
    }

    fn default_strategy(&self) -> SourceStrategy {
        if self.elda_native {
            return SourceStrategy::EldaNative { package: None };
        }
        if self.nix_flake {
            return SourceStrategy::NixFlake;
        }
        if let Some(package) = &self.gentoo_package {
            return SourceStrategy::GentooEbuild {
                package: package.clone(),
            };
        }
        if self.aur_pkgbuild {
            return SourceStrategy::AurPkgbuild;
        }
        if self.xbps_template {
            return SourceStrategy::XbpsTemplate {
                package: String::new(),
            };
        }

        SourceStrategy::Git
    }

    fn push_option_for(&self, name: &str, options: &mut Vec<SourceOptionReport>) {
        match normalize_strategy_name(name).as_str() {
            "elda_native" if self.elda_native => push_option(
                options,
                name,
                "git",
                "source",
                "authoritative",
                "Elda-native metadata found",
            ),
            "nix_flake" if self.nix_flake => push_option(
                options,
                name,
                "nix_flake",
                "source",
                "bounded",
                "Nix flake interbuild source detected",
            ),
            "gentoo_ebuild" if self.gentoo_package.is_some() => push_option(
                options,
                name,
                "gentoo_overlay",
                "source",
                "bounded",
                "Gentoo ebuild interbuild source detected",
            ),
            "aur_pkgbuild" if self.aur_pkgbuild => push_option(
                options,
                name,
                "aur_pkgbuild",
                "source",
                "bounded",
                "AUR PKGBUILD interbuild source detected",
            ),
            "xbps_template" if self.xbps_template => push_option(
                options,
                name,
                "xbps_template",
                "source",
                "bounded",
                "XBPS template interbuild source detected",
            ),
            "native_build" if !self.native_builds.is_empty() => push_option(
                options,
                name,
                "git",
                "source",
                "derived",
                "Native build-system source detected",
            ),
            native if self.native_builds.iter().any(|build| build == native) => push_option(
                options,
                native,
                "git",
                "source",
                "derived",
                &format!("Native {native} build source detected"),
            ),
            _ => {}
        }
    }
}

fn push_option(
    options: &mut Vec<SourceOptionReport>,
    strategy: &str,
    source_kind: &str,
    lane: &str,
    confidence: &str,
    summary: &str,
) {
    if options.iter().any(|option| option.strategy == strategy) {
        return;
    }
    options.push(SourceOptionReport {
        index: 0,
        strategy: strategy.to_owned(),
        source_kind: source_kind.to_owned(),
        lane: lane.to_owned(),
        confidence: confidence.to_owned(),
        summary: summary.to_owned(),
        selected: false,
        tag: None,
        asset: None,
        compatibility: None,
        checksum_available: false,
    });
}

pub(super) fn push_release_option(options: &mut Vec<SourceOptionReport>, option: &ReleaseOption) {
    if options
        .iter()
        .any(|option| option.strategy == "git_release")
    {
        return;
    }
    options.push(SourceOptionReport {
        index: 0,
        strategy: "git_release".to_owned(),
        source_kind: option.source_kind().to_owned(),
        lane: "binary".to_owned(),
        confidence: "detected".to_owned(),
        summary: format!(
            "{} release asset detected: {} / {}",
            option.provider, option.tag, option.asset,
        ),
        selected: false,
        tag: Some(option.tag.clone()),
        asset: Some(option.asset.clone()),
        compatibility: Some(option.compatibility.clone()),
        checksum_available: option.sha256.is_some(),
    });
}

fn mark_selected_options(options: &mut [SourceOptionReport]) {
    let selected_index = options
        .iter()
        .position(|option| option.lane != "binary" || option.checksum_available);
    for (index, option) in options.iter_mut().enumerate() {
        option.index = index + 1;
        option.selected = Some(index) == selected_index;
    }
}

fn normalize_strategy_name(name: &str) -> String {
    match name.replace('-', "_").as_str() {
        "elda" | "native" | "elda_native" => "elda_native".to_owned(),
        "nix" | "nix_flake" => "nix_flake".to_owned(),
        "gentoo" | "gentoo_overlay" | "gentoo_ebuild" | "ebuild" => "gentoo_ebuild".to_owned(),
        "aur" | "pkgbuild" | "aur_pkgbuild" => "aur_pkgbuild".to_owned(),
        "xbps" | "xbps_src" | "xbps_template" => "xbps_template".to_owned(),
        "git_release" | "github" | "github_release" | "release" | "release_asset" => {
            "git_release".to_owned()
        }
        "git" | "git_source" | "source_git" => "git_source".to_owned(),
        "native_build" => "native_build".to_owned(),
        "cargo" | "cmake" | "go" | "make" | "meson" | "nimble" | "python" | "zig" => {
            name.replace('-', "_")
        }
        other => other.to_owned(),
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn source_dir_for_detection(input: &str) -> Option<PathBuf> {
    let path = input.strip_prefix("file://").unwrap_or(input);
    let path = Path::new(path);
    path.is_dir().then(|| path.to_path_buf())
}
