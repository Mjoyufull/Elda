use elda_repo::{RemoteDocument, TrustMode, save_remote};

use super::super::support::*;
use super::super::*;

#[test]
fn install_selects_synced_provider_from_best_remote_priority() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    let mesa_binary = create_vendor_binary(tempdir.path(), "mesa-provider");
    let zink_binary = create_vendor_binary(tempdir.path(), "zink-provider");
    let mesa_index = tempdir.path().join("mesa-index.toml");
    let zink_index = tempdir.path().join("zink-index.toml");
    write_remote_index_with_lua_fields_and_provides(
        &mesa_index,
        "mesa-provider",
        &mesa_binary,
        "1.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    write_remote_index_with_lua_fields_and_provides(
        &zink_index,
        "zink-provider",
        &zink_binary,
        "9.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    save_remote(
        &tempdir.path().join("etc/elda/remotes.d"),
        RemoteDocument {
            name: "mesa".to_owned(),
            index_url: format!("file://{}", mesa_index.display()),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_remote_key_fingerprint()],
            allow_stale: false,
            priority: 10,
        },
    )
    .expect("mesa remote should be saved");
    save_remote(
        &tempdir.path().join("etc/elda/remotes.d"),
        RemoteDocument {
            name: "zink".to_owned(),
            index_url: format!("file://{}", zink_index.display()),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_remote_key_fingerprint()],
            allow_stale: false,
            priority: 20,
        },
    )
    .expect("zink remote should be saved");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider\" }",
        "{}",
        "{}",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert!(tempdir.path().join("opt/elda/bin/mesa-provider").exists());
    assert!(!tempdir.path().join("opt/elda/bin/zink-provider").exists());
}

#[test]
fn install_selects_highest_version_provider_within_same_remote_priority() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    let older_binary = create_vendor_binary(tempdir.path(), "older-provider");
    let newer_binary = create_vendor_binary(tempdir.path(), "newer-provider");
    let older_index = tempdir.path().join("older-index.toml");
    let newer_index = tempdir.path().join("newer-index.toml");
    write_remote_index_with_lua_fields_and_provides(
        &older_index,
        "older-provider",
        &older_binary,
        "1.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    write_remote_index_with_lua_fields_and_provides(
        &newer_index,
        "newer-provider",
        &newer_binary,
        "2.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    for (name, index_path) in [("older", &older_index), ("newer", &newer_index)] {
        save_remote(
            &tempdir.path().join("etc/elda/remotes.d"),
            RemoteDocument {
                name: name.to_owned(),
                index_url: format!("file://{}", index_path.display()),
                packages_url: None,
                metadata_url: None,
                signature_url: None,
                enabled: true,
                trust: TrustMode::Pinned,
                trusted_keys: vec![fixture_remote_key_fingerprint()],
                allow_stale: false,
                priority: 10,
            },
        )
        .expect("remote should be saved");
    }
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider\" }",
        "{}",
        "{}",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert!(!tempdir.path().join("opt/elda/bin/older-provider").exists());
    assert!(tempdir.path().join("opt/elda/bin/newer-provider").exists());
}

#[test]
fn install_allows_configured_provider_preference_to_override_remote_priority() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        "\n[resolver.provider_preferences]\ngl-provider = [\"zink-provider\", \"mesa-provider\"]\n",
    );
    let app_binary = create_script_binary(tempdir.path(), "app-tool", "app tool");
    let mesa_binary = create_vendor_binary(tempdir.path(), "mesa-provider");
    let zink_binary = create_vendor_binary(tempdir.path(), "zink-provider");
    let mesa_index = tempdir.path().join("mesa-index.toml");
    let zink_index = tempdir.path().join("zink-index.toml");
    write_remote_index_with_lua_fields_and_provides(
        &mesa_index,
        "mesa-provider",
        &mesa_binary,
        "1.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    write_remote_index_with_lua_fields_and_provides(
        &zink_index,
        "zink-provider",
        &zink_binary,
        "1.0.0",
        "{}",
        "{}",
        "{ \"gl-provider\" }",
    );
    save_remote(
        &tempdir.path().join("etc/elda/remotes.d"),
        RemoteDocument {
            name: "mesa".to_owned(),
            index_url: format!("file://{}", mesa_index.display()),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_remote_key_fingerprint()],
            allow_stale: false,
            priority: 10,
        },
    )
    .expect("mesa remote should be saved");
    save_remote(
        &tempdir.path().join("etc/elda/remotes.d"),
        RemoteDocument {
            name: "zink".to_owned(),
            index_url: format!("file://{}", zink_index.display()),
            packages_url: None,
            metadata_url: None,
            signature_url: None,
            enabled: true,
            trust: TrustMode::Pinned,
            trusted_keys: vec![fixture_remote_key_fingerprint()],
            allow_stale: false,
            priority: 20,
        },
    )
    .expect("zink remote should be saved");
    write_local_binary_recipe_with_lua_fields(
        tempdir.path(),
        "app-tool",
        &app_binary,
        "0.1.0",
        "{ \"gl-provider\" }",
        "{}",
        "{}",
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(vec!["sync".to_owned()], Vec::new(), OutputMode::Json, false),
    )
    .expect("sync should succeed");
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["app-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");

    assert!(!tempdir.path().join("opt/elda/bin/mesa-provider").exists());
    assert!(tempdir.path().join("opt/elda/bin/zink-provider").exists());
}
