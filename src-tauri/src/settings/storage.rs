use crate::models::AppSettings;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[cfg(windows)]
pub fn atomic_replace_file(temp_path: &Path, destination_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, ReplaceFileW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temp_w: Vec<u16> = temp_path.as_os_str().encode_wide().chain(Some(0)).collect();
    let dest_w: Vec<u16> = destination_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    if destination_path.exists() {
        let success = unsafe {
            ReplaceFileW(
                dest_w.as_ptr(),
                temp_w.as_ptr(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if success != 0 {
            return Ok(());
        }
    }

    let success = unsafe {
        MoveFileExW(
            temp_w.as_ptr(),
            dest_w.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if success != 0 {
        Ok(())
    } else {
        let err = std::io::Error::last_os_error();
        Err(format!(
            "Atomic file replacement failed from {:?} to {:?}: {}",
            temp_path, destination_path, err
        ))
    }
}

#[cfg(not(windows))]
pub fn atomic_replace_file(temp_path: &Path, destination_path: &Path) -> Result<(), String> {
    std::fs::rename(temp_path, destination_path)
        .map_err(|e| format!("Atomic file replacement failed: {}", e))
}

pub struct SettingsStorage;

impl SettingsStorage {
    pub fn get_config_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|b| b.config_dir().join("AetherDesktop"))
            .unwrap_or_else(|| PathBuf::from("./config"))
    }

    pub fn get_aether_data_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map(|b| b.data_local_dir().join("AetherDesktop").join("aether"))
            .unwrap_or_else(|| PathBuf::from("./aether_data"))
    }

    pub fn get_aether_config_path() -> PathBuf {
        Self::get_aether_data_dir().join("aether.toml")
    }

    pub fn get_config_file_path() -> PathBuf {
        Self::get_config_dir().join("config.json")
    }

    pub fn get_singbox_config_path() -> PathBuf {
        Self::get_config_dir().join("sing-box-config.json")
    }

    pub fn load() -> AppSettings {
        let path = Self::get_config_file_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<AppSettings>(&content) {
                    Ok(mut settings) => {
                        let mut needs_migration = false;

                        // Migrate legacy launchArguments or empty profile
                        if !settings.aether.launch_arguments.is_empty() {
                            settings.aether.launch_arguments.clear();
                            needs_migration = true;
                        }

                        // Ensure proper non-interactive defaults
                        if settings.aether.host.is_empty() {
                            settings.aether.host = "127.0.0.1".to_string();
                            needs_migration = true;
                        }
                        if settings.aether.port == 0 {
                            settings.aether.port = 1819;
                            needs_migration = true;
                        }

                        if needs_migration {
                            let _ = Self::save(&settings);
                        }

                        settings
                    }
                    Err(e) => {
                        eprintln!("Failed to parse settings JSON: {}. Using defaults.", e);
                        AppSettings::default()
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read settings file: {}. Using defaults.", e);
                    AppSettings::default()
                }
            }
        } else {
            let defaults = AppSettings::default();
            let _ = Self::save(&defaults);
            defaults
        }
    }

    /// Atomically persists settings to disk using Windows ReplaceFileW/MoveFileExW
    pub fn save(settings: &AppSettings) -> Result<(), String> {
        let dir = Self::get_config_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let final_path = Self::get_config_file_path();
        let temp_path = dir.join(format!("config.tmp.{}.json", Uuid::new_v4()));

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp settings file: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write settings to temp file: {}", e))?;

        file.sync_all()
            .map_err(|e| format!("Failed to flush temp settings file: {}", e))?;

        drop(file);

        if let Err(e) = atomic_replace_file(&temp_path, &final_path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(e);
        }

        Ok(())
    }

    pub fn reset() -> Result<AppSettings, String> {
        let defaults = AppSettings::default();
        Self::save(&defaults)?;
        Ok(defaults)
    }
}
