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

    pub fn get_aether_config_path_for_protocol(protocol: &crate::models::settings::AetherProtocol) -> PathBuf {
        match protocol {
            crate::models::settings::AetherProtocol::Masque => {
                Self::get_aether_data_dir().join("aether-masque.toml")
            }
            _ => Self::get_aether_data_dir().join("aether.toml"),
        }
    }

    pub fn get_aether_config_path() -> PathBuf {
        Self::get_aether_data_dir().join("aether.toml")
    }

    pub fn get_active_aether_lastconn_paths(settings: &AppSettings) -> Vec<PathBuf> {
        let config_path = Self::get_aether_config_path_for_protocol(&settings.aether.protocol);
        vec![lastconn_path(&config_path)]
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

                        // Migrate Discord factory preset from SecondaryProxy to Aether if matching old factory preset
                        for rule in &mut settings.application_rules {
                            if rule.source == crate::models::RuleSource::Preset
                                && rule.process_name.eq_ignore_ascii_case("discord.exe")
                                && rule.priority == crate::models::RulePriority::High
                                && rule.destination == crate::models::RouteDestination::SecondaryProxy
                            {
                                rule.destination = crate::models::RouteDestination::Aether;
                                needs_migration = true;
                            }
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

/// Derives a sibling path matching upstream Aether's `derive_sibling_path(config_path, suffix)`.
/// E.g. `aether.toml` + `"lastconn"` -> `aether-lastconn.toml`
/// E.g. `aether-masque.toml` + `"lastconn"` -> `aether-masque-lastconn.toml`
pub fn derive_sibling_path(config_path: &Path, suffix: &str) -> PathBuf {
    let parent = config_path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = config_path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_default();
    let extension = config_path
        .extension()
        .map(|s| s.to_string_lossy());

    let new_file_name = match extension {
        Some(ext) if !ext.is_empty() => format!("{}-{}.{}", file_stem, suffix, ext),
        _ => format!("{}-{}", file_stem, suffix),
    };

    parent.join(new_file_name)
}

/// Derives the native lastconn persistence path for a given Aether configuration file.
pub fn lastconn_path(config_path: &Path) -> PathBuf {
    derive_sibling_path(config_path, "lastconn")
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
/// Targets exact native lastconn path(s) derived from active Aether configuration.
/// Excludes all identity, private keys, and configuration files.
#[derive(Debug)]
pub struct AetherPersistenceSnapshot {
    pub snapshot_dir: PathBuf,
    pub entries: Vec<LastconnEntryState>,
}

impl AetherPersistenceSnapshot {
    /// Creates a transactional snapshot for explicit target lastconn paths.
    /// Does NOT scan or enumerate arbitrary files from the directory.
    pub fn create_for_targets(target_paths: &[PathBuf]) -> Result<Self, String> {
        if target_paths.is_empty() {
            return Err("Cannot create snapshot: no target lastconn paths provided".to_string());
        }

        let parent_dir = target_paths[0]
            .parent()
            .unwrap_or_else(|| Path::new("."));

        if !parent_dir.exists() {
            std::fs::create_dir_all(parent_dir)
                .map_err(|e| format!("Failed to create Aether data directory {:?}: {}", parent_dir, e))?;
        }

        let snapshot_dir = parent_dir.join(format!(".rollback_snapshot_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&snapshot_dir).map_err(|e| {
            format!(
                "Failed to create snapshot directory {:?}: {}",
                snapshot_dir, e
            )
        })?;

        let mut entries = Vec::new();

        for target_path in target_paths {
            let file_name = match target_path.file_name() {
                Some(name) => name,
                None => {
                    let _ = std::fs::remove_dir_all(&snapshot_dir);
                    return Err(format!("Invalid target path with no filename: {:?}", target_path));
                }
            };

            if target_path.exists() {
                let backup_path = snapshot_dir.join(file_name);
                if let Err(e) = std::fs::copy(target_path, &backup_path) {
                    let _ = std::fs::remove_dir_all(&snapshot_dir);
                    return Err(format!(
                        "Failed to snapshot lastconn persistence file {:?} to {:?}: {}",
                        target_path, backup_path, e
                    ));
                }
                entries.push(LastconnEntryState::Existed {
                    target_path: target_path.clone(),
                    backup_path,
                });
            } else {
                entries.push(LastconnEntryState::Absent {
                    target_path: target_path.clone(),
                });
            }
        }

        Ok(Self {
            snapshot_dir,
            entries,
        })
    }

    /// Convenience constructor using active settings
    pub fn create_for_settings(settings: &AppSettings) -> Result<Self, String> {
        let targets = SettingsStorage::get_active_aether_lastconn_paths(settings);
        Self::create_for_targets(&targets)
    }

    /// Backwards-compatible helper for directory-based tests
    pub fn create(aether_data_dir: &Path) -> Result<Self, String> {
        let target = lastconn_path(&aether_data_dir.join("aether.toml"));
        Self::create_for_targets(&[target])
    }

    pub fn restore(&self) -> Result<(), String> {
        for entry in &self.entries {
            match entry {
                LastconnEntryState::Existed {
                    target_path,
                    backup_path,
                } => {
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
                            "Atomic replacement failed during lastconn restore from {:?} to {:?}: {}",
                            temp_path, target_path, e
                        )
                    })?;
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
    fn test_a_aether_toml_is_not_included_in_snapshot() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_filter_a_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let toml_file = temp_dir.join("aether.toml");
        let lastconn_file = temp_dir.join("aether-lastconn.toml");

        std::fs::write(&toml_file, b"private_key = 'SECRET_IDENTITY'").unwrap();
        std::fs::write(&lastconn_file, b"endpoint = '162.159.192.1:2408'\nrtt = 45").unwrap();

        let mut settings = AppSettings::default();
        settings.aether.protocol = crate::models::settings::AetherProtocol::Wireguard;

        // Snapshot explicitly derived target paths for WireGuard config
        let lastconn_target = lastconn_path(&toml_file);
        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[lastconn_target]).unwrap();

        for entry in &snapshot.entries {
            match entry {
                LastconnEntryState::Existed { target_path, .. }
                | LastconnEntryState::Absent { target_path } => {
                    let name = target_path.file_name().unwrap().to_string_lossy();
                    assert_ne!(name, "aether.toml", "aether.toml (identity/config) must NOT be included in lastconn snapshot");
                }
            }
        }

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_b_aether_lastconn_toml_is_included_in_snapshot() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_filter_b_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let toml_file = temp_dir.join("aether.toml");
        let lastconn_file = temp_dir.join("aether-lastconn.toml");

        std::fs::write(&toml_file, b"private_key = 'SECRET_IDENTITY'").unwrap();
        std::fs::write(&lastconn_file, b"endpoint = '162.159.192.1:2408'\nrtt = 45").unwrap();

        let lastconn_target = lastconn_path(&toml_file);
        assert_eq!(lastconn_target.file_name().unwrap().to_string_lossy(), "aether-lastconn.toml");

        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[lastconn_target]).unwrap();
        assert_eq!(snapshot.entries.len(), 1);

        match &snapshot.entries[0] {
            LastconnEntryState::Existed { target_path, backup_path } => {
                assert_eq!(target_path.file_name().unwrap().to_string_lossy(), "aether-lastconn.toml");
                assert!(backup_path.exists());
            }
            _ => panic!("Expected aether-lastconn.toml to be marked as Existed"),
        }

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_c_aether_masque_toml_is_not_included_in_snapshot() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_filter_c_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let masque_config = temp_dir.join("aether-masque.toml");
        let masque_lastconn = temp_dir.join("aether-masque-lastconn.toml");

        std::fs::write(&masque_config, b"auth_token = 'MASQUE_SECRET'").unwrap();
        std::fs::write(&masque_lastconn, b"endpoint = '162.159.193.1:443'").unwrap();

        let target = lastconn_path(&masque_config);
        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

        for entry in &snapshot.entries {
            match entry {
                LastconnEntryState::Existed { target_path, .. }
                | LastconnEntryState::Absent { target_path } => {
                    let name = target_path.file_name().unwrap().to_string_lossy();
                    assert_ne!(name, "aether-masque.toml", "aether-masque.toml must NOT be included in lastconn snapshot");
                }
            }
        }

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_d_aether_masque_lastconn_toml_is_included_when_masque_is_active() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_filter_d_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let masque_config = temp_dir.join("aether-masque.toml");
        let masque_lastconn = temp_dir.join("aether-masque-lastconn.toml");

        std::fs::write(&masque_config, b"auth_token = 'MASQUE_SECRET'").unwrap();
        std::fs::write(&masque_lastconn, b"endpoint = '162.159.193.1:443'\nrtt = 55").unwrap();

        let target = lastconn_path(&masque_config);
        assert_eq!(target.file_name().unwrap().to_string_lossy(), "aether-masque-lastconn.toml");

        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();
        assert_eq!(snapshot.entries.len(), 1);

        match &snapshot.entries[0] {
            LastconnEntryState::Existed { target_path, backup_path } => {
                assert_eq!(target_path.file_name().unwrap().to_string_lossy(), "aether-masque-lastconn.toml");
                assert!(backup_path.exists());
            }
            _ => panic!("Expected aether-masque-lastconn.toml to be marked as Existed"),
        }

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_e_preexisting_aether_lastconn_toml_is_restored_byte_for_byte() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_e_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let toml_file = temp_dir.join("aether.toml");
        let lastconn_file = temp_dir.join("aether-lastconn.toml");
        let original_bytes = b"endpoint = '162.159.192.1:2408'\nrtt = 45\n";
        std::fs::write(&lastconn_file, original_bytes).unwrap();

        // 1. Snapshot native lastconn state
        let target = lastconn_path(&toml_file);
        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

        // 2. Simulate fresh scan modifying or writing new failed lastconn
        let modified_bytes = b"endpoint = '162.159.193.99:500'\nrtt = 999\n";
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
    fn test_f_previously_absent_aether_lastconn_toml_created_by_failed_scan_is_removed() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_f_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let toml_file = temp_dir.join("aether.toml");
        let lastconn_file = temp_dir.join("aether-lastconn.toml");
        assert!(!lastconn_file.exists());

        // 1. Snapshot with no existing lastconn
        let target = lastconn_path(&toml_file);
        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

        // 2. Fresh scan creates a new lastconn file
        std::fs::write(&lastconn_file, b"endpoint = '162.159.192.9:2408'\nrtt = 80\n").unwrap();
        assert!(lastconn_file.exists());

        // 3. Rollback: newly created lastconn file must be removed
        snapshot.restore().unwrap();
        assert!(!lastconn_file.exists(), "Newly created aether-lastconn.toml file must be removed on rollback");

        snapshot.cleanup();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_g_optimization_success_preserves_new_lastconn_and_discards_backup() {
        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_snap_g_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let toml_file = temp_dir.join("aether.toml");
        let lastconn_file = temp_dir.join("aether-lastconn.toml");
        let initial_bytes = b"endpoint = '162.159.192.1:2408'\nrtt = 95\n";
        std::fs::write(&lastconn_file, initial_bytes).unwrap();

        // 1. Snapshot initial state
        let target = lastconn_path(&toml_file);
        let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();
        let snapshot_dir = snapshot.snapshot_dir.clone();
        assert!(snapshot_dir.exists());

        // 2. Optimization succeeds and writes faster working candidate
        let optimized_bytes = b"endpoint = '162.159.192.5:2408'\nrtt = 38\n";
        std::fs::write(&lastconn_file, optimized_bytes).unwrap();

        // 3. Commit: cleanup snapshot
        snapshot.cleanup();
        assert!(!snapshot_dir.exists());

        // 4. Optimized bytes remain intact
        assert_eq!(std::fs::read(&lastconn_file).unwrap(), optimized_bytes);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
