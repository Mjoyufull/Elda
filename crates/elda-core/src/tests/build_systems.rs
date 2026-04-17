use super::support::*;
use super::*;

#[test]
fn direct_git_install_supports_regular_build_systems() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");

    let cases = vec![
        (
            "make-tool",
            create_git_make_repo(tempdir.path(), "make-tool"),
            vec!["git", "make"],
            "make tool",
        ),
        (
            "cmake-tool",
            create_git_cmake_repo(tempdir.path(), "cmake-tool"),
            vec!["git", "cmake", "cc"],
            "cmake tool",
        ),
        (
            "mesonpkg-tool",
            create_git_meson_repo(tempdir.path(), "mesonpkg-tool"),
            vec!["git", "meson", "ninja", "cc"],
            "meson tool",
        ),
        (
            "go-tool",
            create_git_go_repo(tempdir.path(), "go-tool"),
            vec!["git", "go"],
            "go tool",
        ),
        (
            "zig-tool",
            create_git_zig_repo(tempdir.path(), "zig-tool"),
            vec!["git", "zig"],
            "zig tool",
        ),
        (
            "py-tool",
            create_git_python_repo(tempdir.path(), "py-tool"),
            vec!["git", "python3", "pip3"],
            "python tool",
        ),
        (
            "nimtool",
            create_git_nimble_repo(tempdir.path(), "nimtool"),
            vec!["git", "nimble", "nim"],
            "nim tool",
        ),
    ];

    for (name, repo_dir, tools, expected_output) in cases {
        if !all_tools_available(&tools) {
            continue;
        }

        let repo_url = format!("file://{}", repo_dir.display());
        let report = run_from_root(
            tempdir.path(),
            CommandRequest::new(
                vec!["i".to_owned()],
                vec![repo_url],
                OutputMode::Json,
                false,
            ),
        )
        .expect("install should succeed");

        assert_eq!(report.area, "install");
        assert_eq!(
            run_installed_binary(tempdir.path(), &format!("/opt/elda/bin/{name}")),
            expected_output
        );

        run_from_root(
            tempdir.path(),
            CommandRequest::new(
                vec!["rm".to_owned()],
                vec![name.to_owned()],
                OutputMode::Json,
                false,
            ),
        )
        .expect("remove should succeed");
    }
}
