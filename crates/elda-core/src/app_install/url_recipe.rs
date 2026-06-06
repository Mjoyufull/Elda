use crate::error::CoreError;

pub(super) fn reject_stale_url_recipe_reuse(
    target: &str,
    report: &elda_recipe::ImportReport,
) -> Result<(), CoreError> {
    if !report.reused_existing_pkg_lua {
        return Ok(());
    }

    Err(CoreError::Operator(format!(
        "remote metadata for `{target}` was not imported because local recipe `{}` already exists at {}; install `{}` explicitly to use the local recipe, or pass `--replace` to regenerate it from the remote source",
        report.recipe_name,
        report.recipe_dir.display(),
        report.recipe_name,
    )))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::reject_stale_url_recipe_reuse;

    #[test]
    fn stale_url_recipe_reuse_requires_an_explicit_operator_choice() {
        let report = elda_recipe::ImportReport {
            recipe_name: "hyprlock".to_owned(),
            recipe_dir: PathBuf::from("/etc/elda/recipes/hyprlock"),
            source_options: Vec::new(),
            selected_source_option: None,
            imported_pkg_lua: false,
            imported_build_lua: false,
            imported_patches: false,
            generated_pkg_lua: false,
            reused_existing_pkg_lua: true,
            generated_build_lua: false,
            imported_legacy_pkgdeps: false,
            imported_legacy_bldit: false,
            wrote_legacy_summary: false,
        };

        let error =
            reject_stale_url_recipe_reuse("https://aur.archlinux.org/hyprlock.git", &report)
                .expect_err("stale local recipe reuse must stop");

        assert!(error.to_string().contains("install `hyprlock` explicitly"));
        assert!(error.to_string().contains("pass `--replace`"));
    }
}
