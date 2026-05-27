use clap::{CommandFactory, Parser};

use super::Cli;

#[test]
fn no_stream_global_flag_round_trips_into_command_request() {
    let request = Cli::parse_from(["elda", "--no-stream", "--json", "i", "foot"])
        .command_request()
        .expect("request should exist");

    assert!(request.no_stream);
    assert_eq!(request.command_path, vec!["i"]);
}

#[test]
fn doctor_round_trips_into_command_request() {
    let request = Cli::parse_from(["elda", "doctor"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["doctor"]);
    assert!(request.operands.is_empty());
}

#[test]
fn review_commands_round_trip_into_command_request() {
    let list = Cli::parse_from(["elda", "review", "ls"])
        .command_request()
        .expect("request should exist");
    assert_eq!(list.command_path, vec!["review", "ls"]);

    let diff = Cli::parse_from(["elda", "review", "diff", "demo"])
        .command_request()
        .expect("request should exist");
    assert_eq!(diff.command_path, vec!["review", "diff"]);
    assert_eq!(diff.operands[0], "demo");
    assert!(diff.operands.contains(&"--kind".to_owned()));
}

#[test]
fn install_preference_flags_round_trip_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "--offline",
        "-S",
        "i",
        "--use=+wayland,-x11",
        "--prefer-source",
        "ripgrep",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["i"]);
    assert_eq!(
        request.operands,
        vec!["ripgrep", "--use=+wayland,-x11", "--prefer-source",]
    );
    assert!(request.system_mode);
    assert!(request.offline);
}

#[test]
fn install_exclude_tail_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "i",
        "metapkg",
        "--prefer-source",
        "--exclude",
        "firefox",
        "vlc",
        "foot",
    ]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["i"]);
    assert_eq!(
        request.operands,
        vec![
            "metapkg",
            "--prefer-source",
            "--exclude",
            "firefox",
            "vlc",
            "foot",
        ]
    );
}

#[test]
fn rotated_key_acceptance_flag_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "--accept-rotated-key",
        "main",
        "--accept-rotated-key",
        "staging",
        "sync",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["sync"]);
    assert_eq!(
        request.accept_rotated_keys,
        vec!["main".to_owned(), "staging".to_owned()]
    );
}

#[test]
fn explicit_lane_commands_are_exposed() {
    let ig = Cli::parse_from(["elda", "ig", "ripgrep"]);
    let ib = Cli::parse_from(["elda", "ib", "ripgrep"]);

    assert_eq!(
        ig.command_request()
            .expect("request should exist")
            .command_path,
        vec!["ig"]
    );
    assert_eq!(
        ib.command_request()
            .expect("request should exist")
            .command_path,
        vec!["ib"]
    );
}

#[test]
fn profile_apply_flags_round_trip_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "pf",
        "apply",
        "yoka-core",
        "--init",
        "dinit",
        "--native-arch",
        "amd64",
        "--foreign-arch",
        "i386",
        "--foreign-arch",
        "arm64",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["pf", "apply"]);
    assert_eq!(
        request.operands,
        vec![
            "yoka-core",
            "--init",
            "dinit",
            "--native-arch",
            "amd64",
            "--foreign-arch",
            "i386",
            "--foreign-arch",
            "arm64",
        ]
    );
}

#[test]
fn profile_edit_commands_round_trip_into_command_request() {
    let add = Cli::parse_from(["elda", "pf", "add", "yoka-desktop"]);
    let rm = Cli::parse_from(["elda", "pf", "rm", "yoka-core"]);
    let set_arch = Cli::parse_from(["elda", "pf", "set-arch", "arm64"]);
    let add_foreign = Cli::parse_from(["elda", "pf", "add-foreign-arch", "i386", "armhf"]);
    let clear_init = Cli::parse_from(["elda", "pf", "clear-init"]);

    assert_eq!(
        add.command_request()
            .expect("request should exist")
            .command_path,
        vec!["pf", "add"]
    );
    assert_eq!(
        rm.command_request()
            .expect("request should exist")
            .command_path,
        vec!["pf", "rm"]
    );
    assert_eq!(
        set_arch
            .command_request()
            .expect("request should exist")
            .operands,
        vec!["arm64"]
    );
    assert_eq!(
        add_foreign
            .command_request()
            .expect("request should exist")
            .operands,
        vec!["i386", "armhf"]
    );
    assert_eq!(
        clear_init
            .command_request()
            .expect("request should exist")
            .command_path,
        vec!["pf", "clear-init"]
    );
}

