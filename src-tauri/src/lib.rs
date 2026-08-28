pub mod commands;
pub mod dependencies;
pub mod health;
pub mod logging;
pub mod models;
pub mod process;
pub mod routing;
pub mod settings;

use commands::AppState;
use logging::RingBufferLogger;
use models::ConnectionState;
use parking_lot::RwLock;
use process::ConnectionOrchestrator;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let logger = RingBufferLogger::new(1000);
    logger.log("INFO", "App", "Aether Desktop initialized");

    let connection_state = Arc::new(RwLock::new(ConnectionState::Disconnected));
    let orchestrator = Arc::new(ConnectionOrchestrator::new(
        logger.clone(),
        connection_state.clone(),
    ));

    let app_state = AppState {
        logger: logger.clone(),
        connection_state: connection_state.clone(),
        orchestrator: orchestrator.clone(),
    };

    let orchestrator_exit = orchestrator.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::get_connection_state,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::get_health_status,
            commands::get_running_applications,
            commands::inspect_executable_file,
            commands::pick_executable_file,
            commands::generate_singbox_config_preview,
            commands::test_secondary_proxy,
            commands::test_aether_proxy,
            commands::get_logs,
            commands::export_logs,
            commands::validate_binaries,
            commands::check_dependencies,
            commands::install_aether_dependency,
            commands::install_singbox_dependency
        ])
        .build(tauri::generate_context!())
        .expect("error while building aether desktop application")
        .run(move |_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                orchestrator_exit.shutdown();
            }
        });
}
