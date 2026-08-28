use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    #[serde(rename = "browser_download_url")]
    pub browser_download_url: String,
    /// GitHub Release Asset digest (e.g. "sha256:4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945")
    pub digest: Option<String>,
}

impl ReleaseAsset {
    /// Extracts the raw SHA-256 hex string from the asset digest field if present
    pub fn parse_sha256_digest(&self) -> Result<Option<String>, String> {
        if let Some(ref d) = self.digest {
            let trimmed = d.trim();
            if let Some(hex) = trimmed.strip_prefix("sha256:") {
                let hex_clean = hex.trim().to_lowercase();
                if hex_clean.len() == 64 && hex_clean.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Ok(Some(hex_clean));
                } else {
                    return Err(format!(
                        "Malformed sha256 digest in GitHub metadata: '{}'",
                        d
                    ));
                }
            } else if trimmed.len() == 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(Some(trimmed.to_lowercase()));
            } else {
                return Err(format!(
                    "Unsupported or malformed digest format in GitHub metadata: '{}'",
                    d
                ));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

pub struct GithubClient;

impl GithubClient {
    pub async fn fetch_latest_release(repo: &str) -> Result<GithubRelease, String> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("AetherDesktop/0.1.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch release metadata from {}: {}", url, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "GitHub API returned status {} for {}",
                resp.status(),
                repo
            ));
        }

        let release: GithubRelease = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse GitHub release JSON: {}", e))?;

        Ok(release)
    }

    /// Strict matching for Aether official release assets (Windows x86_64 only).
    pub fn find_aether_asset(release: &GithubRelease) -> Result<&ReleaseAsset, String> {
        release
            .assets
            .iter()
            .find(|asset| {
                let name = asset.name.to_lowercase();
                name.starts_with("aether")
                    && name.contains("windows")
                    && (name.contains("x86_64") || name.contains("amd64"))
                    && name.ends_with(".zip")
            })
            .ok_or_else(|| {
                format!(
                    "No supported Windows x86_64 asset ('aether-windows-x86_64.zip') found in CluvexStudio/Aether release {}",
                    release.tag_name
                )
            })
    }

    /// Strict matching for sing-box official release assets (Windows amd64 only, non-legacy).
    pub fn find_singbox_asset(release: &GithubRelease) -> Result<&ReleaseAsset, String> {
        release
            .assets
            .iter()
            .find(|asset| {
                let name = asset.name.to_lowercase();
                name.starts_with("sing-box")
                    && name.contains("windows-amd64")
                    && name.ends_with(".zip")
                    && !name.contains("legacy")
            })
            .ok_or_else(|| {
                format!(
                    "No supported Windows amd64 asset ('sing-box-*-windows-amd64.zip') found in SagerNet/sing-box release {}",
                    release.tag_name
                )
            })
    }

    pub fn find_companion_checksum_asset<'a>(
        release: &'a GithubRelease,
        target_asset_name: &str,
    ) -> Option<&'a ReleaseAsset> {
        let direct_name = format!("{}.sha256", target_asset_name);
        if let Some(asset) = release
            .assets
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(&direct_name))
        {
            return Some(asset);
        }

        release.assets.iter().find(|a| {
            let n = a.name.to_lowercase();
            n == "sha256sums.txt" || n == "checksums.txt" || n == "sha256sum.txt"
        })
    }

    pub async fn fetch_checksum_text(url: &str) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("AetherDesktop/0.1.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .map_err(|e| format!("Failed to create client: {}", e))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch checksum from {}: {}", url, e))?;

        if !resp.status().is_success() {
            return Err(format!("Checksum URL returned HTTP {}", resp.status()));
        }

        resp.text().await.map_err(|e| e.to_string())
    }
}
