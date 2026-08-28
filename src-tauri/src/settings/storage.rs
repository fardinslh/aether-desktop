use crate::models::AppSettings;
use directories::ProjectDirs;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub struct SettingsStorage;

impl SettingsStorage {
    /// Returns the app data directory: %APPDATA%\aether-desktop on Windows
    pub fn get_app_data_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "aether", "aether-desktop") {
            proj_dirs.config_dir().to_path_buf()
        } else {
            PathBuf::from("C:\\ProgramData\\aether-desktop")
        }
    }

    /// Returns config file path: %APPDATA%\aether-desktop\config.json
    pub fn get_config_path() -> PathBuf {
        Self::get_app_data_dir().join("config.json")
    }

    /// Returns generated sing-box config path: %APPDATA%\aether-desktop\sing-box-config.json
    pub fn get_singbox_config_path() -> PathBuf {
        Self::get_app_data_dir().join("sing-box-config.json")
    }

    /// Returns logs directory: %APPDATA%\aether-desktop\logs
    pub fn get_logs_dir() -> PathBuf {
        Self::get_app_data_dir().join("logs")
    }

    /// Loads settings from disk, or creates default settings if not existing
    pub fn load() -> AppSettings {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }

        let default_settings = AppSettings::default();
        let _ = Self::save(&default_settings);
        default_settings
    }

    /// Saves settings to disk atomically (write to temp file, sync to disk, atomic rename)
    pub fn save(settings: &AppSettings) -> Result<(), String> {
        let dir = Self::get_app_data_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        let path = Self::get_config_path();
        let temp_path = path.with_extension("tmp.json");

        // 1. Write to temp file and flush
        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp settings file: {}", e))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write temp settings: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to flush temp settings to disk: {}", e))?;
        drop(file);

        // 2. Atomic rename/replace
        if let Err(e) = fs::rename(&temp_path, &path) {
            // Fallback for Windows file locking / cross-device rename
            if fs::copy(&temp_path, &path).is_err() {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Failed to atomically replace settings file: {}", e));
            }
            let _ = fs::remove_file(&temp_path);
        }

        Ok(())
    }

    /// Resets settings back to factory defaults
    pub fn reset() -> Result<AppSettings, String> {
        let defaults = AppSettings::default();
        Self::save(&defaults)?;
        Ok(defaults)
    }
}
