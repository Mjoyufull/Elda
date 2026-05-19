use std::path::Path;

use crate::app::PlannedInstallAction;
use crate::app_render_tree::{FrameFooter, TreeStyle, frame_from_sections};
use crate::app_review_memory::{load_review_stamp, review_is_unchanged};

pub(super) fn interbuild_review_lines(
    action: &PlannedInstallAction,
    data_dir: &Path,
) -> Vec<String> {
    let frame = render_interbuild_review_frame(action, data_dir);
    frame.lines().map(str::to_owned).collect()
}

fn render_interbuild_review_frame(action: &PlannedInstallAction, data_dir: &Path) -> String {
    let source_kind = action.resolved.selected_source_kind.as_str();
    let mut identity_lines = vec![
        format!(
            "Identity: {} {}",
            action.package_name,
            recipe_version(action)
        ),
        format!("Lane: {} / {source_kind}", action.resolved.selected_lane),
        "Provenance: [I] parsed, no foreign package-manager CLI".to_owned(),
        format!("Parser: {}", interbuild_parser_name(source_kind)),
    ];
    identity_lines.extend(interbuild_metadata_review_lines(action));
    if let Some(remote) = &action.resolved.remote_name {
        identity_lines.push(format!("Remote: {remote}"));
    }
    if let Some(source_ref) = &action.resolved.source_ref {
        identity_lines.push(format!("Source ref: {source_ref}"));
    }
    identity_lines.push(format!("Recipe: {}", action.resolved.recipe.path.display()));

    let mut risk_lines = Vec::new();
    if action.replaced_packages.is_empty() {
        risk_lines.push("no replacements in planned action".to_owned());
    } else {
        risk_lines.push(format!(
            "replaces {} package(s): {}",
            action.replaced_packages.len(),
            action.replaced_packages.join(", ")
        ));
    }

    let review_memory_lines = interbuild_review_memory_lines(action, data_dir);
    let translation_lines = interbuild_translation_lines(action);
    let logic_lines = interbuild_logic_lines(action);
    let activation_lines = vec![
        "build: parsed foreign source is handed to Elda native build/stage path".to_owned(),
        "stage: payload manifest, object analysis, conffiles, and ownership stay Elda-native"
            .to_owned(),
        "activate: no foreign package-manager transaction is invoked".to_owned(),
    ];

    let title = format!("Interbuild source review for `{}`", action.package_name);
    let footer = FrameFooter {
        glyph: None,
        text: "Proceed? [Y/n/e]".to_owned(),
    };
    let frame = frame_from_sections(
        title,
        &[
            ("Identity".to_owned(), identity_lines),
            ("Review Memory".to_owned(), review_memory_lines),
            ("Translation".to_owned(), translation_lines),
            ("Build Logic".to_owned(), logic_lines),
            ("Risk".to_owned(), risk_lines),
            ("Activation".to_owned(), activation_lines),
        ],
        Some(footer),
    );
    frame.render(TreeStyle::detect())
}

fn interbuild_review_memory_lines(action: &PlannedInstallAction, data_dir: &Path) -> Vec<String> {
    let recipe_path = &action.resolved.recipe.path;
    let stamp = load_review_stamp(data_dir, &action.package_name, "interbuild")
        .ok()
        .flatten();
    let unchanged = review_is_unchanged(data_dir, &action.package_name, "interbuild", recipe_path)
        .unwrap_or(false);

    match stamp {
        Some(stamp) if unchanged => vec![
            "status: current".to_owned(),
            format!("accepted recipe: {}", stamp.recipe_path),
            format!(
                "digest: {}",
                &stamp.recipe_hash[..16.min(stamp.recipe_hash.len())]
            ),
            "pager: skipped because reviewed recipe is unchanged".to_owned(),
        ],
        Some(stamp) => vec![
            "status: changed since last acceptance".to_owned(),
            format!("previous recipe: {}", stamp.recipe_path),
            format!(
                "previous digest: {}",
                &stamp.recipe_hash[..16.min(stamp.recipe_hash.len())]
            ),
            "pager: opens generated metadata before this prompt".to_owned(),
        ],
        None => vec![
            "status: new source review".to_owned(),
            "pager: opens generated metadata before this prompt".to_owned(),
        ],
    }
}

