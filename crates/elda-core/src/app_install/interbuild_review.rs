use std::path::Path;

use crate::app::PlannedInstallAction;
use crate::app_render_tree::{Frame, FrameFooter, TreeStyle};
use crate::app_review_memory::{load_review_stamp, review_is_unchanged};
use crate::render_style::highlight_operator_frame;

pub(super) fn interbuild_review_lines(
    action: &PlannedInstallAction,
    data_dir: &Path,
) -> Vec<String> {
    render_interbuild_review_frame(action, data_dir)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn render_interbuild_review_frame(action: &PlannedInstallAction, data_dir: &Path) -> String {
    let source_kind = action.resolved.selected_source_kind.as_str();
    let title = format!("Interbuild source review for `{}`", action.package_name);
    let mut frame = Frame::new(title);

    frame.section("Identity");
    frame.kv("package", &action.package_name);
    frame.kv("version", &recipe_version(action));
    frame.kv(
        "lane",
        &format!("{} / {source_kind}", action.resolved.selected_lane),
    );
    frame.kv("provenance", "[I] parsed, no foreign package-manager CLI");
    frame.kv("parser", interbuild_parser_name(source_kind));
    push_interbuild_identity_rows(&mut frame, action, source_kind);
    if let Some(remote) = &action.resolved.remote_name {
        frame.kv("remote", remote);
    }
    if let Some(source_ref) = &action.resolved.source_ref {
        frame.kv("source ref", source_ref);
    }
    frame.kv("recipe", &action.resolved.recipe.path.display().to_string());

    frame.spacer();
    frame.section("Review Memory");
    for line in interbuild_review_memory_lines(action, data_dir) {
        push_detail_row(&mut frame, &line);
    }

    frame.spacer();
    frame.section("Translation");
    for line in interbuild_translation_lines(action) {
        push_detail_row(&mut frame, &line);
    }

    frame.spacer();
    frame.section("Build Logic");
    for line in interbuild_logic_lines(action) {
        push_detail_row(&mut frame, &line);
    }

    frame.spacer();
    frame.section("Risk");
    if action.replaced_packages.is_empty() {
        push_detail_row(&mut frame, "no replacements in planned action");
    } else {
        push_detail_row(
            &mut frame,
            &format!(
                "replaces {} package(s): {}",
                action.replaced_packages.len(),
                action.replaced_packages.join(", ")
            ),
        );
    }

    frame.spacer();
    frame.section("Activation");
    push_detail_row(
        &mut frame,
        "build:: parsed foreign source is handed to Elda native build/stage path",
    );
    push_detail_row(
        &mut frame,
        "stage:: payload manifest, object analysis, conffiles, and ownership stay Elda-native",
    );
    push_detail_row(
        &mut frame,
        "activate:: no foreign package-manager transaction is invoked",
    );

    frame.footer(FrameFooter {
        glyph: None,
        text: "Proceed? [Y/n/e]".to_owned(),
    });

    highlight_operator_frame(&frame.render(TreeStyle::detect()))
}

fn push_detail_row(frame: &mut Frame, line: &str) {
    if let Some((key, value)) = line.split_once(":: ") {
        frame.kv(key, value);
    } else if let Some((key, value)) = line.split_once(": ") {
        frame.kv(key, value);
    } else {
        frame.line(line.to_owned());
    }
}

fn push_interbuild_identity_rows(
    frame: &mut Frame,
    action: &PlannedInstallAction,
    source_kind: &str,
) {
    match source_kind {
        "gentoo_overlay" => {
            let atom = source_string(action, "package").unwrap_or(&action.package_name);
            frame.kv("atom", atom);
            if let Some(ebuild) = find_ebuild_name(action) {
                frame.kv("ebuild", &ebuild);
            }
            frame.kv(
                "metadata",
                "DEPEND/RDEPEND/BDEPEND/IUSE parsed from selected ebuild",
            );
        }
        "nix_flake" => {
            let installable = source_string(action, "installable").unwrap_or("default");
            frame.kv("installable", installable);
        }
        "aur_pkgbuild" => {
            let pkgname = source_string(action, "pkgname").unwrap_or(&action.package_name);
            frame.kv("pkgname", pkgname);
            frame.kv(
                "metadata",
                "source/checksum/dependency arrays preserved for review",
            );
        }
        "xbps_template" => {
            let pkgname = source_string(action, "pkgname").unwrap_or(&action.package_name);
            frame.kv("pkgname", pkgname);
            frame.kv(
                "metadata",
                "distfiles/checksum/dependency families preserved for review",
            );
        }
        _ => {}
    }
}

fn find_ebuild_name(action: &PlannedInstallAction) -> Option<String> {
    let recipe_dir = action.resolved.recipe.path.parent()?;
    let entries = std::fs::read_dir(recipe_dir).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".ebuild") {
            return Some(name);
        }
    }
    None
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
            "status:: current".to_owned(),
            format!("accepted recipe:: {}", stamp.recipe_path),
            format!(
                "digest:: {}",
                &stamp.recipe_hash[..16.min(stamp.recipe_hash.len())]
            ),
            "pager:: skipped because reviewed recipe is unchanged".to_owned(),
        ],
        Some(stamp) => vec![
            "status:: changed since last acceptance".to_owned(),
            format!("previous recipe:: {}", stamp.recipe_path),
            format!(
                "previous digest:: {}",
                &stamp.recipe_hash[..16.min(stamp.recipe_hash.len())]
            ),
            "pager:: opens generated metadata before this prompt".to_owned(),
        ],
        None => vec![
            "status:: new source review".to_owned(),
            "pager:: opens generated metadata before this prompt".to_owned(),
        ],
    }
}

