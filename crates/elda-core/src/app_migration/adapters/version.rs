#[derive(Debug, Clone)]
pub(crate) struct ForeignVersion {
    pub(crate) epoch: u64,
    pub(crate) pkgver: String,
    pub(crate) pkgrel: u64,
    pub(crate) raw: String,
}

pub(crate) fn parse_foreign_version(raw: &str) -> ForeignVersion {
    let (epoch, rest) = raw
        .split_once(':')
        .and_then(|(epoch, rest)| epoch.parse::<u64>().ok().map(|epoch| (epoch, rest)))
        .unwrap_or((0, raw));
    let (pkgver, pkgrel) = rest
        .rsplit_once('-')
        .and_then(|(version, rel)| rel.parse::<u64>().ok().map(|rel| (version, rel)))
        .unwrap_or((rest, 1));

    ForeignVersion {
        epoch,
        pkgver: sanitize_pkgver(pkgver),
        pkgrel,
        raw: raw.to_owned(),
    }
}

fn sanitize_pkgver(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|c| if c.is_ascii_whitespace() { '_' } else { c })
        .collect::<String>();
    if sanitized.is_empty() {
        "0".to_owned()
    } else {
        sanitized
    }
}
