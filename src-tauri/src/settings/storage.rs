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

/// Transactional snapshot manager for native upstream Aether persistence state (lastconn, credentials, config).
/// Preserves native byte-for-byte endpoint state before optimization and restores it atomically upon failure.
#[derive(Debug)]
pub struct AetherPersistenceSnapshot {
    pub snapshot_dir: PathBuf,
    pub target_dir: PathBuf,
    pub snapshotted_files: Vec<(PathBuf, PathBuf)>, // (original_path, backup_path)
}

impl AetherPersistenceSnapshot {
    pub fn create(aether_data_dir: &Path) -> Result<Self, String> {
        if !aether_data_dir.exists() {
            std::fs::create_dir_all(aether_data_dir)
                .map_err(|e| format!("Failed to create Aether data directory: {}", e))?;
        }

        let snapshot_dir =
            aether_data_dir.join(format!(".rollback_snapshot_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&snapshot_dir).map_err(|e| {
            format!(
                "Failed to create snapshot directory {:?}: {}",
                snapshot_dir, e
            )
        })?;

        let mut snapshotted_files = Vec::new();
        let entries = std::fs::read_dir(aether_data_dir).map_err(|e| {
            format!(
                "Failed to read Aether data directory {:?}: {}",
                aether_data_dir, e
            )
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();
                // Exclude any temporary rollback snapshots themselves
                if !file_name_str.starts_with(".rollback") {
                    let backup_path = snapshot_dir.join(&file_name);
                    std::fs::copy(&path, &backup_path).map_err(|e| {
                        format!(
                            "Failed to snapshot persistence file {:?} to {:?}: {}",
                            path, backup_path, e
                        )
                    })?;
                    snapshotted_files.push((path, backup_path));
                }
            }
        }

        Ok(Self {
            snapshot_dir,
            target_dir: aether_data_dir.to_path_buf(),
            snapshotted_files,
        })
    }

    pub fn restore(&self) -> Result<(), String> {
        for (original, backup) in &self.snapshotted_files {
            if backup.exists() {
                // Byte-for-byte copy back to original location
                std::fs::copy(backup, original).map_err(|e| {
                    format!(
                        "Failed to restore persistence file from {:?} to {:?}: {}",
                        backup, original, e
                    )
                })?;
            }
        }
        Ok(())
    }

    pub fn cleanup(self) {
        let _ = std::fs::remove_dir_all(&self.snapshot_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_previous_lastconn_exists_fresh_scan_fails_restores_original_bytes() {
        let temp_dir = std::env::temp_dir().join(format!("aether_test_snap_a_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let lastconn_file = temp_dir.join("lastconn.json");
        let original_bytes = b"{\"endpoint\":\"162.159.192.1:2408\",\"rtt\":45}";
        std::fs::write(&lastconn_file, original_bytes).unwrap();

        // 1. Snapshot native lastconn state
        let snapshot = AetherPersistenceSnapshot::create(&temp_dir).unwrap();
        assert_eq!(snapshot.snapshotted_files.len(), 1);

        // 2. Simulate fresh scan modifying or writing new failed/intermediate lastconn
        let modified_bytes = b"{\"endpoint\":\"162.159.193.99:500\",\"rtt\":999}";
        std::fs::write(&lastconn_file, modified_bytes).unwrap();
        assert_eq!(std::fs::read(&lastconn_file).unwrap(), modified_bytes);

        // 3. Rollback: restore snapshot
        snapshot.restore().unwrap();
        assert_eq!(std::fs::read(&lastconn_file).unwrap(), original_bytes);

        // 4. Cleanup
        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_b_optimization_succeeds_new_lastconn_remains_and_old_backup_discarded() {
        let temp_dir = std::env::temp_dir().join(format!("aether_test_snap_b_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let lastconn_file = temp_dir.join("lastconn.json");
        let initial_bytes = b"{\"endpoint\":\"162.159.192.1:2408\",\"rtt\":95}";
        std::fs::write(&lastconn_file, initial_bytes).unwrap();

        // 1. Snapshot initial state
        let snapshot = AetherPersistenceSnapshot::create(&temp_dir).unwrap();
        let snapshot_dir = snapshot.snapshot_dir.clone();
        assert!(snapshot_dir.exists());

        // 2. Optimization succeeds and writes faster working candidate
        let optimized_bytes = b"{\"endpoint\":\"162.159.192.5:2408\",\"rtt\":38}";
        std::fs::write(&lastconn_file, optimized_bytes).unwrap();

        // 3. Commit: cleanup snapshot
        snapshot.cleanup();
        assert!(!snapshot_dir.exists());

        // 4. Optimized bytes remain intact
        assert_eq!(std::fs::read(&lastconn_file).unwrap(), optimized_bytes);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
