use crate::models::AppUpdateCheck;
use chrono::Utc;
use semver::Version;
use serde::Deserialize;
use std::io::Read;
use std::time::Duration;
use url::Url;

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/NouraldinFarge/gamevault/releases/latest";
const MAX_RESPONSE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Deserialize)]
struct LatestRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
}

pub fn check() -> Result<AppUpdateCheck, String> {
    let response = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(8))
        .build()
        .get(LATEST_RELEASE_API)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", "GameVault-update-check")
        .call()
        .map_err(|_| "GitHub's release service could not be reached.".to_string())?;
    if response.status() != 200 {
        return Err("GitHub did not return a latest release.".to_string());
    }
    let mut body = String::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|_| "The release response could not be read.".to_string())?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("The release response exceeded the safe size limit.".to_string());
    }
    let release: LatestRelease = serde_json::from_str(&body)
        .map_err(|_| "GitHub returned an unexpected release response.".to_string())?;
    if !is_approved_release_url(&release.html_url) {
        return Err("GitHub returned an unapproved release URL.".to_string());
    }
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let latest_version = normalize_version(&release.tag_name)?;
    let update_available =
        compare_versions(&latest_version, &current_version)? == std::cmp::Ordering::Greater;
    Ok(AppUpdateCheck {
        current_version,
        latest_version,
        update_available,
        release_url: release.html_url,
        published_at: release.published_at,
        checked_at: Utc::now().to_rfc3339(),
    })
}

pub fn is_approved_release_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let segments = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && segments.len() == 5
        && segments[0] == "NouraldinFarge"
        && segments[1] == "gamevault"
        && segments[2] == "releases"
        && segments[3] == "tag"
        && segments[4].starts_with('v')
}

fn normalize_version(value: &str) -> Result<String, String> {
    let normalized = value.trim().trim_start_matches('v');
    parse_version(normalized)?;
    Ok(normalized.to_string())
}

fn compare_versions(left: &str, right: &str) -> Result<std::cmp::Ordering, String> {
    Ok(parse_version(left)?.cmp(&parse_version(right)?))
}

fn parse_version(value: &str) -> Result<Version, String> {
    Version::parse(value)
        .map_err(|_| "The release tag is not a supported semantic version.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_versions_compare_numerically() {
        assert_eq!(
            compare_versions("0.10.0", "0.9.9").expect("compare"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.0.0", "1.0.0").expect("compare"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_versions("0.4.0", "0.4.0-dev.0").expect("compare"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("0.4.0-dev.0", "0.3.5").expect("compare"),
            std::cmp::Ordering::Greater
        );
        assert!(compare_versions("1.0", "1.0.0").is_err());
    }

    #[test]
    fn release_url_allowlist_is_exact() {
        assert!(is_approved_release_url(
            "https://github.com/NouraldinFarge/gamevault/releases/tag/v0.3.5"
        ));
        assert!(!is_approved_release_url(
            "https://example.com/NouraldinFarge/gamevault/releases/tag/v0.3.5"
        ));
        assert!(!is_approved_release_url(
            "https://github.com/NouraldinFarge/another/releases/tag/v0.3.5"
        ));
    }

    #[test]
    fn leading_v_is_removed_from_release_tags() {
        assert_eq!(normalize_version("v1.2.3").expect("version"), "1.2.3");
    }
}
