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

const KNOWN_LASTCONN_FILENAMES: &[&str] = &[
    "lastconn",
    "lastconn.json",
    "last_connection.json",
    "last_connection",
    "last_endpoint.json",
    "last_endpoint",
    "lastconn.bin",
    "lastconn.dat",
    "endpoints.json",
];

pub fn is_lastconn_persistence_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Strictly exclude identity, private keys, certificates, and configuration files
    if lower == "aether.toml"
        || lower.ends_with(".toml")
        || lower.contains("identity")
        || lower.contains("key")
        || lower.contains("cert")
        || lower.contains("config")
    {
        return false;
    }

    if KNOWN_LASTCONN_FILENAMES.iter().any(|k| lower == *k) {
        return true;
    }

    lower.contains("lastconn") || lower.contains("last_conn") || lower.contains("last_endpoint")
}

#[derive(Debug, Clone, PartialEq)]
pub enum LastconnEntryState {
    Existed {
        target_path: PathBuf,
        backup_path: PathBuf,
    },
    Absent {
        target_path: PathBuf,
    },
}

/// Transactional snapshot manager for native upstream Aether last-connection persistence state ONLY.
/// Preserves native byte-for-byte endpoint state before optimization and restores it atomically upon failure.
/// Excludes all identity, private keys, and configuration files.
#[derive(Debug)]
pub struct AetherPersistenceSnapshot {
    pub snapshot_dir: PathBuf,
    pub target_dir: PathBuf,
    pub entries: Vec<LastconnEntryState>,
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

        let mut entries = Vec::new();
        let mut tracked_names = std::collections::HashSet::new();

        // 1. Check existing files in aether_data_dir
        if let Ok(dir_entries) = std::fs::read_dir(aether_data_dir) {
            for entry in dir_entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    if is_lastconn_persistence_file(&file_name_str) {
                        let backup_path = snapshot_dir.join(&file_name);
                        std::fs::copy(&path, &backup_path).map_err(|e| {
                            format!(
                                "Failed to snapshot lastconn persistence file {:?} to {:?}: {}",
                                path, backup_path, e
                            )
                        })?;
                        entries.push(LastconnEntryState::Existed {
                            target_path: path,
                            backup_path,
                        });
                        tracked_names.insert(file_name_str.to_string());
                    }
                }
            }
        }

        // 2. Track known lastconn candidate names that were ABSENT before optimization
        for &known_name in KNOWN_LASTCONN_FILENAMES {
            if !tracked_names.contains(known_name) {
                let target_path = aether_data_dir.join(known_name);
                if !target_path.exists() {
                    entries.push(LastconnEntryState::Absent { target_path });
                    tracked_names.insert(known_name.to_string());
                }
            }
        }

        Ok(Self {
            snapshot_dir,
            target_dir: aether_data_dir.to_path_buf(),
            entries,
        })
    }

    pub fn restore(&self) -> Result<(), String> {
        for entry in &self.entries {
            match entry {
                LastconnEntryState::Existed {
                    target_path,
                    backup_path,
                } => {
                    if backup_path.exists() {
                        let bytes = std::fs::read(backup_path).map_err(|e| {
                            format!(
                                "Failed to read backup persistence file {:?}: {}",
                                backup_path, e
                            )
                        })?;

                        let parent = target_path
                            .parent()
                            .unwrap_or_else(|| Path::new("."));
                        let temp_path = parent.join(format!(
                            "{}.tmp.{}",
                            target_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy(),
                            Uuid::new_v4()
                        ));

                        let mut file = File::create(&temp_path).map_err(|e| {
                            format!("Failed to create temp file for atomic restore: {}", e)
                        })?;
                        file.write_all(&bytes).map_err(|e| {
                            format!("Failed to write bytes to temp file: {}", e)
                        })?;
                        file.sync_all().map_err(|e| {
                            format!("Failed to sync temp file: {}", e)
                        })?;
                        drop(file);

                        atomic_replace_file(&temp_path, target_path).map_err(|e| {
                            let _ = std::fs::remove_file(&temp_path);
                            format!(
                                "Atomic replacement failed during lastconn restore: {}",
                                e
                            )
                        })?;
                    }
                }
                LastconnEntryState::Absent { target_path } => {
                    // If a new lastconn file was created during failed scan, remove it on rollback
                    if target_path.exists() {
                        std::fs::remove_file(target_path).map_err(|e| {
                            format!(
                                "Failed to remove newly-created lastconn file {:?} during rollback: {}",
                                target_path, e
                            )
                        })?;
                    }
                }
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
    fn test_a_identity_and_config_files_never_included_in_snapshot() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_filter_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 1. Create identity, config, and lastconn files in data directory
        let toml_file = temp_dir.join("aether.toml");
        let identity_file = temp_dir.join("identity.json");
        let key_file = temp_dir.join("client.key");
        let lastconn_file = temp_dir.join("lastconn.json");

        std::fs::write(&toml_file, b"scan_mode = 'Thorough'").unwrap();
        std::fs::write(&identity_file, b"{\"private_key\":\"SECRET\"}").unwrap();
        std::fs::write(&key_file, b"PRIVATE_KEY_BYTES").unwrap();
        std::fs::write(&lastconn_file, b"{\"endpoint\":\"162.159.192.1:2408\",\"rtt\":45}").unwrap();

        // 2. Snapshot
        let snapshot = AetherPersistenceSnapshot::create(&temp_dir).unwrap();

        // 3. Verify snapshot entries strictly exclude config and identity files
        for entry in &snapshot.entries {
            match entry {
                LastconnEntryState::Existed { target_path, .. } => {
                    let name = target_path.file_name().unwrap().to_string_lossy();
                    assert_ne!(name, "aether.toml");
                    assert_ne!(name, "identity.json");
                    assert_ne!(name, "client.key");
                    assert_eq!(name, "lastconn.json");
                }
                LastconnEntryState::Absent { target_path } => {
                    let name = target_path.file_name().unwrap().to_string_lossy();
                    assert_ne!(name, "aether.toml");
                    assert_ne!(name, "identity.json");
                    assert_ne!(name, "client.key");
                }
            }
        }

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_b_previous_lastconn_existed_restores_original_bytes_atomically() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_b_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let lastconn_file = temp_dir.join("lastconn.json");
        let original_bytes = b"{\"endpoint\":\"162.159.192.1:2408\",\"rtt\":45}";
        std::fs::write(&lastconn_file, original_bytes).unwrap();

        // 1. Snapshot native lastconn state
        let snapshot = AetherPersistenceSnapshot::create(&temp_dir).unwrap();

        // 2. Simulate fresh scan modifying or writing new failed lastconn
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
    fn test_c_previous_lastconn_absent_removes_newly_created_file_on_rollback() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_c_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let lastconn_file = temp_dir.join("lastconn.json");
        assert!(!lastconn_file.exists());

        // 1. Snapshot with no existing lastconn
        let snapshot = AetherPersistenceSnapshot::create(&temp_dir).unwrap();

        // 2. Fresh scan creates a new lastconn file
        std::fs::write(&lastconn_file, b"{\"endpoint\":\"162.159.192.9:2408\"}").unwrap();
        assert!(lastconn_file.exists());

        // 3. Rollback: newly created lastconn file must be removed
        snapshot.restore().unwrap();
        assert!(!lastconn_file.exists(), "Newly created lastconn file must be removed on rollback");

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_d_optimization_succeeds_new_lastconn_remains_and_old_backup_discarded() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_d_{}", Uuid::new_v4()));
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
