use crate::app::PlannedInstallAction;

pub(crate) fn interbuild_parse_detail(action: &PlannedInstallAction) -> String {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => "parse flake.nix and optional flake.lock without nix CLI".to_owned(),
        "gentoo_overlay" => "parse ebuild metadata without Portage or emerge".to_owned(),
        "aur_pkgbuild" => "parse PKGBUILD metadata without makepkg or pacman".to_owned(),
        "xbps_template" => "parse XBPS template metadata without xbps-src".to_owned(),
        other => format!("parse {other} metadata"),
    }
}

pub(crate) fn interbuild_translate_detail(action: &PlannedInstallAction) -> String {
    match action.resolved.selected_source_kind.as_str() {
        "nix_flake" => "select default installable from supported static flake outputs".to_owned(),
        "gentoo_overlay" => "validate EAPI, eclasses, metadata, and simple phase shape".to_owned(),
        "aur_pkgbuild" => "validate package arrays and bounded PKGBUILD functions".to_owned(),
        "xbps_template" => "validate template variables and bounded build functions".to_owned(),
        other => format!("translate {other} source into Elda build input"),
    }
}
