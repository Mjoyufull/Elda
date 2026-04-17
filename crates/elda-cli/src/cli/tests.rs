use clap::{CommandFactory, Parser};

use super::Cli;

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
fn upgrade_refresh_weak_deps_flag_round_trips_into_command_request() {
    let cli = Cli::parse_from(["elda", "u", "mesa", "--refresh-weak-deps"]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["u"]);
    assert_eq!(
        request.operands,
        vec!["mesa".to_owned(), "--refresh-weak-deps".to_owned()]
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
            "--foreign-arch",
            "i386",
            "--foreign-arch",
            "arm64",
        ]
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
        ]
    );
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
        "--allow-stale",
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
            "--allow-stale",
        ]
    );
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
        "recover",
        "rollback",
        "fix-triggers",
        "autoremove",
        "rmt",
        "rc",
        "ci",
        "vendor",
        "forge",
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