#[test]
fn recipe_add_kind_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "rc",
        "add",
        "yoka-core",
        "--kind",
        "profile",
        "--replace",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["rc", "add"]);
    assert_eq!(
        request.operands,
        vec!["yoka-core", "--kind", "profile", "--replace"]
    );
}

#[test]
fn vendor_add_args_round_trip_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "vendor",
        "add",
        "demo-bin",
        "https://example.invalid/demo.tar.gz",
        "--binary",
        "demo",
        "--asset",
        "demo.tar.gz",
        "--replace",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["vendor", "add"]);
    assert_eq!(
        request.operands,
        vec![
            "demo-bin",
            "https://example.invalid/demo.tar.gz",
            "--binary",
            "demo",
            "--asset",
            "demo.tar.gz",
            "--replace",
        ]
    );
}

#[test]
fn vendor_import_replace_round_trips_into_command_request() {
    let request = Cli::parse_from(["elda", "vendor", "import", "vendor.lock", "--replace"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["vendor", "import"]);
    assert_eq!(request.operands, vec!["vendor.lock", "--replace"]);
}

#[test]
fn ls_filter_flags_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "ls", "--explicit", "--source-kind", "git"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["ls"]);
    assert_eq!(request.operands, vec!["--explicit", "--source-kind", "git"]);
}

#[test]
fn files_search_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "files", "search", "bin/demo"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["files", "search"]);
    assert_eq!(request.operands, vec!["bin/demo"]);
}

#[test]
fn trigger_commands_round_trip_into_command_request() {
    let list = Cli::parse_from(["elda", "trigger", "ls"])
        .command_request()
        .expect("request should exist");
    let info = Cli::parse_from(["elda", "trigger", "info", "ldconfig"])
        .command_request()
        .expect("request should exist");

    assert_eq!(list.command_path, vec!["trigger", "ls"]);
    assert!(list.operands.is_empty());
    assert_eq!(info.command_path, vec!["trigger", "info"]);
    assert_eq!(info.operands, vec!["ldconfig"]);
}

#[test]
fn config_pending_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "config", "pending"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["config", "pending"]);
    assert!(request.operands.is_empty());
}

#[test]
fn rc_show_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "rc", "show", "demo"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["rc", "show"]);
    assert_eq!(request.operands, vec!["demo"]);
}

#[test]
fn rc_diff_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "rc", "diff", "demo"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["rc", "diff"]);
    assert_eq!(request.operands, vec!["demo"]);
}

#[test]
fn rc_publish_ready_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "rc", "publish-ready", "demo"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["rc", "publish-ready"]);
    assert_eq!(request.operands, vec!["demo"]);
}

#[test]
fn sync_targets_round_trip_into_command_request() {
    let request = Cli::parse_from(["elda", "sync", "main", "testing"])
        .command_request()
        .expect("request should exist");

    assert_eq!(request.command_path, vec!["sync"]);
    assert_eq!(request.operands, vec!["main", "testing"]);
}

