use super::github::GithubClient;
use crate::settings::SettingsStorage;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub component: String,
    pub status: String,
    pub percent: u8,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyStatus {
    pub aether_installed: bool,
    pub aether_path: String,
    pub aether_version: Option<String>,
    pub singbox_installed: bool,
    pub singbox_path: String,
    pub singbox_version: Option<String>,
}

pub struct DependencyManager;

impl DependencyManager {
    pub fn get_base_dependencies_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|b| {
                b.data_local_dir()
                    .join("AetherDesktop")
                    .join("dependencies")
            })
            .unwrap_or_else(|| PathBuf::from("./dependencies"))
    }

    pub fn check_status() -> DependencyStatus {
        let settings = SettingsStorage::load();

        let aether_path = settings.aether.executable_path.clone();
        let aether_exists = !aether_path.is_empty() && Path::new(&aether_path).exists();
        let aether_version = if aether_exists {
            Some("Ready".to_string())
        } else {
            None
        };

        let singbox_path = settings.sing_box.executable_path.clone();
        let singbox_exists = !singbox_path.is_empty() && Path::new(&singbox_path).exists();
        let singbox_version = if singbox_exists {
            // Try querying sing-box version
            let mut cmd = Command::new(&singbox_path);
            cmd.arg("version")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }
            if let Ok(output) = cmd.output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().next().map(|l| l.to_string())
            } else {
                Some("Ready".to_string())
            }
        } else {
            None
        };

        DependencyStatus {
            aether_installed: aether_exists,
            aether_path,
            aether_version,
            singbox_installed: singbox_exists,
            singbox_path,
            singbox_version,
        }
    }

    pub async fn install_aether(app: Option<&AppHandle>) -> Result<String, String> {
        Self::emit_progress(
            app,
            "aether",
            "Resolving latest release from GitHub...",
            0,
            0,
            0,
        );

        let release = GithubClient::fetch_latest_release("CluvexStudio/Aether").await?;
        let asset = GithubClient::find_asset(&release, "aether-windows-x86_64.zip")
            .or_else(|| GithubClient::find_asset(&release, "windows-x86_64.zip"))
            .or_else(|| GithubClient::find_asset(&release, ".zip"))
            .ok_or_else(|| {
                format!(
                    "No compatible Windows x86_64 zip asset found in Aether release {}",
                    release.tag_name
                )
            })?;

        let base_dir = Self::get_base_dependencies_dir()
            .join("aether")
            .join(&release.tag_name);
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| format!("Failed to create dependency folder {:?}: {}", base_dir, e))?;
        }

        // Stream download
        let temp_archive_path = base_dir.join("aether-download.tmp.zip");
        Self::download_file_with_progress(
            app,
            "aether",
            &asset.browser_download_url,
            &temp_archive_path,
            asset.size,
        )
        .await?;

        // Extract with zip-slip safety
        Self::emit_progress(
            app,
            "aether",
            "Extracting files securely...",
            90,
            asset.size,
            asset.size,
        );
        Self::extract_zip_safely(&temp_archive_path, &base_dir)?;
        let _ = std::fs::remove_file(&temp_archive_path);

        // Find aether.exe in extracted folder
        Self::emit_progress(
            app,
            "aether",
            "Locating and verifying executable...",
            98,
            asset.size,
            asset.size,
        );
        let found_exe = Self::find_executable_in_dir(&base_dir, "aether.exe")
            .ok_or_else(|| format!("aether.exe not found after extracting {:?}", base_dir))?;

        // Update settings
        let exe_str = found_exe.to_string_lossy().to_string();
        let mut settings = SettingsStorage::load();
        settings.aether.executable_path = exe_str.clone();
        SettingsStorage::save(&settings)?;

        Self::emit_progress(
            app,
            "aether",
            "Installation complete",
            100,
            asset.size,
            asset.size,
        );
        Ok(exe_str)
    }

    pub async fn install_singbox(app: Option<&AppHandle>) -> Result<String, String> {
        Self::emit_progress(
            app,
            "sing-box",
            "Resolving latest release from GitHub...",
            0,
            0,
            0,
        );

        let release = GithubClient::fetch_latest_release("SagerNet/sing-box").await?;
        let asset = GithubClient::find_asset(&release, "windows-amd64.zip")
            .or_else(|| GithubClient::find_asset(&release, "windows-x86_64.zip"))
            .ok_or_else(|| {
                format!(
                    "No compatible Windows amd64 zip asset found in sing-box release {}",
                    release.tag_name
                )
            })?;

        let base_dir = Self::get_base_dependencies_dir()
            .join("sing-box")
            .join(&release.tag_name);
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| format!("Failed to create dependency folder {:?}: {}", base_dir, e))?;
        }

        // Stream download
        let temp_archive_path = base_dir.join("sing-box-download.tmp.zip");
        Self::download_file_with_progress(
            app,
            "sing-box",
            &asset.browser_download_url,
            &temp_archive_path,
            asset.size,
        )
        .await?;

        // Extract with zip-slip safety
        Self::emit_progress(
            app,
            "sing-box",
            "Extracting files securely...",
            90,
            asset.size,
            asset.size,
        );
        Self::extract_zip_safely(&temp_archive_path, &base_dir)?;
        let _ = std::fs::remove_file(&temp_archive_path);

        // Find sing-box.exe in extracted folder
        Self::emit_progress(
            app,
            "sing-box",
            "Locating and verifying executable...",
            98,
            asset.size,
            asset.size,
        );
        let found_exe = Self::find_executable_in_dir(&base_dir, "sing-box.exe")
            .ok_or_else(|| format!("sing-box.exe not found after extracting {:?}", base_dir))?;

        // Validate sing-box execution
        let mut cmd = Command::new(&found_exe);
        cmd.arg("version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run sing-box version check: {}", e))?;
        if !output.status.success() {
            return Err("sing-box binary verification failed on execution test".to_string());
        }

        // Update settings
        let exe_str = found_exe.to_string_lossy().to_string();
        let mut settings = SettingsStorage::load();
        settings.sing_box.executable_path = exe_str.clone();
        SettingsStorage::save(&settings)?;

        Self::emit_progress(
            app,
            "sing-box",
            "Installation complete",
            100,
            asset.size,
            asset.size,
        );
        Ok(exe_str)
    }

    async fn download_file_with_progress(
        app: Option<&AppHandle>,
        component: &str,
        url: &str,
        dest_path: &Path,
        expected_size: u64,
    ) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .user_agent("AetherDesktop/0.1.0 (Windows NT 10.0; Win64; x64)")
            .build()
            .map_err(|e| format!("Failed to create client: {}", e))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "Download failed with HTTP status {}",
                resp.status()
            ));
        }

        let total_size = resp.content_length().unwrap_or(expected_size);
        let mut file = File::create(dest_path).map_err(|e| {
            format!(
                "Failed to create temporary download file {:?}: {}",
                dest_path, e
            )
        })?;

        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Error downloading chunk: {}", e))?;
            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write chunk to file: {}", e))?;
            downloaded += chunk.len() as u64;

            let percent = if total_size > 0 {
                ((downloaded as f64 / total_size as f64) * 85.0) as u8
            } else {
                50
            };

            Self::emit_progress(
                app,
                component,
                &format!(
                    "Downloading... ({:.1} MB / {:.1} MB)",
                    downloaded as f64 / 1_048_576.0,
                    total_size as f64 / 1_048_576.0
                ),
                percent,
                downloaded,
                total_size,
            );
        }

        Ok(())
    }

    fn extract_zip_safely(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
        let file = File::open(archive_path)
            .map_err(|e| format!("Failed to open zip archive {:?}: {}", archive_path, e))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry #{}: {}", i, e))?;

            // Zip-slip security protection: enclosed_name prevents directory traversal attacks
            let outpath = match file.enclosed_name() {
                Some(path) => dest_dir.join(path),
                None => continue,
            };

            if file.is_dir() {
                std::fs::create_dir_all(&outpath).map_err(|e| {
                    format!("Failed to create extracted directory {:?}: {}", outpath, e)
                })?;
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create extracted file {:?}: {}", outpath, e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract content to {:?}: {}", outpath, e))?;
            }
        }

        Ok(())
    }

    fn find_executable_in_dir(dir: &Path, exe_name: &str) -> Option<PathBuf> {
        let direct_match = dir.join(exe_name);
        if direct_match.exists() {
            return Some(direct_match);
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = Self::find_executable_in_dir(&path, exe_name) {
                        return Some(found);
                    }
                } else if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        if file_name.to_string_lossy().eq_ignore_ascii_case(exe_name) {
                            return Some(path);
                        }
                    }
                }
            }
        }
        None
    }

    fn emit_progress(
        app: Option<&AppHandle>,
        component: &str,
        status: &str,
        percent: u8,
        downloaded_bytes: u64,
        total_bytes: u64,
    ) {
        if let Some(handle) = app {
            let _ = handle.emit(
                "dependency-progress",
                DownloadProgress {
                    component: component.to_string(),
                    status: status.to_string(),
                    percent,
                    downloaded_bytes,
                    total_bytes,
                },
            );
        }
    }
}
