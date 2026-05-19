use super::support::*;
use super::*;

#[test]
fn adopt_pacman_package_records_adopted_state() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_pacman_package(
        tempdir.path(),
        "foreign-tool",
        "1:2.3.4-5",
        &["usr/bin/foreign-tool", "etc/foreign-tool.conf"],
        &["glibc>=2.39"],
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["adopt".to_owned()],
            vec![
                "--from".to_owned(),
                "pacman".to_owned(),
                "foreign-tool".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("adopt should succeed");

    assert_eq!(report.area, "migration");
    assert_eq!(report.status, "ok");
    assert_eq!(
        installed_source_kind(tempdir.path(), "foreign-tool"),
        "adopted"
    );

    let files = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["files".to_owned()],
            vec!["foreign-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("files should succeed");
    assert!(
        files
            .details
            .as_ref()
            .and_then(|details| details.get("files"))
            .and_then(|value| value.as_array())
            .is_some_and(|files| files
                .iter()
                .any(|file| file.get("path").and_then(|path| path.as_str())
                    == Some("/usr/bin/foreign-tool")))
    );
}

#[test]
fn migration_from_apt_imports_installed_packages() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_dpkg_package(
        tempdir.path(),
        "coreutils",
        "9.5-1",
        &["/usr/bin/ls", "/usr/bin/cp"],
    );
    write_dpkg_package(tempdir.path(), "bash", "5.2.21-2", &["/usr/bin/bash"]);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["mg".to_owned(), "from".to_owned()],
            vec!["apt".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("migration should succeed");

    assert_eq!(report.area, "migration");
    assert_eq!(report.status, "ok");
    assert_eq!(installed_source_kind(tempdir.path(), "bash"), "adopted");
    assert_eq!(
        installed_source_kind(tempdir.path(), "coreutils"),
        "adopted"
    );
}

#[test]
fn adopt_rejects_existing_elda_owner() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let binary = create_vendor_binary(tempdir.path(), "owned-tool");
    write_local_binary_recipe(tempdir.path(), "owned-tool", &binary, &[]);
    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["owned-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("install should succeed");
    write_pacman_package(
        tempdir.path(),
        "foreign-owned",
        "1.0.0-1",
        &["usr/bin/owned-tool"],
        &[],
    );

    let error = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["adopt".to_owned()],
            vec![
                "--from".to_owned(),
                "pacman".to_owned(),
                "foreign-owned".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect_err("adopt should fail on path conflict");
    assert!(error.to_string().contains("managed path conflicts"));
}

fn write_pacman_package(
    root: &std::path::Path,
    name: &str,
    version: &str,
    files: &[&str],
    dependencies: &[&str],
) {
    let package_dir = root
        .join("var/lib/pacman/local")
        .join(format!("{name}-{version}"));
    fs::create_dir_all(&package_dir).expect("pacman package dir should be created");
    let mut desc = format!("%NAME%\n{name}\n\n%VERSION%\n{version}\n\n%ARCH%\nx86_64\n\n%FILES%\n");
    for path in files {
        desc.push_str(path);
        desc.push('\n');
    }
    if !dependencies.is_empty() {
        desc.push_str("\n%DEPENDS%\n");
        for dependency in dependencies {
            desc.push_str(dependency);
            desc.push('\n');
        }
    }
    fs::write(package_dir.join("desc"), desc).expect("pacman desc should be written");
}

fn write_dpkg_package(root: &std::path::Path, name: &str, version: &str, files: &[&str]) {
    let status_path = root.join("var/lib/dpkg/status");
    let info_dir = root.join("var/lib/dpkg/info");
    fs::create_dir_all(&info_dir).expect("dpkg info dir should be created");
    let mut status = if status_path.exists() {
        fs::read_to_string(&status_path).expect("status should be readable")
    } else {
        String::new()
    };
    if !status.is_empty() && !status.ends_with("\n\n") {
        status.push('\n');
    }
    status.push_str(&format!(
        "Package: {name}\nStatus: install ok installed\nVersion: {version}\nArchitecture: amd64\nDepends: libc6 (>= 2.39)\n\n"
    ));
    fs::write(status_path, status).expect("dpkg status should be written");
    fs::write(info_dir.join(format!("{name}.list")), files.join("\n"))
        .expect("dpkg file list should be written");
}

fn installed_source_kind(root: &std::path::Path, package_name: &str) -> String {
    run_from_root(
        root,
        CommandRequest::new(
            vec!["info".to_owned()],
            vec![package_name.to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("info should succeed")
    .details
    .and_then(|details| details.get("installed").cloned())
    .and_then(|installed| installed.get("source_kind").cloned())
    .and_then(|value| value.as_str().map(str::to_owned))
    .expect("installed source kind should be present")
}
