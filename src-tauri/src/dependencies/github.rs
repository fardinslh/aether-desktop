use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
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

    pub fn find_asset<'a>(release: &'a GithubRelease, pattern: &str) -> Option<&'a ReleaseAsset> {
        let pattern_lower = pattern.to_lowercase();
        release
            .assets
            .iter()
            .find(|asset| asset.name.to_lowercase().contains(&pattern_lower))
    }
}
