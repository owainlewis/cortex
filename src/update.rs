use std::{
    cmp::Ordering,
    io,
    process::{Command, Stdio},
};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/owainlewis/cortex/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    NoRelease,
    Release {
        latest_tag: String,
        ordering: Ordering,
    },
}

impl UpdateStatus {
    pub fn latest_tag(&self) -> Option<&str> {
        match self {
            UpdateStatus::NoRelease => None,
            UpdateStatus::Release { latest_tag, .. } => Some(latest_tag),
        }
    }

    pub fn has_update(&self) -> bool {
        match self {
            UpdateStatus::NoRelease => false,
            UpdateStatus::Release { ordering, .. } => *ordering == Ordering::Less,
        }
    }
}

pub fn check_latest(current_version: &str) -> Result<UpdateStatus, String> {
    let Some(response) = fetch_latest_release()? else {
        return Ok(UpdateStatus::NoRelease);
    };
    let latest_tag = release_tag_name(&response)
        .ok_or_else(|| "GitHub release response did not include tag_name".to_string())?;
    compare_versions(current_version, &latest_tag).map(|ordering| UpdateStatus::Release {
        latest_tag,
        ordering,
    })
}

fn fetch_latest_release() -> Result<Option<String>, String> {
    let output = Command::new("curl")
        .args(curl_args())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(curl_launch_error)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        return Err(if message.is_empty() {
            format!(
                "GitHub latest release request failed with {}",
                output.status
            )
        } else {
            format!("GitHub latest release request failed: {message}")
        });
    }

    let response = String::from_utf8(output.stdout)
        .map_err(|error| format!("GitHub release response was not valid UTF-8: {error}"))?;
    parse_latest_response(&response)
}

fn curl_args() -> [&'static str; 9] {
    [
        "-sS",
        "-L",
        "--connect-timeout",
        "5",
        "--max-time",
        "20",
        "-w",
        "\n%{http_code}",
        LATEST_RELEASE_URL,
    ]
}

fn parse_latest_response(response: &str) -> Result<Option<String>, String> {
    let (body, status) = response
        .rsplit_once('\n')
        .ok_or_else(|| "GitHub release response did not include an HTTP status".to_string())?;

    match status {
        "200" => Ok(Some(body.to_string())),
        "404" => Ok(None),
        _ => {
            let message = body.trim();
            Err(if message.is_empty() {
                format!("GitHub latest release request returned HTTP {status}")
            } else {
                format!("GitHub latest release request returned HTTP {status}: {message}")
            })
        }
    }
}

fn curl_launch_error(error: io::Error) -> String {
    if error.kind() == io::ErrorKind::NotFound {
        "curl is required to check for updates".to_string()
    } else {
        format!("failed to run curl: {error}")
    }
}

fn release_tag_name(response: &str) -> Option<String> {
    let key = "\"tag_name\"";
    let key_start = response.find(key)?;
    let after_key = &response[key_start + key.len()..];
    let after_colon = after_key.split_once(':')?.1.trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let mut tag = String::new();
    let mut escaped = false;

    for ch in after_quote.chars() {
        if escaped {
            tag.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Some(tag),
            _ => tag.push(ch),
        }
    }

    None
}

fn compare_versions(current: &str, latest_tag: &str) -> Result<Ordering, String> {
    let current = semantic_version(current)
        .ok_or_else(|| format!("current version is not a semantic version: {current}"))?;
    let latest = semantic_version(latest_tag)
        .ok_or_else(|| format!("latest release tag is not a semantic version: {latest_tag}"))?;
    Ok(current.cmp(&latest))
}

fn semantic_version(version: &str) -> Option<[u64; 3]> {
    let normalized = version
        .trim()
        .strip_prefix('v')
        .or_else(|| version.trim().strip_prefix('V'))
        .unwrap_or_else(|| version.trim());
    let core = normalized
        .split_once(['-', '+'])
        .map(|(core, _)| core)
        .unwrap_or(normalized);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some([major, minor, patch])
}

#[cfg(test)]
mod tests {
    use super::{
        compare_versions, curl_args, parse_latest_response, release_tag_name, semantic_version,
        UpdateStatus,
    };
    use std::cmp::Ordering;

    #[test]
    fn parses_release_tag_name_from_response() {
        let response = r#"{"name":"Cortex","tag_name":"v1.2.3"}"#;

        assert_eq!(release_tag_name(response), Some("v1.2.3".to_string()));
    }

    #[test]
    fn parses_escaped_release_tag_name() {
        let response = r#"{"tag_name":"v1.2.3-alpha\"quoted"}"#;

        assert_eq!(
            release_tag_name(response),
            Some("v1.2.3-alpha\"quoted".to_string())
        );
    }

    #[test]
    fn compares_current_version_with_latest_release() {
        assert_eq!(compare_versions("0.1.0", "v0.2.0"), Ok(Ordering::Less));
        assert_eq!(compare_versions("0.1.0", "v0.1.0"), Ok(Ordering::Equal));
        assert_eq!(compare_versions("0.1.0", "v0.0.9"), Ok(Ordering::Greater));
    }

    #[test]
    fn compares_release_tags_with_suffixes() {
        assert_eq!(
            compare_versions("0.1.0", "v9.9.9-version-test.20260622"),
            Ok(Ordering::Less)
        );
    }

    #[test]
    fn rejects_invalid_version_shapes() {
        assert!(semantic_version("not-a-version").is_none());
        assert!(semantic_version("v1.2").is_none());
        assert!(semantic_version("v1.2.3.4").is_none());
        assert!(compare_versions("0.1.0", "latest").is_err());
    }

    #[test]
    fn parses_successful_latest_release_response() {
        let response = "{\"tag_name\":\"v1.2.3\"}\n200";

        assert_eq!(
            parse_latest_response(response),
            Ok(Some("{\"tag_name\":\"v1.2.3\"}".to_string()))
        );
    }

    #[test]
    fn treats_github_latest_release_404_as_no_release() {
        let response = "{\"message\":\"Not Found\"}\n404";

        assert_eq!(parse_latest_response(response), Ok(None));
    }

    #[test]
    fn reports_unexpected_http_statuses() {
        let error = parse_latest_response("{\"message\":\"rate limited\"}\n403").unwrap_err();

        assert!(error.contains("HTTP 403"));
        assert!(error.contains("rate limited"));
    }

    #[test]
    fn curl_args_include_network_timeouts() {
        let args = curl_args();

        assert!(args
            .windows(2)
            .any(|pair| pair == ["--connect-timeout", "5"]));
        assert!(args.windows(2).any(|pair| pair == ["--max-time", "20"]));
    }

    #[test]
    fn reports_update_availability_from_ordering() {
        let newer = UpdateStatus::Release {
            latest_tag: "v0.2.0".to_string(),
            ordering: Ordering::Less,
        };
        let same = UpdateStatus::Release {
            latest_tag: "v0.1.0".to_string(),
            ordering: Ordering::Equal,
        };

        assert!(newer.has_update());
        assert!(!same.has_update());
        assert!(!UpdateStatus::NoRelease.has_update());
    }
}
