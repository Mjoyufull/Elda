use std::fs;
use std::path::Path;

pub(in crate::tests) fn write_prefix_config(root: &Path, prefix: &str) {
    write_prefix_config_with_extras(root, prefix, true, false, "");
}

pub(in crate::tests) fn write_prefix_config_with_recommends(
    root: &Path,
    prefix: &str,
    install_recommends: bool,
) {
    write_prefix_config_with_extras(root, prefix, install_recommends, false, "");
}

pub(in crate::tests) fn write_prefix_config_with_policy(
    root: &Path,
    prefix: &str,
    install_recommends: bool,
    refresh_weak_deps: bool,
) {
    write_prefix_config_with_extras(root, prefix, install_recommends, refresh_weak_deps, "");
}

pub(in crate::tests) fn write_prefix_config_with_extras(
    root: &Path,
    prefix: &str,
    install_recommends: bool,
    refresh_weak_deps: bool,
    extras: &str,
) {
    let config_dir = root.join("etc/elda");
    fs::create_dir_all(&config_dir).expect("config dir should exist");
    fs::write(
        config_dir.join("config.toml"),
        format!(
            "[defaults]\nprefix = \"{prefix}\"\nbuild_mode = \"host\"\nremote = \"yoka-main\"\ninstall_recommends = {install_recommends}\nrefresh_weak_deps = {refresh_weak_deps}\n{extras}"
        ),
    )
    .expect("config should be written");
}
