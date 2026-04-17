use super::*;

pub(super) fn read_location_text(location: &str) -> Result<String, RepoError> {
    let bytes = if location.starts_with("http://") || location.starts_with("https://") {
        let response = ureq::get(location)
            .call()
            .map_err(|error| RepoError::Http(error.to_string()))?;
        let mut reader = response.into_reader();
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        buffer
    } else if let Some(path) = location.strip_prefix("file://") {
        fs::read(path)?
    } else {
        fs::read(location)?
    };

    let decoded = if location.ends_with(".zst") {
        zstd::stream::decode_all(bytes.as_slice())?
    } else {
        bytes
    };

    String::from_utf8(decoded)
        .map_err(|error| RepoError::Parse(format!("index content is not utf-8: {error}")))
}
