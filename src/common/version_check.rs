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
    newer_tag(body, &current)
}

fn newer_tag(json: &str, current: &semver::Version) -> Option<String> {
    let tag = parse_tag_name(json)?;
    let latest: semver::Version = tag.strip_prefix('v').unwrap_or(&tag).parse().ok()?;

    if latest > *current { Some(tag) } else { None }
}

fn parse_tag_name(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    parsed["tag_name"].as_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tag_name_with_spaces() {
        let json = r#"{"tag_name": "v1.2.0", "name": "Release"}"#;
        assert_eq!(parse_tag_name(json), Some("v1.2.0".to_string()));
    }

    #[test]
    fn parse_tag_name_compact() {
        let json = r#"{"tag_name":"v1.0.0"}"#;
        assert_eq!(parse_tag_name(json), Some("v1.0.0".to_string()));
    }

    #[test]
    fn parse_tag_name_missing_field() {
        assert_eq!(parse_tag_name(r#"{"name": "Release"}"#), None);
    }

    #[test]
    fn parse_tag_name_empty_json() {
        assert_eq!(parse_tag_name("{}"), None);
    }

    #[test]
    fn newer_tag_returns_some_when_newer() {
        let current: semver::Version = "1.0.0".parse().unwrap();
        let json = r#"{"tag_name": "v2.0.0"}"#;
        assert_eq!(newer_tag(json, &current), Some("v2.0.0".to_string()));
    }

    #[test]
    fn newer_tag_returns_none_when_same() {
        let current: semver::Version = "1.0.0".parse().unwrap();
        let json = r#"{"tag_name": "v1.0.0"}"#;
        assert_eq!(newer_tag(json, &current), None);
    }

    #[test]
    fn newer_tag_returns_none_when_older() {
        let current: semver::Version = "2.0.0".parse().unwrap();
        let json = r#"{"tag_name": "v1.0.0"}"#;
        assert_eq!(newer_tag(json, &current), None);
    }

    #[test]
    fn newer_tag_without_v_prefix() {
        let current: semver::Version = "1.0.0".parse().unwrap();
        let json = r#"{"tag_name": "2.0.0"}"#;
        assert_eq!(newer_tag(json, &current), Some("2.0.0".to_string()));
    }

    #[test]
    fn newer_tag_with_invalid_version() {
        let current: semver::Version = "1.0.0".parse().unwrap();
        let json = r#"{"tag_name": "not-a-version"}"#;
        assert_eq!(newer_tag(json, &current), None);
    }

    #[test]
    fn newer_tag_with_missing_tag() {
        let current: semver::Version = "1.0.0".parse().unwrap();
        assert_eq!(newer_tag("{}", &current), None);
    }
}
