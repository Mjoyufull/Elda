use clap::Parser;

use super::Cli;

#[test]
fn metadata_add_commands_round_trip_into_command_request() {
    let short = Cli::parse_from([
        "elda",
        "a",
        "https://example.invalid/tool.git",
        "--source-option",
        "2",
        "--strategy",
        "aur_pkgbuild",
        "--to-tag",
        "v1.2.3",
        "--prefer-source",
    ]);
    let long = Cli::parse_from(["elda", "add", "./tool"]);

    assert_eq!(
        short
            .command_request()
            .expect("request should exist")
            .command_path,
        vec!["a"]
    );
    assert_eq!(
        short
            .command_request()
            .expect("request should exist")
            .operands,
        vec![
            "https://example.invalid/tool.git".to_owned(),
            "--source-option".to_owned(),
            "2".to_owned(),
            "--strategy".to_owned(),
            "aur_pkgbuild".to_owned(),
            "--to-tag".to_owned(),
            "v1.2.3".to_owned(),
            "--prefer-source".to_owned(),
        ]
    );
    assert_eq!(
        long.command_request()
            .expect("request should exist")
            .command_path,
        vec!["add"]
    );
}

#[test]
fn upgrade_git_ref_flags_round_trip_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "u",
        "mesa",
        "--refresh-weak-deps",
        "--to-tag",
        "v1.2.3",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["u"]);
    assert_eq!(
        request.operands,
        vec![
            "mesa".to_owned(),
            "--refresh-weak-deps".to_owned(),
            "--to-tag".to_owned(),
            "v1.2.3".to_owned(),
        ]
    );
}

#[test]
fn downgrade_git_ref_flags_round_trip_into_command_request() {
    let cli = Cli::parse_from(["elda", "downgrade", "fsel", "--to-tag", "v3.3.1"]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["downgrade"]);
    assert_eq!(
        request.operands,
        vec![
            "fsel".to_owned(),
            "--to-tag".to_owned(),
            "v3.3.1".to_owned(),
        ]
    );
}

#[test]
fn git_tags_command_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "git",
        "tags",
        "https://example.invalid/tool.git",
        "--max-tags",
        "20",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["git", "tags"]);
    assert_eq!(
        request.operands,
        vec!["https://example.invalid/tool.git", "--max-tags", "20"]
    );
}

#[test]
fn git_tags_with_releases_flag_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "git",
        "tags",
        "https://github.com/Mjoyufull/fsel",
        "--max-tags",
        "5",
        "--with-releases",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["git", "tags"]);
    assert_eq!(
        request.operands,
        vec![
            "https://github.com/Mjoyufull/fsel",
            "--max-tags",
            "5",
            "--with-releases"
        ]
    );
}

#[test]
fn versions_command_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "versions",
        "https://example.invalid/tool.git",
        "--max-tags",
        "10",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["versions"]);
    assert_eq!(
        request.operands,
        vec!["https://example.invalid/tool.git", "--max-tags", "10"]
    );
}

#[test]
fn git_releases_command_round_trips_into_command_request() {
    let cli = Cli::parse_from([
        "elda",
        "git",
        "releases",
        "Mjoyufull/fsel",
        "--max-releases",
        "3",
        "--tag",
        "v3.5.0",
    ]);
    let request = cli.command_request().expect("request should exist");

    assert_eq!(request.command_path, vec!["git", "releases"]);
    assert_eq!(
        request.operands,
        vec!["Mjoyufull/fsel", "--max-releases", "3", "--tag", "v3.5.0"]
    );
}
