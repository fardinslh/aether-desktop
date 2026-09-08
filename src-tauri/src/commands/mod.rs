use crate::dependencies::{DependencyManager, DependencyStatus};
use crate::health::HealthProber;
use crate::logging::{LogEntry, RingBufferLogger};
use crate::models::health::CloudflareTrace;
use crate::models::settings::{
    SecondaryProxyMode, SecondaryProxyProfile, SecondaryProxySettings,
};
use crate::models::{AppSettings, ConnectionState, HealthStatus};
use crate::process::icon::extract_icon_base64;
use crate::process::orchestrator::RouteOptimizationResult;
use crate::process::{
    pick_windows_executable, ConnectionOrchestrator, ProcessDetector, RunningProcessInfo,
};
use crate::process::job::assign_child_to_global_job;
use crate::routing::secondary::parse_subscription_payload;
use crate::routing::SingBoxConfigGenerator;
use crate::settings::SettingsStorage;
use futures_util::{stream, StreamExt};
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondarySubscriptionUpdate {
    pub profiles: Vec<SecondaryProxyProfile>,
    pub skipped: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryProfilePing {
    pub index: usize,
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub ip: Option<String>,
    pub colo: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub fn get_settings() -> Result<AppSettings, String> {
    Ok(SettingsStorage::load())
}

/// Fully Transactional save_settings:
/// - In Connected Mode: applies candidate settings live on runtime.
///   If disk persistence subsequently fails, rolls runtime back to previous configuration.
///   Surfaces clear errors indicating whether rollback succeeded or failed critically.
/// - In Disconnected Mode: saves to disk atomically.
#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> Result<(), String> {
    SingBoxConfigGenerator::try_generate(&settings)?;
    let old_settings = SettingsStorage::load();
    let is_connected = *state.connection_state.read() == ConnectionState::Connected;

    if is_connected {
        // 1. Live apply candidate settings to running router first
        state.orchestrator.apply_live_settings(&settings).await?;

        // 2. Apply OS integration and persist candidate settings atomically
        let persist_result =
            SettingsStorage::configure_start_with_windows(settings.general.start_with_windows)
                .and_then(|_| SettingsStorage::save(&settings));
        if let Err(save_err) = persist_result {
            let _ = SettingsStorage::configure_start_with_windows(
                old_settings.general.start_with_windows,
            );
            state.logger.log(
                "ERROR",
                "Settings",
                format!(
                    "Settings persistence failed: {}. Initiating runtime rollback...",
                    save_err
                ),
            );
            // Attempt rollback to old_settings
            match state.orchestrator.apply_live_settings(&old_settings).await {
                Ok(_) => {
                    let msg = format!("Settings persistence failed ({}). Runtime restored successfully to previous configuration.", save_err);
                    state.logger.log("ERROR", "Settings", &msg);
                    return Err(msg);
                }
                Err(rb_err) => {
                    let msg = format!("CRITICAL: Settings persistence failed ({}) AND runtime rollback also failed ({}).", save_err, rb_err);
                    state.logger.log("ERROR", "Settings", &msg);
                    return Err(msg);
                }
            }
        }

        state.logger.log(
            "INFO",
            "Settings",
            "Settings validated, live applied, and persisted atomically",
        );
    } else {
        SettingsStorage::configure_start_with_windows(settings.general.start_with_windows)?;
        if let Err(err) = SettingsStorage::save(&settings) {
            let _ = SettingsStorage::configure_start_with_windows(
                old_settings.general.start_with_windows,
            );
            return Err(err);
        }
        state
            .logger
            .log("INFO", "Settings", "Settings saved successfully to storage");
    }
    Ok(())
}

#[tauri::command]
pub async fn reset_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let old_settings = SettingsStorage::load();
    let defaults = AppSettings::default();
    let connected = *state.connection_state.read() == ConnectionState::Connected;
    if connected {
        state.orchestrator.apply_live_settings(&defaults).await?;
    }
    if let Err(err) =
        SettingsStorage::configure_start_with_windows(defaults.general.start_with_windows)
            .and_then(|_| SettingsStorage::save(&defaults))
    {
        let _ =
            SettingsStorage::configure_start_with_windows(old_settings.general.start_with_windows);
        if connected {
            let _ = state.orchestrator.apply_live_settings(&old_settings).await;
        }
        return Err(err);
    }
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
pub async fn find_faster_gateway(
    state: State<'_, AppState>,
) -> Result<RouteOptimizationResult, String> {
    let settings = SettingsStorage::load();
    state.orchestrator.find_faster_gateway(&settings).await
}

#[tauri::command]
pub async fn disconnect_tunnel(state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.disconnect().await
}

#[tauri::command]
pub async fn cancel_connection(state: State<'_, AppState>) -> Result<(), String> {
    state.orchestrator.cancel_connection().await
}

#[tauri::command]
pub async fn get_health_status(state: State<'_, AppState>) -> Result<HealthStatus, String> {
    let settings = SettingsStorage::load();
    let health = state.orchestrator.check_health(&settings).await;
    if settings.general.reconnect_automatically
        && *state.connection_state.read() == ConnectionState::Error
    {
        let orchestrator = state.orchestrator.clone();
        tauri::async_runtime::spawn(async move {
            orchestrator.recover_connection(&settings).await;
        });
    }
    Ok(health)
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
    let config = SingBoxConfigGenerator::try_generate(&settings)?;
    SingBoxConfigGenerator::to_json_string(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_secondary_proxy(
    settings: SecondaryProxySettings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<CloudflareTrace, String> {
    state.logger.log(
        "INFO",
        "SecondaryProxy",
        match settings.mode {
            SecondaryProxyMode::ExternalSocks => {
        format!(
                    "Testing external SOCKS5 proxy at {}:{}",
                    settings.host, settings.port
                )
            }
            SecondaryProxyMode::Embedded => "Testing built-in secondary proxy config".to_string(),
        },
    );

    match settings.mode {
        SecondaryProxyMode::ExternalSocks => {
            HealthProber::query_cloudflare_trace_via_socks5(&settings.host, settings.port).await
        }
        SecondaryProxyMode::Embedded => {
            ensure_singbox_for_secondary_tests(&app).await?;
            test_embedded_secondary_proxy(&settings).await
        }
    }
}

async fn ensure_singbox_for_secondary_tests(app: &AppHandle) -> Result<(), String> {
    let configured_path = SettingsStorage::load().sing_box.executable_path;
    if Path::new(&configured_path).is_file() {
        return Ok(());
    }

    DependencyManager::install_singbox(Some(app))
        .await
        .map(|_| ())
        .map_err(|error| {
            format!("sing-box was missing. Automatic dependency repair failed: {error}")
        })
}

#[tauri::command]
pub async fn update_secondary_subscription(
    subscription_url: String,
    state: State<'_, AppState>,
) -> Result<SecondarySubscriptionUpdate, String> {
    const MAX_SUBSCRIPTION_BYTES: usize = 4 * 1024 * 1024;
    let url = url::Url::parse(subscription_url.trim())
        .map_err(|_| "Enter a valid HTTP or HTTPS subscription URL.".to_string())?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Subscription URL must use HTTP or HTTPS.".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Aether Desktop/0.1.3")
        .build()
        .map_err(|_| "Unable to initialize subscription client.".to_string())?;
    let response = client.get(url).send().await.map_err(|error| {
        if error.is_timeout() {
            "Subscription request timed out.".to_string()
        } else {
            "Subscription request failed.".to_string()
        }
    })?;
    if !response.status().is_success() {
        return Err(format!(
            "Subscription server returned HTTP {}.",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_SUBSCRIPTION_BYTES as u64)
    {
        return Err("Subscription response exceeds the 4 MiB limit.".to_string());
    }

    let mut body = Vec::new();
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|_| "Unable to read subscription response.".to_string())?;
        if body.len() + chunk.len() > MAX_SUBSCRIPTION_BYTES {
            return Err("Subscription response exceeds the 4 MiB limit.".to_string());
        }
        body.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(body)
        .map_err(|_| "Subscription response is not valid UTF-8 text.".to_string())?;
    let parsed = parse_subscription_payload(&text)?;
    state.logger.log(
        "INFO",
        "SecondaryProxy",
        format!(
            "Subscription updated: {} supported profiles, {} skipped",
            parsed.profiles.len(),
            parsed.skipped
        ),
    );
    Ok(SecondarySubscriptionUpdate {
        profiles: parsed.profiles,
        skipped: parsed.skipped,
    })
}

#[tauri::command]
pub async fn ping_secondary_profiles(
    profiles: Vec<SecondaryProxyProfile>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<SecondaryProfilePing>, String> {
    if profiles.is_empty() {
        return Err("Update the subscription before running Ping All.".to_string());
    }
    if profiles.len() > 200 {
        return Err("Ping All supports at most 200 profiles.".to_string());
    }
    ensure_singbox_for_secondary_tests(&app).await?;
    state.logger.log(
        "INFO",
        "SecondaryProxy",
        format!("Testing {} subscription profiles", profiles.len()),
    );

    let base_settings = SettingsStorage::load().secondary_proxy;
    let mut results: Vec<SecondaryProfilePing> = stream::iter(
        profiles
            .into_iter()
            .enumerate()
            .map(|(index, profile)| {
                let mut settings = base_settings.clone();
                async move {
                    settings.enabled = true;
                    settings.mode = SecondaryProxyMode::Embedded;
                    settings.share_link = profile.share_link;
                    match test_embedded_secondary_proxy(&settings).await {
                        Ok(trace) => SecondaryProfilePing {
                            index,
                            ok: true,
                            latency_ms: Some(trace.latency_ms),
                            ip: Some(trace.ip),
                            colo: Some(trace.colo),
                            error: None,
                        },
                        Err(error) => SecondaryProfilePing {
                            index,
                            ok: false,
                            latency_ms: None,
                            ip: None,
                            colo: None,
                            error: Some(error),
                        },
                    }
                }
            }),
    )
    .buffer_unordered(4)
    .collect()
    .await;
    results.sort_by_key(|result| result.index);
    Ok(results)
}

fn build_xray_test_config(link: &str, port: u16) -> Option<serde_json::Value> {
    if !link.starts_with("vless://") {
        return None;
    }
    let url = url::Url::parse(link).ok()?;
    let host = url.host_str()?.to_string();
    let server_port = url.port().unwrap_or(443);
    let uuid = percent_encoding::percent_decode_str(url.username())
        .decode_utf8()
        .ok()?
        .to_string();
    let query: std::collections::HashMap<String, String> = url
        .query_pairs()
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect();
    let security = query
        .get("security")
        .map(|s| s.as_str())
        .unwrap_or("none");
    let flow = query.get("flow").cloned().unwrap_or_default();
    let transport_type = query.get("type").map(|t| t.as_str()).unwrap_or("tcp");

    let mut stream_settings = serde_json::Map::new();
    stream_settings.insert(
        "network".to_string(),
        serde_json::json!(match transport_type {
            "ws" | "websocket" => "ws",
            "xhttp" => "xhttp",
            _ => "raw",
        }),
    );
    stream_settings.insert("security".to_string(), serde_json::json!(security));

    if security == "reality" {
        let sni = query
            .get("sni")
            .or_else(|| query.get("servername"))
            .cloned()
            .unwrap_or_else(|| host.clone());
        let pbk = query
            .get("pbk")
            .or_else(|| query.get("publickey"))?
            .clone();
        let sid = query
            .get("sid")
            .or_else(|| query.get("shortid"))
            .cloned()
            .unwrap_or_default();
        let fp = query
            .get("fp")
            .cloned()
            .unwrap_or_else(|| "chrome".to_string());
        let spx = query.get("spx").cloned().unwrap_or_default();

        stream_settings.insert(
            "realitySettings".to_string(),
            serde_json::json!({
                "serverName": sni,
                "fingerprint": fp,
                "show": false,
                "publicKey": pbk,
                "shortId": sid,
                "spiderX": spx
            }),
        );
    } else if security == "tls" {
        let sni = query
            .get("sni")
            .or_else(|| query.get("servername"))
            .cloned()
            .unwrap_or_else(|| host.clone());
        let fp = query
            .get("fp")
            .cloned()
            .unwrap_or_else(|| "chrome".to_string());
        stream_settings.insert(
            "tlsSettings".to_string(),
            serde_json::json!({
                "serverName": sni,
                "fingerprint": fp
            }),
        );
    }

    if transport_type == "ws" || transport_type == "websocket" {
        let path = query
            .get("path")
            .cloned()
            .unwrap_or_else(|| "/".to_string());
        let host_header = query
            .get("host")
            .cloned()
            .unwrap_or_else(|| host.clone());
        stream_settings.insert(
            "wsSettings".to_string(),
            serde_json::json!({
                "path": path,
                "headers": {
                    "Host": host_header
                }
            }),
        );
    }

    Some(serde_json::json!({
        "log": { "loglevel": "warning" },
        "inbounds": [{
            "tag": "test-in",
            "port": port,
            "listen": "127.0.0.1",
            "protocol": "mixed",
            "settings": { "auth": "noauth", "udp": true }
        }],
        "outbounds": [{
            "tag": "proxy",
            "protocol": "vless",
            "settings": {
                "vnext": [{
                    "address": host,
                    "port": server_port,
                    "users": [{
                        "id": uuid,
                        "encryption": "none",
                        "flow": if flow.is_empty() { serde_json::Value::Null } else { serde_json::json!(flow) }
                    }]
                }]
            },
            "streamSettings": stream_settings
        }]
    }))
}

async fn run_xray_probe(
    xray_path: &std::path::Path,
    xray_config: &serde_json::Value,
    port: u16,
) -> Result<CloudflareTrace, String> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let config_dir = SettingsStorage::get_config_dir();
    std::fs::create_dir_all(&config_dir)
        .map_err(|error| format!("Unable to create the config directory: {error}"))?;
    let config_path = config_dir.join(format!("xray-test-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(
        &config_path,
        serde_json::to_vec_pretty(xray_config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Unable to write the xray test config: {error}"))?;

    let mut run_command = std::process::Command::new(xray_path);
    run_command
        .args(["run", "-c"])
        .arg(&config_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    run_command.creation_flags(CREATE_NO_WINDOW);

    let mut child = run_command.spawn().map_err(|error| {
        let _ = std::fs::remove_file(&config_path);
        format!("Unable to start the Xray config test: {error}")
    })?;
    assign_child_to_global_job(&child);

    let mut ready = false;
    for _ in 0..40 {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => break,
        }
        if std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            std::time::Duration::from_millis(50),
        )
        .is_ok()
        {
            ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let result = if ready {
        HealthProber::query_cloudflare_trace_via_socks5("127.0.0.1", port).await
    } else {
        Err("Xray stopped before its local test port became ready.".to_string())
    };

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&config_path);
    result
}

async fn test_embedded_secondary_proxy(
    secondary: &SecondaryProxySettings,
) -> Result<CloudflareTrace, String> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("Unable to reserve a local test port: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Unable to read the local test port: {error}"))?
        .port();
    drop(listener);

    // 1. If Xray executable is available on the system, try it first for native Reality/WS compatibility
    if let Some(xray_path) = ProcessDetector::find_xray_executable() {
        if let Some(xray_config) = build_xray_test_config(&secondary.share_link, port) {
            if let Ok(trace) = run_xray_probe(&xray_path, &xray_config, port).await {
                return Ok(trace);
            }
        }
    }

    // 2. Primary / fallback sing-box probe
    let mut app_settings = SettingsStorage::load();
    app_settings.secondary_proxy = secondary.clone();
    app_settings.secondary_proxy.enabled = true;
    let outbound = SingBoxConfigGenerator::secondary_outbound(&app_settings)?;

    let config = serde_json::json!({
        "log": { "level": "warn", "timestamp": true },
        "dns": {
            "servers": [
                { "type": "udp", "tag": "direct-dns", "server": "8.8.8.8", "server_port": 53 },
                { "type": "local", "tag": "local-dns" }
            ],
            "strategy": "prefer_ipv4"
        },
        "inbounds": [{
            "type": "socks",
            "tag": "secondary-test",
            "listen": "127.0.0.1",
            "listen_port": port
        }],
        "outbounds": [
            outbound,
            { "type": "direct", "tag": "direct" }
        ],
        "route": {
            "auto_detect_interface": true,
            "default_domain_resolver": "direct-dns",
            "rules": [{
                "inbound": ["secondary-test"],
                "action": "route",
                "outbound": "v2ray"
            }],
            "final": "v2ray"
        }
    });
    let config_dir = SettingsStorage::get_config_dir();
    std::fs::create_dir_all(&config_dir)
        .map_err(|error| format!("Unable to create the config directory: {error}"))?;
    let config_path = config_dir.join(format!("secondary-test-{}.json", uuid::Uuid::new_v4()));
    std::fs::write(
        &config_path,
        serde_json::to_vec_pretty(&config).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("Unable to write the secondary test config: {error}"))?;

    let executable = &app_settings.sing_box.executable_path;
    let mut check_command = std::process::Command::new(executable);
    check_command
        .args(["check", "-c"])
        .arg(&config_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    check_command.creation_flags(CREATE_NO_WINDOW);
    let check = check_command.output().map_err(|error| {
        let _ = std::fs::remove_file(&config_path);
        format!("Unable to run sing-box config validation: {error}")
    })?;
    if !check.status.success() {
        let _ = std::fs::remove_file(&config_path);
        return Err(format!(
            "sing-box rejected this config: {}",
            String::from_utf8_lossy(&check.stderr).trim()
        ));
    }

    let mut run_command = std::process::Command::new(executable);
    run_command
        .args(["run", "-c"])
        .arg(&config_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    run_command.creation_flags(CREATE_NO_WINDOW);
    let mut child = run_command.spawn().map_err(|error| {
        let _ = std::fs::remove_file(&config_path);
        format!("Unable to start the secondary config test: {error}")
    })?;
    assign_child_to_global_job(&child);

    let mut ready = false;
    for _ in 0..40 {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&config_path);
                return Err(format!(
                    "Unable to inspect the secondary test process: {error}"
                ));
            }
        }
        if std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            std::time::Duration::from_millis(50),
        )
        .is_ok()
        {
            ready = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let result = if ready {
        HealthProber::query_cloudflare_trace_via_socks5("127.0.0.1", port).await
    } else {
        Err(
            "The built-in secondary proxy stopped before its local test port became ready."
                .to_string(),
        )
    };
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&config_path);
    result
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
pub fn save_exported_logs(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let raw_logs = state.logger.export_as_string();
    let formatted_logs = if raw_logs.contains("\r\n") {
        raw_logs
    } else {
        raw_logs.replace('\n', "\r\n")
    };
    std::fs::write(&path, formatted_logs.as_bytes())
        .map_err(|e| format!("Failed to write log file to '{}': {}", path, e))?;
    Ok(())
}

#[tauri::command]
pub fn get_platform() -> String {
    #[cfg(target_os = "android")]
    {
        "android".to_string()
    }
    #[cfg(target_os = "windows")]
    {
        "windows".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "macos".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "linux".to_string()
    }
    #[cfg(not(any(target_os = "android", target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

#[tauri::command]
pub fn validate_binaries() -> Result<BinaryValidationResult, String> {
    #[cfg(target_os = "android")]
    {
        return Ok(BinaryValidationResult {
            aether_exists: true,
            aether_path: "internal://android-vpn".to_string(),
            singbox_exists: true,
            singbox_path: "internal://android-box".to_string(),
        });
    }
    #[cfg(not(target_os = "android"))]
    {
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

#[tauri::command]
pub async fn ensure_dependencies_and_complete_setup(
    app: AppHandle,
) -> Result<DependencyStatus, String> {
    #[cfg(target_os = "android")]
    {
        let _ = app;
        let mut settings = SettingsStorage::load();
        settings.first_run_completed = true;
        let _ = SettingsStorage::save(&settings);
        return Ok(DependencyManager::check_status());
    }
    #[cfg(not(target_os = "android"))]
    {
        // 1. Check with discovery
        let mut status = DependencyManager::check_status();

        // 2. Install Aether if missing
        if !status.aether_installed {
            DependencyManager::install_aether(Some(&app)).await?;
            status = DependencyManager::check_status();
        }

        // 3. Install sing-box if missing
        if !status.singbox_installed {
            DependencyManager::install_singbox(Some(&app)).await?;
            status = DependencyManager::check_status();
        }

        // 4. Verify both are installed
        if !status.aether_installed || !status.singbox_installed {
            return Err("One or more core engines failed validation after installation.".to_string());
        }

        // 5. Mark first run completed and persist
        let mut settings = SettingsStorage::load();
        settings.first_run_completed = true;
        SettingsStorage::save(&settings)?;

        Ok(status)
    }
}

#[tauri::command]
pub async fn get_best_candidate_rtt(state: State<'_, AppState>) -> Result<Option<u32>, String> {
    Ok(state.orchestrator.get_best_candidate_rtt().await)
}