#[test]
fn remote_add_trust_args_round_trip_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "rmt",
        "add",
        "main=https://example.invalid/index.toml",
        "--priority",
        "20",
        "--trust",
        "pinned",
        "--trusted-key",
        "sha256:abc123",
        "--signature-url",
        "https://example.invalid/index.toml.sig",
        "--metadata-url",
        "https://example.invalid/remote-metadata-v1.toml",
        "--packages-url",
        "https://example.invalid/packages.git",
        "--channel",
        "stable-7d",
        "--allow-stale",
        "--replace",
        "--exclude",
        "firefox",
        "vlc",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["rmt", "add"]);
    assert_eq!(
        request.operands,
        vec![
            "main=https://example.invalid/index.toml",
            "--priority",
            "20",
            "--trust",
            "pinned",
            "--trusted-key",
            "sha256:abc123",
            "--signature-url",
            "https://example.invalid/index.toml.sig",
            "--metadata-url",
            "https://example.invalid/remote-metadata-v1.toml",
            "--packages-url",
            "https://example.invalid/packages.git",
            "--channel",
            "stable-7d",
            "--allow-stale",
            "--replace",
            "--exclude",
            "firefox",
            "vlc",
        ]
    );
}

#[test]
fn remote_info_and_preview_round_trip_into_command_request() {
    let info = Cli::parse_from(["elda", "rmt", "info", "heather"])
        .command_request()
        .expect("request should exist");
    assert_eq!(info.command_path, vec!["rmt", "info"]);
    assert_eq!(info.operands, vec!["heather"]);

    let preview = Cli::parse_from(["elda", "rmt", "preview", "heather"])
        .command_request()
        .expect("request should exist");
    assert_eq!(preview.command_path, vec!["rmt", "preview"]);
    assert_eq!(preview.operands, vec!["heather"]);

    let trust = Cli::parse_from(["elda", "rmt", "trust", "heather"])
        .command_request()
        .expect("request should exist");
    assert_eq!(trust.command_path, vec!["rmt", "trust"]);
    assert_eq!(trust.operands, vec!["heather"]);
}

#[test]
fn remote_management_commands_round_trip_into_command_request() {
    let list = Cli::parse_from(["elda", "rmt", "list"])
        .command_request()
        .expect("request should exist");
    assert_eq!(list.command_path, vec!["rmt", "ls"]);
    assert!(list.operands.is_empty());

    let enable = Cli::parse_from(["elda", "rmt", "enable", "main"])
        .command_request()
        .expect("request should exist");
    assert_eq!(enable.command_path, vec!["rmt", "enable"]);
    assert_eq!(enable.operands, vec!["main"]);

    let disable = Cli::parse_from(["elda", "rmt", "disable", "main"])
        .command_request()
        .expect("request should exist");
    assert_eq!(disable.command_path, vec!["rmt", "disable"]);
    assert_eq!(disable.operands, vec!["main"]);

    let priority = Cli::parse_from(["elda", "rmt", "set-priority", "main", "10"])
        .command_request()
        .expect("request should exist");
    assert_eq!(priority.command_path, vec!["rmt", "set-priority"]);
    assert_eq!(priority.operands, vec!["main", "10"]);
}

#[test]
fn config_merge_commands_round_trip_into_command_request() {
    let diff = Cli::parse_from(["elda", "config", "diff", "/etc/example.conf"])
        .command_request()
        .expect("request should exist");
    assert_eq!(diff.command_path, vec!["config", "diff"]);
    assert_eq!(diff.operands, vec!["/etc/example.conf"]);

    let apply = Cli::parse_from(["elda", "config", "apply", "example"])
        .command_request()
        .expect("request should exist");
    assert_eq!(apply.command_path, vec!["config", "apply"]);
    assert_eq!(apply.operands, vec!["example"]);

    let keep = Cli::parse_from(["elda", "config", "keep", "example"])
        .command_request()
        .expect("request should exist");
    assert_eq!(keep.command_path, vec!["config", "keep"]);
    assert_eq!(keep.operands, vec!["example"]);
}

#[test]
fn cache_add_priority_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "cache",
        "add",
        "lan=file:///var/cache/elda-mirror",
        "--priority",
        "20",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["cache", "add"]);
    assert_eq!(
        request.operands,
        vec!["lan=file:///var/cache/elda-mirror", "--priority", "20",]
    );
}