fn interbuild_translation_lines(action: &PlannedInstallAction) -> Vec<String> {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => vec![
            "phase:: parsing flake.nix and optional flake.lock".to_owned(),
            "environment:: buildInputs/nativeBuildInputs mapped into Elda dependency families"
                .to_owned(),
            "boundary:: no Nix daemon or store activation in this source-build slice".to_owned(),
        ],
        "gentoo_overlay" => vec![
            "phase:: locating package ebuild and validating EAPI 8 metadata".to_owned(),
            "environment:: DEPEND/RDEPEND/BDEPEND/IUSE translated into Elda fields".to_owned(),
            "boundary:: accepted eclasses and simple phase commands only".to_owned(),
        ],
        "aur_pkgbuild" => vec![
            "phase:: parsing PKGBUILD metadata and source/checksum arrays".to_owned(),
            "environment:: depends/makedepends/checkdepends/optdepends preserved for review"
                .to_owned(),
            "boundary:: no makepkg or pacman transaction is invoked".to_owned(),
        ],
        "xbps_template" => vec![
            "phase:: parsing srcpkgs template variables and distfile/checksum data".to_owned(),
            "environment:: hostmakedepends/makedepends/depends/checkdepends preserved".to_owned(),
            "boundary:: no xbps-src transaction is invoked".to_owned(),
        ],
        _ => vec!["phase:: interbuild parser boundary".to_owned()],
    }
}

fn interbuild_logic_lines(action: &PlannedInstallAction) -> Vec<String> {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => vec![
            "execution:: extract build/install phases where static analysis can prove them".to_owned(),
            "review:: pinned inputs and unsupported shell constructs surfaced before build"
                .to_owned(),
        ],
        "gentoo_overlay" => vec![
            "execution:: src_prepare/src_configure/src_compile/src_install map to Elda build.lua phases"
                .to_owned(),
            "review:: inherited eclasses and skipped phase functions stay visible".to_owned(),
        ],
        "aur_pkgbuild" => vec![
            "execution:: pkgver/build/package functions accepted only in bounded shell subset"
                .to_owned(),
            "review:: VCS sources, arch sources, provides/conflicts/replaces, optdepends stay visible"
                .to_owned(),
        ],
        "xbps_template" => vec![
            "execution:: do_configure/do_build/do_install and simple v* install macros are bounded"
                .to_owned(),
            "review:: build_style, arch filters, provides/conflicts, skipped functions stay visible"
                .to_owned(),
        ],
        _ => vec!["execution:: bounded parser output".to_owned()],
    }
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
