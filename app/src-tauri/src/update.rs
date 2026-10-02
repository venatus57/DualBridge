//! Update checks against the project's GitHub releases.
//!
//! The app asks GitHub for the latest releases, and when one is newer than
//! the running version it offers to install it: the installer is downloaded
//! over HTTPS from the release, checked against the SHA-256 digest GitHub
//! publishes for it, then started, and the app quits so it can be replaced.

use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const RELEASES_API: &str = "https://api.github.com/repos/venatus57/DualBridge/releases?per_page=10";
const USER_AGENT: &str = concat!("DualBridge/", env!("CARGO_PKG_VERSION"));

/// A newer release that can be installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    /// Version without the leading `v`, e.g. `0.2.0`.
    pub version: String,
    /// Release page, for the release notes.
    pub page: String,
    /// Installer for this platform, if the release has one.
    pub asset: Option<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    /// Lowercase hex SHA-256, when GitHub provides it.
    pub sha256: Option<String>,
}

/// `(major, minor, patch)` from `v1.2.3` or `1.2.3` (a `-beta` style suffix
/// is ignored).
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let v = (
        parts.next()??,
        parts.next()??,
        parts.next().unwrap_or(Some(0))?,
    );
    parts.next().is_none().then_some(v)
}

/// Installer file name suffix for this platform.
fn asset_suffix() -> Option<&'static str> {
    if cfg!(target_os = "windows") {
        Some("_x64-setup.exe")
    } else if cfg!(target_os = "macos") {
        Some("_universal.dmg")
    } else {
        None
    }
}

/// Picks the newest published release above `current` from the GitHub
/// releases API response.
pub fn pick_update(releases: &Value, current: &str, suffix: Option<&str>) -> Option<UpdateInfo> {
    let current = parse_version(current)?;
    let (version, release) = releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(true))
        .filter_map(|r| Some((parse_version(r["tag_name"].as_str()?)?, r)))
        .filter(|(v, _)| *v > current)
        .max_by_key(|(v, _)| *v)?;
    let asset = suffix.and_then(|suffix| {
        release["assets"].as_array()?.iter().find_map(|a| {
            let name = a["name"].as_str()?;
            name.ends_with(suffix).then(|| Asset {
                name: name.to_string(),
                url: a["browser_download_url"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                size: a["size"].as_u64().unwrap_or(0),
                sha256: a["digest"]
                    .as_str()
                    .and_then(|d| d.strip_prefix("sha256:"))
                    .map(str::to_ascii_lowercase),
            })
        })
    });
    Some(UpdateInfo {
        version: format!("{}.{}.{}", version.0, version.1, version.2),
        page: release["html_url"].as_str().unwrap_or_default().to_string(),
        asset,
    })
}

/// Asks GitHub for a newer release than the running one.
pub fn check() -> Result<Option<UpdateInfo>, String> {
    let mut resp = ureq::get(RELEASES_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| e.to_string())?;
    let text = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let releases: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(pick_update(
        &releases,
        env!("CARGO_PKG_VERSION"),
        asset_suffix(),
    ))
}

/// Downloads the installer to a temporary folder, calling `progress` with
/// the percentage, and checks its digest. Returns the file's path.
pub fn download(asset: &Asset, mut progress: impl FnMut(u8)) -> Result<PathBuf, String> {
    // Only ever download from this project's GitHub releases.
    if !asset
        .url
        .starts_with("https://github.com/venatus57/DualBridge/releases/download/")
        || asset.name.contains(['/', '\\'])
    {
        return Err("unexpected download address".into());
    }
    let dir = std::env::temp_dir().join("DualBridge-update");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(&asset.name);

    let resp = ureq::get(&asset.url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let mut reader = resp.into_body().into_reader();
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    let mut last_pct = u8::MAX;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if let Some(pct) = (done * 100).checked_div(asset.size) {
            let pct = pct.min(100) as u8;
            if pct != last_pct {
                last_pct = pct;
                progress(pct);
            }
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    if let Some(expected) = &asset.sha256 {
        let got: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if &got != expected {
            let _ = std::fs::remove_file(&path);
            return Err("the download is corrupted (checksum mismatch)".into());
        }
    }
    Ok(path)
}

/// Starts the downloaded installer (Windows) or opens the disk image (macOS).
pub fn launch(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = path;
        Err("automatic updates are not available on this platform".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.10.3"), Some((1, 10, 3)));
        assert_eq!(parse_version("v1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("v0.3.0-beta.1"), Some((0, 3, 0)));
        assert_eq!(parse_version("nightly"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert!(parse_version("0.10.0") > parse_version("0.9.9"));
    }

    fn release(tag: &str, draft: bool) -> Value {
        json!({
            "tag_name": tag,
            "draft": draft,
            "prerelease": true,
            "html_url": format!("https://github.com/venatus57/DualBridge/releases/tag/{tag}"),
            "assets": [
                {
                    "name": format!("DualBridge_{}_x64-setup.exe", &tag[1..]),
                    "size": 1000,
                    "digest": "sha256:ABCDEF",
                    "browser_download_url": format!("https://github.com/venatus57/DualBridge/releases/download/{tag}/x.exe"),
                },
                { "name": "DualBridge_universal.dmg", "size": 5 },
            ],
        })
    }

    #[test]
    fn picks_the_newest_published_release() {
        let releases = json!([
            release("v0.1.0", false),
            release("v0.3.0", true),
            release("v0.2.1", false),
            release("v0.2.0", false),
        ]);
        let u = pick_update(&releases, "0.1.0", Some("_x64-setup.exe")).unwrap();
        assert_eq!(u.version, "0.2.1");
        assert!(u.page.ends_with("/v0.2.1"));
        let a = u.asset.unwrap();
        assert_eq!(a.name, "DualBridge_0.2.1_x64-setup.exe");
        assert_eq!(a.size, 1000);
        assert_eq!(a.sha256.as_deref(), Some("abcdef"));

        // Up to date, or no installer for this platform.
        assert_eq!(
            pick_update(&releases, "0.2.1", Some("_x64-setup.exe")),
            None
        );
        assert_eq!(pick_update(&releases, "0.2.1", None), None);
        let u = pick_update(&releases, "0.2.0", Some("_arm64.msi")).unwrap();
        assert_eq!(u.asset, None);
        assert_eq!(
            pick_update(&json!({"message": "rate limited"}), "0.1.0", None),
            None
        );
    }

    #[test]
    fn refuses_other_download_addresses() {
        let asset = Asset {
            name: "x.exe".into(),
            url: "https://example.com/x.exe".into(),
            size: 1,
            sha256: None,
        };
        assert!(download(&asset, |_| {}).is_err());
        let asset = Asset {
            name: "../x.exe".into(),
            url: "https://github.com/venatus57/DualBridge/releases/download/v1/x.exe".into(),
            size: 1,
            sha256: None,
        };
        assert!(download(&asset, |_| {}).is_err());
    }
}
