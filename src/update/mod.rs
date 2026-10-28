//! Release discovery and explicit self-updates. Normal builds never wait for the network.

mod cache;
mod download;

use serde_json::Value;
use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const API: &str = "https://api.github.com/repos/farion/laml/releases/latest";
const BASE: &str = "https://github.com/farion/laml/releases/download/";

#[derive(Clone, Debug)]
struct Asset {
    name: String,
    url: String,
    digest: Option<String>,
}
#[derive(Clone, Debug)]
struct Release {
    tag: String,
    version: [u64; 3],
    assets: Vec<Asset>,
}
impl Release {
    fn asset(&self, name: &str) -> Result<&Asset, String> {
        self.assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| format!("release {} has no asset '{name}'", self.tag))
    }
}

fn version(input: &str) -> Option<[u64; 3]> {
    let value = input.strip_prefix('v').unwrap_or(input);
    let pieces: Vec<_> = value.split('.').collect();
    if pieces.len() != 3
        || pieces.iter().any(|part| {
            part.is_empty()
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|c| c.is_ascii_digit())
        })
    {
        return None;
    }
    Some([
        pieces[0].parse().ok()?,
        pieces[1].parse().ok()?,
        pieces[2].parse().ok()?,
    ])
}

fn parse_release(bytes: &[u8]) -> Result<Release, String> {
    let data: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid release metadata: {e}"))?;
    let tag = data["tag_name"]
        .as_str()
        .ok_or("release has no version tag")?;
    if !tag.starts_with('v') {
        return Err("release tag must start with 'v'".into());
    }
    let parsed = version(tag).ok_or("release tag is not a stable version")?;
    let assets = data["assets"]
        .as_array()
        .ok_or("release has no assets")?
        .iter()
        .filter_map(|asset| {
            let name = asset["name"].as_str()?;
            let url = asset["browser_download_url"].as_str()?;
            // Never follow a URL supplied by release metadata to another repository.
            if url != format!("{BASE}{tag}/{name}") {
                return None;
            }
            Some(Asset {
                name: name.into(),
                url: url.into(),
                digest: asset["digest"].as_str().map(str::to_owned),
            })
        })
        .collect();
    Ok(Release {
        tag: tag.into(),
        version: parsed,
        assets,
    })
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}
fn fetch(url: &str, limit: u64, timeout: Duration) -> Result<Vec<u8>, String> {
    let mut response = agent(timeout)
        .get(url)
        .header("User-Agent", "laml-updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("cannot fetch release: {e}"))?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|e| format!("cannot read release: {e}"))
}
fn latest() -> Result<Option<Release>, String> {
    match fetch(API, 1_000_000, Duration::from_secs(5)) {
        Ok(bytes) => parse_release(&bytes).map(Some),
        Err(e) if e.contains("http status: 404") => Ok(None),
        Err(e) => Err(e),
    }
}

fn target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}

/// Reads a small local cache. At most once a day, starts a detached check.
pub fn notify_in_background() {
    cache::notify_in_background();
}
/// Hidden child entry point. No logging and no effect on a running build.
pub fn refresh_cache() {
    cache::refresh();
}

/// An explicit `laml update` invocation. Never called by the background check.
pub fn update() -> Result<String, String> {
    let target = target().ok_or("no release binary for this OS/architecture")?;
    let release = latest()?.ok_or("no published LAML release is available")?;
    if release.version <= version(VERSION).expect("valid package version") {
        return Ok(format!("LAML v{VERSION} is already up to date"));
    }
    let archive_name = format!(
        "laml-{}-{target}.{}",
        release.tag,
        if cfg!(windows) { "zip" } else { "tar.gz" }
    );
    let archive = release.asset(&archive_name)?;
    let checksums = release.asset("SHA256SUMS")?;
    let bytes = fetch(&archive.url, 64_000_000, Duration::from_secs(120))?;
    let sums = fetch(&checksums.url, 65_536, Duration::from_secs(10))?;
    download::verify(&bytes, &sums, &archive_name, archive.digest.as_deref())?;
    download::install(&bytes, &release.tag, target)?;
    if cfg!(windows) {
        Ok(format!(
            "LAML {} is staged; restart laml after this process exits",
            release.tag
        ))
    } else {
        Ok(format!("LAML {} installed", release.tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_numeric_and_strict() {
        assert!(version("v1.10.0") > version("v1.9.9"));
        for bad in ["v1.0", "v1.0.0-rc1", "v01.0.0", "v1.x.0", "v1.0.0.0"] {
            assert!(version(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn metadata_accepts_only_matching_release_assets() {
        let source=br#"{"tag_name":"v1.1.0","assets":[{"name":"SHA256SUMS","browser_download_url":"https://github.com/farion/laml/releases/download/v1.1.0/SHA256SUMS","digest":"sha256:abc"}]}"#;
        let release = parse_release(source).unwrap();
        assert_eq!(release.version, [1, 1, 0]);
        assert_eq!(
            release.asset("SHA256SUMS").unwrap().digest.as_deref(),
            Some("sha256:abc")
        );
    }
    #[test]
    fn metadata_rejects_asset_urls_outside_the_release() {
        let source=br#"{"tag_name":"v1.1.0","assets":[{"name":"laml.zip","browser_download_url":"https://evil.example/laml.zip"}]}"#;
        assert!(parse_release(source).unwrap().assets.is_empty());
    }
}
