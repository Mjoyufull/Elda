use std::str::FromStr;

use elda_types::PackageVersion;

pub(crate) fn pkgver_from_tag(tag: &str) -> Option<String> {
    let (normalized, _) = elda_git::normalize_tag_version(tag);
    normalized
        .and_then(|version| PackageVersion::from_str(&version).ok())
        .map(|version| version.pkgver)
}

#[cfg(test)]
mod tests {
    use super::pkgver_from_tag;

    #[test]
    fn tag_versions_render_as_recipe_pkgver_values() {
        assert_eq!(pkgver_from_tag("v1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(pkgver_from_tag("v1.2.3-rc1").as_deref(), Some("1.2.3rc1"));
    }

    #[test]
    fn unparseable_tags_do_not_replace_explicit_recipe_fallbacks() {
        assert_eq!(pkgver_from_tag("!!!"), None);
    }
}
