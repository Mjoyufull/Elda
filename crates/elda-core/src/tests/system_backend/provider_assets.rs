use std::fs;

use super::super::support::*;
use super::super::*;
use super::fixtures::{create_system_make_repo, write_system_recipe_with_provider_assets};

#[test]
fn system_mode_install_materializes_active_provider_assets() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "system-service", "system backend v1");

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

    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "system-service",
        &repo_dir,
        "0.1.0",
        "u service - ServiceUser /usr/bin/false",
        "d /run/system-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/system-service",
          file = "providers/init/dinit/system-service",
          mode = "0755",
        },
      },
      openrc = {
        {
          kind = "file",
          target = "/etc/init.d/system-service",
          text = "#!/bin/sh\necho openrc\n",
          mode = "0755",
        },
      },
    },
  }"##,
    );
    let provider_dir = tempdir
        .path()
        .join("etc/elda/recipes/system-service/providers/init/dinit");
    fs::create_dir_all(&provider_dir).expect("provider asset dir should exist");
    fs::write(
        provider_dir.join("system-service"),
        "#!/bin/sh\necho dinit\n",
    )
    .expect("provider asset file should be written");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["system-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/provider-assets/init/dinit/system-service/0.asset")
        )
        .expect("stored provider asset should exist"),
        "#!/bin/sh\necho dinit\n"
    );
    assert_eq!(
        fs::read_to_string(tempdir.path().join("etc/dinit.d/system-service"))
            .expect("active provider asset should exist"),
        "#!/bin/sh\necho dinit\n"
    );
    assert!(!tempdir.path().join("etc/init.d/system-service").exists());

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["info".to_owned()],
            vec!["system-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed");

    assert!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("provider_asset_visibility"))
            .and_then(|visibility| visibility.get("installed_system_assets"))
            .and_then(|assets| assets.get("provider_assets"))
            .and_then(|assets| assets.as_array())
            .is_some_and(|assets| assets.iter().any(|asset| {
                asset.get("provider").and_then(|value| value.as_str()) == Some("dinit")
                    && asset.get("active").and_then(|value| value.as_bool()) == Some(true)
            }))
    );
}

#[test]
fn system_mode_install_materializes_tree_provider_assets() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");
    let repo_dir = create_system_make_repo(tempdir.path(), "tree-service", "system backend v1");

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

    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "tree-service",
        &repo_dir,
        "0.1.0",
        "u service - ServiceUser /usr/bin/false",
        "d /run/tree-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "tree",
          target = "/etc/dinit.d/tree-service",
          dir = "providers/init/dinit/tree-service",
        },
      },
    },
  }"##,
    );
    let provider_root = tempdir
        .path()
        .join("etc/elda/recipes/tree-service/providers/init/dinit/tree-service");
    fs::create_dir_all(provider_root.join("log")).expect("provider tree should exist");
    fs::write(provider_root.join("run"), "#!/bin/sh\necho run\n")
        .expect("provider tree file should be written");
    fs::write(provider_root.join("log/run"), "#!/bin/sh\necho log\n")
        .expect("provider tree file should be written");

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["tree-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("system-mode install should succeed");

    assert_eq!(
        fs::read_to_string(
            tempdir
                .path()
                .join("usr/lib/elda/provider-assets/init/dinit/tree-service/0/run")
        )
        .expect("stored provider tree should exist"),
        "#!/bin/sh\necho run\n"
    );
    assert_eq!(
        fs::read_to_string(tempdir.path().join("etc/dinit.d/tree-service/log/run"))
            .expect("active provider tree should exist"),
        "#!/bin/sh\necho log\n"
    );
}

#[test]
fn system_mode_install_fails_closed_on_provider_asset_target_conflict() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/usr");

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

    let alpha_repo = create_system_make_repo(tempdir.path(), "alpha-service", "alpha v1");
    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "alpha-service",
        &alpha_repo,
        "0.1.0",
        "u alpha - AlphaUser /usr/bin/false",
        "d /run/alpha-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/shared-service",
          text = "#!/bin/sh\necho alpha\n",
          mode = "0755",
        },
      },
    },
  }"##,
    );
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["alpha-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("first install should succeed");

    let beta_repo = create_system_make_repo(tempdir.path(), "beta-service", "beta v1");
    write_system_recipe_with_provider_assets(
        tempdir.path(),
        "beta-service",
        &beta_repo,
        "0.1.0",
        "u beta - BetaUser /usr/bin/false",
        "d /run/beta-service 0755 root root -",
        r##"{
    init = {
      dinit = {
        {
          kind = "file",
          target = "/etc/dinit.d/shared-service",
          text = "#!/bin/sh\necho beta\n",
          mode = "0755",
        },
      },
    },
  }"##,
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["beta-service".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("conflicting provider asset target should fail");

    assert!(error.to_string().contains(
        "path conflict on `/etc/dinit.d/shared-service` with installed package `alpha-service`"
    ));
}
