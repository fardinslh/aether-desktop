use crate::dependencies::{DependencyManager, DependencyStatus};
use crate::health::HealthProber;
use crate::logging::{LogEntry, RingBufferLogger};
use crate::models::health::CloudflareTrace;
use crate::models::{AppSettings, ConnectionState, HealthStatus};
use crate::process::icon::extract_icon_base64;
use crate::process::{
    pick_windows_executable, ConnectionOrchestrator, ProcessDetector, RunningProcessInfo,
};
use crate::routing::SingBoxConfigGenerator;
use crate::settings::SettingsStorage;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tauri::{AppHandle, State};

pub struct AppState {
    pub logger: RingBufferLogger,
    pub connection_state: Arc<RwLock<ConnectionState>>,
    pub orchestrator: Arc<ConnectionOrchestrator>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryValidationResult {
    pub aether_exists: bool,
    pub aether_path: String,
    pub singbox_exists: bool,
    pub singbox_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableInspection {
    pub display_name: String,
    pub process_name: String,
    pub executable_path: String,
    pub icon_base64: Option<String>,
}

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    Ok(SettingsStorage::load())
}

/// Transactional save_settings:
/// - If Connected: live candidate settings must succeed on the network BEFORE saving to disk.
/// - If Disconnected: saves to disk atomically.
#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let is_connected = *state.connection_state.read() == ConnectionState::Connected;
    if is_connected {
        // 1. In connected mode, apply candidate settings live first. If this fails, error is returned
        // and disk storage is NOT mutated.
        state.orchestrator.apply_live_settings(&settings).await?;
        SettingsStorage::save(&settings)?;
        state.logger.log(
            "INFO",
            "Settings",
            "Settings validated, live applied, and persisted atomically",
        );
    } else {
        SettingsStorage::save(&settings)?;
        state
            .logger
            .log("INFO", "Settings", "Settings saved successfully to storage");
    }
    Ok(())
}

#[tauri::command]
pub fn reset_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let defaults = SettingsStorage::reset()?;
    state
        .logger
        .log("WARN", "Settings", "Reset settings to factory defaults");
    Ok(defaults)
}

#[tauri::command]
pub fn get_connection_state(state: State<'_, AppState>) -> ConnectionState {
    *state.connection_state.read()
}

#[tauri::command]
pub async fn connect_tunnel(state: State<'_, AppState>) -> Result<(), String> {
    let settings = SettingsStorage::load();
    state.orchestrator.connect(&settings).await
}

#[tauri::command]
pub async fn disconnect_tunnel(state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.disconnect().await
}

#[tauri::command]
pub async fn get_health_status(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let settings = SettingsStorage::load();
    Ok(state.orchestrator.check_health(&settings).await)
}

#[tauri::command]
pub fn get_running_applications() -> Vec<RunningProcessInfo> {
    ProcessDetector::get_running_gui_applications()
}

#[tauri::command]
pub fn inspect_executable_file(file_path: String) -> ExecutableInspection {
    let (display_name, process_name) = ProcessDetector::inspect_executable(&file_path);
    let icon_base64 = extract_icon_base64(&file_path);
    ExecutableInspection {
        display_name,
        process_name,
        executable_path: file_path,
        icon_base64,
    }
}

#[tauri::command]
pub fn pick_executable_file() -> Result<Option<String>, String> {
    Ok(pick_windows_executable())
}

#[tauri::command]
pub fn validate_aether_path(path: String) -> Result<String, String> {
    DependencyManager::validate_aether_binary(Path::new(&path))
}

#[tauri::command]
pub fn validate_singbox_path(path: String) -> Result<String, String> {
    DependencyManager::validate_singbox_binary(Path::new(&path))
}

#[tauri::command]
pub fn generate_singbox_config_preview() -> Result<String, String> {
    let settings = SettingsStorage::load();
    let config = SingBoxConfigGenerator::generate(&settings);
    SingBoxConfigGenerator::to_json_string(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_secondary_proxy(state: State<'_, AppState>) -> Result<CloudflareTrace, String> {
    let settings = SettingsStorage::load();
    state.logger.log(
        "INFO",
        "SecondaryProxy",
        format!(
            "Testing SOCKS5 proxy at {}:{}",
            settings.secondary_proxy.host, settings.secondary_proxy.port
        ),
    );
    HealthProber::query_cloudflare_trace_via_socks5(
        &settings.secondary_proxy.host,
        settings.secondary_proxy.port,
    )
    .await
}

#[tauri::command]
pub async fn test_aether_proxy(state: State<'_, AppState>) -> Result<CloudflareTrace, String> {
    let settings = SettingsStorage::load();
    state.logger.log(
        "INFO",
        "Aether",
        format!(
            "Testing Aether SOCKS5 proxy at {}:{}",
            settings.aether.host, settings.aether.port
        ),
    );
    HealthProber::query_cloudflare_trace_via_socks5(&settings.aether.host, settings.aether.port)
        .await
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logger.get_entries()
}

#[tauri::command]
pub fn export_logs(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.logger.export_as_string())
}

#[tauri::command]
pub fn validate_binaries() -> Result<BinaryValidationResult, String> {
    let settings = SettingsStorage::load();
    let aether_exists = !settings.aether.executable_path.is_empty()
        && Path::new(&settings.aether.executable_path).exists();
    let singbox_exists = !settings.sing_box.executable_path.is_empty()
        && Path::new(&settings.sing_box.executable_path).exists();

    Ok(BinaryValidationResult {
        aether_exists,
        aether_path: settings.aether.executable_path,
        singbox_exists,
        singbox_path: settings.sing_box.executable_path,
    })
}

#[tauri::command]
pub fn check_dependencies() -> DependencyStatus {
    DependencyManager::check_status()
}

#[tauri::command]
pub async fn install_aether_dependency(app: AppHandle) -> Result<String, String> {
    DependencyManager::install_aether(Some(&app)).await
}

#[tauri::command]
pub async fn install_singbox_dependency(app: AppHandle) -> Result<String, String> {
    DependencyManager::install_singbox(Some(&app)).await
}