fn interbuild_translation_lines(action: &PlannedInstallAction) -> Vec<String> {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => vec![
            "phase: parsing flake.nix and optional flake.lock".to_owned(),
            "environment: buildInputs/nativeBuildInputs are mapped into Elda dependency families"
                .to_owned(),
            "boundary: no Nix daemon or store activation in this source-build slice".to_owned(),
        ],
        "gentoo_overlay" => vec![
            "phase: locating package ebuild and validating EAPI 8 metadata".to_owned(),
            "environment: DEPEND/RDEPEND/BDEPEND/IUSE are translated into Elda fields".to_owned(),
            "boundary: accepted eclasses and simple phase commands only".to_owned(),
        ],
        "aur_pkgbuild" => vec![
            "phase: parsing PKGBUILD metadata and source/checksum arrays".to_owned(),
            "environment: depends/makedepends/checkdepends/optdepends are preserved for review"
                .to_owned(),
            "boundary: no makepkg or pacman transaction is invoked".to_owned(),
        ],
        "xbps_template" => vec![
            "phase: parsing srcpkgs template variables and distfile/checksum data".to_owned(),
            "environment: hostmakedepends/makedepends/depends/checkdepends are preserved"
                .to_owned(),
            "boundary: no xbps-src transaction is invoked".to_owned(),
        ],
        _ => vec!["phase: interbuild parser boundary".to_owned()],
    }
}

fn interbuild_logic_lines(action: &PlannedInstallAction) -> Vec<String> {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => vec![
            "execution: extract build/install phases where static analysis can prove them".to_owned(),
            "review: pinned inputs and unsupported shell constructs are surfaced before build"
                .to_owned(),
        ],
        "gentoo_overlay" => vec![
            "execution: src_prepare/src_configure/src_compile/src_install map to Elda build.lua phases"
                .to_owned(),
            "review: inherited eclasses and skipped phase functions stay visible".to_owned(),
        ],
        "aur_pkgbuild" => vec![
            "execution: pkgver/build/package functions are accepted only in the bounded shell subset"
                .to_owned(),
            "review: VCS sources, arch sources, provides/conflicts/replaces, and optdepends stay visible"
                .to_owned(),
        ],
        "xbps_template" => vec![
            "execution: do_configure/do_build/do_install and simple v* install macros are bounded"
                .to_owned(),
            "review: build_style, arch filters, provides/conflicts, and skipped functions stay visible"
                .to_owned(),
        ],
        _ => vec!["execution: bounded parser output".to_owned()],
    }
}

fn interbuild_metadata_review_lines(action: &PlannedInstallAction) -> Vec<String> {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => nix_review_lines(action),
        "gentoo_overlay" => gentoo_review_lines(action),
        "aur_pkgbuild" => aur_review_lines(action),
        "xbps_template" => xbps_review_lines(action),
        _ => Vec::new(),
    }
}

fn nix_review_lines(action: &PlannedInstallAction) -> Vec<String> {
    let installable = source_string(action, "installable").unwrap_or("default");
    vec![format!(
        "Metadata: installable={installable}, candidate arch systems derived from pkg.arch"
    )]
}

fn gentoo_review_lines(action: &PlannedInstallAction) -> Vec<String> {
    let package = source_string(action, "package").unwrap_or(&action.package_name);
    vec![format!(
        "Metadata: package={package}, deps/rdeps/bdeps will be parsed from selected ebuild"
    )]
}

fn aur_review_lines(action: &PlannedInstallAction) -> Vec<String> {
    let package = source_string(action, "pkgname").unwrap_or(&action.package_name);
    vec![format!(
        "Metadata: pkgname={package}, arrays include source/checksum/dependency families"
    )]
}

fn xbps_review_lines(action: &PlannedInstallAction) -> Vec<String> {
    let package = source_string(action, "pkgname").unwrap_or(&action.package_name);
    vec![format!(
        "Metadata: pkgname={package}, template variables include distfiles/checksum/dependency families"
    )]
}

fn source_string<'a>(action: &'a PlannedInstallAction, key: &str) -> Option<&'a str> {
    match action.resolved.recipe.package.source.fields.get(key) {
        Some(elda_recipe::ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn recipe_version(action: &PlannedInstallAction) -> String {
    format!(
        "{}:{}-{}",
        action.resolved.recipe.package.epoch,
        action.resolved.recipe.package.version,
        action.resolved.recipe.package.rel
    )
}

fn interbuild_parser_name(source_kind: &str) -> &'static str {
    match source_kind {
        "nix_flake" => "static flake output parser",
        "gentoo_overlay" => "bounded ebuild metadata parser",
        "aur_pkgbuild" => "bounded PKGBUILD metadata parser",
        "xbps_template" => "bounded XBPS template parser",
        _ => "interbuild parser",
    }
}
