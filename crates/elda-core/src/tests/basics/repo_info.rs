use std::fs;

use super::*;

#[test]
fn info_reports_local_recipe_assets_and_pending_provider_handlers() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let repo_dir = create_git_make_repo(tempdir.path(), "service-tool");
    let recipe_dir = tempdir.path().join("etc/elda/recipes/service-tool");
    fs::create_dir_all(recipe_dir.join("hooks")).expect("recipe dir should exist");
    fs::write(
        recipe_dir.join("pkg.lua"),
        format!(
            "pkg = {{\n  name = \"service-tool\",\n  epoch = 0,\n  version = \"0.1.0\",\n  rel = 1,\n  arch = {{ \"amd64\" }},\n  kind = \"normal\",\n  source = {{\n    kind = \"git\",\n    url = \"file://{repo}\",\n    branch = \"main\",\n  }},\n  depends = {{}},\n  makedepends = {{}},\n  checkdepends = {{}},\n  recommends = {{}},\n  suggests = {{}},\n  supplements = {{}},\n  enhances = {{}},\n  provides = {{}},\n  conflicts = {{}},\n  replaces = {{}},\n  conffiles = {{}},\n  sysusers = {{ \"u service - ServiceUser /usr/bin/false\" }},\n  tmpfiles = {{ file = \"metadata/tmpfiles.conf\" }},\n  alternatives = {{\n    {{ name = \"service-tool\", link = \"/usr/bin/service-toolctl\", path = \"/usr/bin/service-tool\", priority = 25 }},\n  }},\n  hooks = {{\n    post_install = {{ file = \"hooks/post_install.lua\" }},\n  }},\n}}\n",
            repo = repo_dir.display(),
        ),
    )
    .expect("pkg.lua should be written");
    fs::create_dir_all(recipe_dir.join("metadata")).expect("metadata dir should exist");
    fs::write(
        recipe_dir.join("metadata/tmpfiles.conf"),
        "d /run/service-tool 0755 root root -\n",
    )
    .expect("tmpfiles metadata should be written");
    fs::write(
        recipe_dir.join("hooks/post_install.lua"),
        "print('post install')\n",
    )
    .expect("hook should be written");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["pf".to_owned(), "set-init".to_owned()],
            vec!["dinit".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("pf set-init should succeed");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["info".to_owned()],
            vec!["service-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed for local recipe");

    assert_eq!(report.area, "info");
    assert_eq!(report.status, "ok");
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("recipe"))
            .and_then(|recipe| recipe.get("source"))
            .and_then(|source| source.as_str()),
        Some("local")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("declarative_assets"))
            .and_then(|assets| assets.get("sysusers"))
            .and_then(|sysusers| sysusers.get("kind"))
            .and_then(|kind| kind.as_str()),
        Some("inline")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("declarative_assets"))
            .and_then(|assets| assets.get("tmpfiles"))
            .and_then(|tmpfiles| tmpfiles.get("file"))
            .and_then(|file| file.as_str()),
        Some("metadata/tmpfiles.conf")
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("declarative_assets"))
            .and_then(|assets| assets.get("hooks"))
            .and_then(|hooks| hooks.as_array())
            .is_some_and(|hooks| hooks.iter().any(|hook| {
                hook.get("phase").and_then(|phase| phase.as_str()) == Some("post_install")
                    && hook.get("source_kind").and_then(|kind| kind.as_str()) == Some("file")
            }))
    );
    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("pending_provider_handlers"))
            .and_then(|handlers| handlers.as_array())
            .is_some_and(|handlers| handlers.iter().any(|handler| {
                handler.get("kind").and_then(|kind| kind.as_str())
                    == Some("init-provider-transition")
            }))
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("provider_families"))
            .and_then(|families| families.get("init"))
            .and_then(|init| init.as_str()),
        Some("dinit")
    );
}