#[test]
fn search_interactive_flag_round_trips_into_command_request() {
    let cli = Cli::parse_from(["elda", "search", "fsel", "--interactive"]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["search"]);
    assert_eq!(request.operands, vec!["fsel", "--interactive"]);
}

#[test]
fn appimage_inspect_round_trips_into_command_request() {
    let cli = Cli::parse_from(["elda", "appimage", "inspect", "/tmp/demo.AppImage"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["appimage", "inspect"]);
    assert_eq!(request.operands, vec!["/tmp/demo.AppImage".to_owned()]);
}

#[test]
fn publish_scope_flags_round_trip_as_assignments() {
    let cli = Cli::parse_from([
        "elda",
        "publish",
        "plan",
        "demo",
        "--tree",
        "/tmp/pkgs",
        "--channel",
        "testing",
        "--profile",
        "forge",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["publish", "plan"]);
    assert_eq!(
        request.operands,
        vec![
            "demo",
            "--tree=/tmp/pkgs",
            "--channel=testing",
            "--profile=forge",
        ]
    );
}

#[test]
fn publish_diff_channel_does_not_look_like_previous_index() {
    let cli = Cli::parse_from(["elda", "publish", "diff", "--channel", "testing"]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["publish", "diff"]);
    assert_eq!(request.operands, vec!["--channel=testing"]);
}

#[test]
fn root_help_contains_canonical_namespaces() {
    let command = Cli::command();
    let names = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name())
        .collect::<Vec<_>>();

    for expected in [
        "i",
        "ig",
        "ib",
        "rm",
        "u",
        "sync",
        "ls",
        "list",
        "search",
        "info",
        "files",
        "verify",
        "reverify",
        "why",
        "rdeps",
        "pin",
        "unpin",
        "hold",
        "unhold",
        "adopt",
        "downgrade",
        "diff",
        "check",
        "doctor",
        "recover",
        "rollback",
        "fix-triggers",
        "autoremove",
        "rmt",
        "rc",
        "ci",
        "vendor",
        "forge",
        "git",
        "appimage",
        "pf",
        "fl",
        "mg",
        "state",
        "cache",
        "daemon",
        "ext",
        "qa",
    ] {
        assert!(names.contains(&expected));
    }
}

#[test]
fn list_routes_to_list_command_path() {
    let cli = Cli::parse_from(["elda", "list"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["list"]);
    assert!(request.operands.is_empty());
}

#[test]
fn list_with_package_names_round_trips_operands() {
    let cli = Cli::parse_from(["elda", "list", "fsel", "foot"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["list"]);
    assert_eq!(request.operands, vec!["fsel", "foot"]);
}

#[test]
fn rc_list_alias_routes_to_rc_ls_command_path() {
    let cli = Cli::parse_from(["elda", "rc", "list"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["rc", "ls"]);
}

#[test]
fn cache_list_alias_routes_to_cache_ls_command_path() {
    let cli = Cli::parse_from(["elda", "cache", "list"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["cache", "ls"]);
}

#[test]
fn ext_list_alias_routes_to_ext_ls_command_path() {
    let cli = Cli::parse_from(["elda", "ext", "list"]);
    let request = cli.command_request().expect("request should exist");
    assert_eq!(request.command_path, vec!["ext", "ls"]);
}

#[test]
fn ci_batch_namespace_contains_nested_commands() {
    let command = Cli::command();
    let ci = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "ci")
        .expect("missing ci namespace");
    let batch = ci
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "batch")
        .expect("missing ci batch namespace");
    let names = batch
        .get_subcommands()
        .map(|subcommand| subcommand.get_name())
        .collect::<Vec<_>>();

    assert!(names.contains(&"new"));
    assert!(names.contains(&"add"));
    assert!(names.contains(&"push"));
}
