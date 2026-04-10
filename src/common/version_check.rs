use crate::common::consts::LATEST_RELEASE_URL;

/// Checks the latest GitHub release and returns the tag if it is newer than the current version.
pub fn check_for_update() -> Option<String> {
    let current: semver::Version = crate::common::strings::VERSION.parse().ok()?;

    let response = minreq::get(LATEST_RELEASE_URL)
        .with_header("User-Agent", "BarvazDNS")
        .with_header("Accept", "application/vnd.github.v3+json")
        .with_timeout(5)
        .send()
        .ok()?;

    if response.status_code != 200 {
        return None;
    }

    let body = response.as_str().ok()?;
    let tag = parse_tag_name(body)?;
    let latest: semver::Version = tag.strip_prefix('v').unwrap_or(tag).parse().ok()?;

    if latest > current {
        Some(tag.to_string())
    } else {
        None
    }
}

fn parse_tag_name(json: &str) -> Option<&str> {
    let marker = "\"tag_name\":";
    let idx = json.find(marker)? + marker.len();
    let rest = json[idx..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag_name() {
        let json = r#"{"tag_name": "v1.2.0", "name": "Release"}"#;
        assert_eq!(parse_tag_name(json), Some("v1.2.0"));

        let json = r#"{"tag_name":"v1.0.0"}"#;
        assert_eq!(parse_tag_name(json), Some("v1.0.0"));
    }
}
