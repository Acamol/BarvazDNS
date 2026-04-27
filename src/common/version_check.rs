use std::path::Path;

use crate::common::consts::LATEST_RELEASE_URL;

pub struct ReleaseInfo {
    pub tag: String,
    pub download_url: String,
}

/// Checks the latest GitHub release and returns the tag if it is newer than the current version.
pub fn check_for_update() -> Option<String> {
    let current: semver::Version = crate::common::strings::VERSION.parse().ok()?;

    let response = fetch_latest_release()?;
    let body = response.as_str().ok()?;
    newer_tag(body, &current)
}

/// Returns release info (tag + exe download URL) if a newer version is available.
pub fn get_update_info() -> Option<ReleaseInfo> {
    let current: semver::Version = crate::common::strings::VERSION.parse().ok()?;

    let response = fetch_latest_release()?;
    let body = response.as_str().ok()?;
    let tag = newer_tag(body, &current)?;
    let download_url = parse_exe_asset_url(body)?;
    Some(ReleaseInfo { tag, download_url })
}

/// Downloads a file from `url` and writes it to `dest`.
pub fn download_release(url: &str, dest: &Path) -> anyhow::Result<()> {
    let response = minreq::get(url)
        .with_header("User-Agent", "BarvazDNS")
        .with_timeout(120)
        .send()?;

    if response.status_code != 200 {
        anyhow::bail!("Download failed with HTTP status {}", response.status_code);
    }

    std::fs::write(dest, response.as_bytes())?;
    Ok(())
}

fn fetch_latest_release() -> Option<minreq::Response> {
    let response = minreq::get(LATEST_RELEASE_URL)
        .with_header("User-Agent", "BarvazDNS")
        .with_header("Accept", "application/vnd.github.v3+json")
        .with_timeout(5)
        .send()
        .ok()?;

    if response.status_code != 200 {
        return None;
    }

    Some(response)
}

fn parse_exe_asset_url(json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    let assets = parsed["assets"].as_array()?;
    assets
        .iter()
        .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with(".exe")))
        .and_then(|a| a["browser_download_url"].as_str())
        .map(String::from)
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
    fn parse_exe_asset_url_found() {
        let json = r#"{"assets":[{"name":"BarvazDNS.exe","browser_download_url":"https://example.com/BarvazDNS.exe"}]}"#;
        assert_eq!(
            parse_exe_asset_url(json),
            Some("https://example.com/BarvazDNS.exe".to_string())
        );
    }

    #[test]
    fn parse_exe_asset_url_no_exe() {
        let json = r#"{"assets":[{"name":"source.tar.gz","browser_download_url":"https://example.com/source.tar.gz"}]}"#;
        assert_eq!(parse_exe_asset_url(json), None);
    }

    #[test]
    fn parse_exe_asset_url_no_assets() {
        assert_eq!(parse_exe_asset_url(r#"{"tag_name":"v1.0.0"}"#), None);
    }

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
