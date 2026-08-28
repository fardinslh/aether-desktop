use super::github::GithubClient;
use crate::settings::SettingsStorage;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const MAX_ARCHIVE_BYTES: u64 = 104_857_600; // 100 MB max allowed archive size
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    pub fn get_staging_dir() -> PathBuf {
        Self::get_base_dependencies_dir().join(".staging")
    }

    fn run_command_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Output, String> {
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;
        let start = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    return child.wait_with_output().map_err(|e| e.to_string());
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(format!("Command execution timed out after {:?}", timeout));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }

    /// Validates an aether.exe executable by checking name, running --version with timeout and parsing the output
    pub fn validate_aether_binary(exe_path: &Path) -> Result<String, String> {
        if !exe_path.exists() {
            return Err(format!("Aether binary not found at {:?}", exe_path));
        }

        // Verify filename
        let name_lower = exe_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if !name_lower.starts_with("aether") || !name_lower.ends_with(".exe") {
            return Err("Executable must be named aether.exe".to_string());
        }

        let mut cmd = Command::new(exe_path);
        cmd.arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = Self::run_command_with_timeout(cmd, Duration::from_secs(4))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "aether --version exited with error: {}",
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            return Err("aether --version produced empty output".to_string());
        }

        Ok(first_line.to_string())
    }

    /// Validates a sing-box.exe executable by checking name, version and running a test config check
    pub fn validate_singbox_binary(exe_path: &Path) -> Result<String, String> {
        if !exe_path.exists() {
            return Err(format!("sing-box binary not found at {:?}", exe_path));
        }

        // Verify filename
        let name_lower = exe_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if !name_lower.starts_with("sing-box") || !name_lower.ends_with(".exe") {
            return Err("Executable must be named sing-box.exe".to_string());
        }

        // 1. Check version output with timeout
        let mut cmd = Command::new(exe_path);
        cmd.arg("version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = Self::run_command_with_timeout(cmd, Duration::from_secs(4))?;

        if !output.status.success() {
            return Err("sing-box version command failed".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").trim();
        if !first_line.to_lowercase().contains("sing-box") {
            return Err("Executable output does not match expected sing-box identity".to_string());
        }

        // 2. Perform minimal configuration check test
        let temp_test_config =
            std::env::temp_dir().join(format!("singbox-check-test-{}.json", Uuid::new_v4()));
        let test_config_json = r#"{
  "log": { "level": "panic" },
  "inbounds": [],
  "outbounds": [{ "type": "direct", "tag": "direct" }],
  "route": { "rules": [], "final": "direct" }
}"#;
        std::fs::write(&temp_test_config, test_config_json)
            .map_err(|e| format!("Failed to write temporary test config: {}", e))?;

        let mut check_cmd = Command::new(exe_path);
        check_cmd
            .arg("check")
            .arg("-c")
            .arg(&temp_test_config)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            check_cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let check_output = Self::run_command_with_timeout(check_cmd, Duration::from_secs(4));
        let _ = std::fs::remove_file(&temp_test_config);

        let check_res = check_output
            .map_err(|e| format!("Failed to run sing-box check on test config: {}", e))?;

        if !check_res.status.success() {
            let err = String::from_utf8_lossy(&check_res.stderr);
            return Err(format!(
                "sing-box check failed validation test: {}",
                err.trim()
            ));
        }

        Ok(first_line.to_string())
    }

    pub fn check_status() -> DependencyStatus {
        let settings = SettingsStorage::load();

        let aether_path = settings.aether.executable_path.clone();
        let (aether_installed, aether_version) = if !aether_path.is_empty() {
            match Self::validate_aether_binary(Path::new(&aether_path)) {
                Ok(ver) => (true, Some(ver)),
                Err(_) => (false, None),
            }
        } else {
            (false, None)
        };

        let singbox_path = settings.sing_box.executable_path.clone();
        let (singbox_installed, singbox_version) = if !singbox_path.is_empty() {
            match Self::validate_singbox_binary(Path::new(&singbox_path)) {
                Ok(ver) => (true, Some(ver)),
                Err(_) => (false, None),
            }
        } else {
            (false, None)
        };

        DependencyStatus {
            aether_installed,
            aether_path,
            aether_version,
            singbox_installed,
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
        let asset = GithubClient::find_aether_asset(&release)?;

        if asset.size > MAX_ARCHIVE_BYTES {
            return Err(format!(
                "Asset size ({} MB) exceeds maximum safety limit of 100 MB",
                asset.size / 1_048_576
            ));
        }

        let install_id = Uuid::new_v4().to_string();
        let staging_dir = Self::get_staging_dir().join(&install_id);
        std::fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Failed to create staging dir {:?}: {}", staging_dir, e))?;

        let temp_archive_path = staging_dir.join("aether-download.tmp.zip");

        let sha256_result = Self::download_file_with_progress(
            app,
            "aether",
            &asset.browser_download_url,
            &temp_archive_path,
            asset.size,
        )
        .await;

        if let Err(e) = sha256_result {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
        let calculated_hash = sha256_result.unwrap();

        if let Some(chk_asset) = GithubClient::find_companion_checksum_asset(&release, &asset.name)
        {
            if let Ok(chk_text) =
                GithubClient::fetch_checksum_text(&chk_asset.browser_download_url).await
            {
                if let Some(expected_hash) =
                    Self::extract_hash_from_checksum_file(&chk_text, &asset.name)
                {
                    if !calculated_hash.eq_ignore_ascii_case(&expected_hash) {
                        let _ = std::fs::remove_dir_all(&staging_dir);
                        return Err(format!(
                            "SHA-256 checksum mismatch! Expected: {}, Calculated: {}",
                            expected_hash, calculated_hash
                        ));
                    }
                }
            }
        }

        Self::emit_progress(
            app,
            "aether",
            "Extracting files in isolated staging...",
            90,
            asset.size,
            asset.size,
        );
        let extracted_dir = staging_dir.join("extracted");
        std::fs::create_dir_all(&extracted_dir)
            .map_err(|e| format!("Failed to create extracted dir: {}", e))?;

        if let Err(e) = Self::extract_zip_safely(&temp_archive_path, &extracted_dir) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
        let _ = std::fs::remove_file(&temp_archive_path);

        Self::emit_progress(
            app,
            "aether",
            "Validating executable execution...",
            95,
            asset.size,
            asset.size,
        );
        let found_exe = match Self::find_executable_in_dir(&extracted_dir, "aether.exe") {
            Some(p) => p,
            None => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!(
                    "aether.exe not found after extracting {:?}",
                    extracted_dir
                ));
            }
        };

        let version_str = match Self::validate_aether_binary(&found_exe) {
            Ok(v) => v,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!("Aether executable failed validation test: {}", e));
            }
        };

        let final_dir = Self::get_base_dependencies_dir()
            .join("aether")
            .join(&release.tag_name);
        if final_dir.exists() {
            let _ = std::fs::remove_dir_all(&final_dir);
        }
        if let Some(parent) = final_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(_e) = std::fs::rename(&extracted_dir, &final_dir) {
            if let Err(copy_err) = Self::copy_dir_recursive(&extracted_dir, &final_dir) {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!(
                    "Failed to promote Aether installation: {}",
                    copy_err
                ));
            }
        }
        let _ = std::fs::remove_dir_all(&staging_dir);

        let final_exe = final_dir.join("aether.exe");
        let exe_str = if final_exe.exists() {
            final_exe.to_string_lossy().to_string()
        } else {
            Self::find_executable_in_dir(&final_dir, "aether.exe")
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| final_exe.to_string_lossy().to_string())
        };

        let mut settings = SettingsStorage::load();
        settings.aether.executable_path = exe_str.clone();
        SettingsStorage::save(&settings)?;

        Self::emit_progress(
            app,
            "aether",
            &format!("Installation verified ({})", version_str),
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
        let asset = GithubClient::find_singbox_asset(&release)?;

        if asset.size > MAX_ARCHIVE_BYTES {
            return Err(format!(
                "Asset size ({} MB) exceeds maximum safety limit of 100 MB",
                asset.size / 1_048_576
            ));
        }

        let install_id = Uuid::new_v4().to_string();
        let staging_dir = Self::get_staging_dir().join(&install_id);
        std::fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Failed to create staging dir {:?}: {}", staging_dir, e))?;

        let temp_archive_path = staging_dir.join("sing-box-download.tmp.zip");

        let sha256_result = Self::download_file_with_progress(
            app,
            "sing-box",
            &asset.browser_download_url,
            &temp_archive_path,
            asset.size,
        )
        .await;

        if let Err(e) = sha256_result {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
        let calculated_hash = sha256_result.unwrap();

        if let Some(chk_asset) = GithubClient::find_companion_checksum_asset(&release, &asset.name)
        {
            if let Ok(chk_text) =
                GithubClient::fetch_checksum_text(&chk_asset.browser_download_url).await
            {
                if let Some(expected_hash) =
                    Self::extract_hash_from_checksum_file(&chk_text, &asset.name)
                {
                    if !calculated_hash.eq_ignore_ascii_case(&expected_hash) {
                        let _ = std::fs::remove_dir_all(&staging_dir);
                        return Err(format!(
                            "SHA-256 checksum mismatch! Expected: {}, Calculated: {}",
                            expected_hash, calculated_hash
                        ));
                    }
                }
            }
        }

        Self::emit_progress(
            app,
            "sing-box",
            "Extracting files in isolated staging...",
            90,
            asset.size,
            asset.size,
        );
        let extracted_dir = staging_dir.join("extracted");
        std::fs::create_dir_all(&extracted_dir)
            .map_err(|e| format!("Failed to create extracted dir: {}", e))?;

        if let Err(e) = Self::extract_zip_safely(&temp_archive_path, &extracted_dir) {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(e);
        }
        let _ = std::fs::remove_file(&temp_archive_path);

        Self::emit_progress(
            app,
            "sing-box",
            "Validating sing-box execution and config check...",
            95,
            asset.size,
            asset.size,
        );
        let found_exe = match Self::find_executable_in_dir(&extracted_dir, "sing-box.exe") {
            Some(p) => p,
            None => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!(
                    "sing-box.exe not found after extracting {:?}",
                    extracted_dir
                ));
            }
        };

        let version_str = match Self::validate_singbox_binary(&found_exe) {
            Ok(v) => v,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!(
                    "sing-box executable failed validation tests: {}",
                    e
                ));
            }
        };

        let final_dir = Self::get_base_dependencies_dir()
            .join("sing-box")
            .join(&release.tag_name);
        if final_dir.exists() {
            let _ = std::fs::remove_dir_all(&final_dir);
        }
        if let Some(parent) = final_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if let Err(_e) = std::fs::rename(&extracted_dir, &final_dir) {
            if let Err(copy_err) = Self::copy_dir_recursive(&extracted_dir, &final_dir) {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(format!(
                    "Failed to promote sing-box installation: {}",
                    copy_err
                ));
            }
        }
        let _ = std::fs::remove_dir_all(&staging_dir);

        let final_exe = final_dir.join("sing-box.exe");
        let exe_str = if final_exe.exists() {
            final_exe.to_string_lossy().to_string()
        } else {
            Self::find_executable_in_dir(&final_dir, "sing-box.exe")
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| final_exe.to_string_lossy().to_string())
        };

        let mut settings = SettingsStorage::load();
        settings.sing_box.executable_path = exe_str.clone();
        SettingsStorage::save(&settings)?;

        Self::emit_progress(
            app,
            "sing-box",
            &format!("Installation verified ({})", version_str),
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
    ) -> Result<String, String> {
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
        if total_size > MAX_ARCHIVE_BYTES {
            return Err(format!(
                "Total size {} exceeds maximum 100 MB limit",
                total_size
            ));
        }

        let mut file = File::create(dest_path).map_err(|e| {
            format!(
                "Failed to create temporary download file {:?}: {}",
                dest_path, e
            )
        })?;

        let mut hasher = Sha256::new();
        let mut stream = resp.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Error downloading chunk: {}", e))?;
            downloaded += chunk.len() as u64;

            if downloaded > MAX_ARCHIVE_BYTES {
                let _ = std::fs::remove_file(dest_path);
                return Err("Download aborted: exceeded maximum safety size of 100 MB".to_string());
            }

            hasher.update(&chunk);
            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write chunk to file: {}", e))?;

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

        file.sync_all()
            .map_err(|e| format!("Failed to flush downloaded file: {}", e))?;
        drop(file);

        if expected_size > 0 && downloaded != expected_size {
            let _ = std::fs::remove_file(dest_path);
            return Err(format!(
                "Download truncated! Expected {} bytes, received {} bytes",
                expected_size, downloaded
            ));
        }

        let hash_bytes = hasher.finalize();
        Ok(format!("{:x}", hash_bytes))
    }

    fn extract_hash_from_checksum_file(
        checksum_content: &str,
        target_filename: &str,
    ) -> Option<String> {
        for line in checksum_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let hash = parts[0].trim();
                let file = parts[1].trim().trim_start_matches('*');
                if file.eq_ignore_ascii_case(target_filename) {
                    return Some(hash.to_string());
                }
            } else if parts.len() == 1 && parts[0].len() == 64 {
                return Some(parts[0].to_string());
            }
        }
        None
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

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create destination dir {:?}: {}", dst, e))?;

        for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path).map_err(|e| {
                    format!(
                        "Failed to copy file {:?} to {:?}: {}",
                        src_path, dst_path, e
                    )
                })?;
            }
        }
        Ok(())
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
