use elda_types::NamedConstraint;

pub(super) fn satisfies(constraint: &NamedConstraint) -> bool {
    if constraint.is_versioned() {
        return false;
    }

    match constraint.name.as_str() {
        "glibc" => cfg!(target_env = "gnu"),
        // Musl hosts may package GCC runtimes separately, so only GNU targets treat these as
        // intrinsic platform capabilities. Other targets resolve them through normal providers.
        "libgcc" | "libstdc++" => cfg!(target_env = "gnu"),
        "musl" => cfg!(target_env = "musl"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::satisfies;
    use elda_types::NamedConstraint;

    #[test]
    fn host_platform_only_satisfies_matching_libc_capability() {
        assert_eq!(satisfies(&dependency("glibc")), cfg!(target_env = "gnu"));
        assert_eq!(satisfies(&dependency("musl")), cfg!(target_env = "musl"));
        assert!(!satisfies(&dependency("python")));
        assert_eq!(satisfies(&dependency("libgcc")), cfg!(target_env = "gnu"));
        assert_eq!(
            satisfies(&dependency("libstdc++")),
            cfg!(target_env = "gnu")
        );
        assert!(!satisfies(&dependency("glibc>=2.39")));
    }

    fn dependency(value: &str) -> NamedConstraint {
        NamedConstraint::parse_dependency(value).expect("constraint should parse")
    }
}
