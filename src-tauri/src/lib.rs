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
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let msg = format!(
            "Aether Desktop encountered a critical error and must close.\n\nDetails: {}\nLocation: {:?}",
            payload,
            info.location()
        );
        eprintln!("{}", msg);

        if let Some(dirs) = directories::BaseDirs::new() {
            let crash_dir = dirs.data_local_dir().join("AetherDesktop");
            let _ = std::fs::create_dir_all(&crash_dir);
            let crash_file = crash_dir.join("crash.log");
            let _ = std::fs::write(&crash_file, &msg);
        }

        #[cfg(windows)]
        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            let msg_w: Vec<u16> = OsStr::new(&msg).encode_wide().chain(Some(0)).collect();
            let title_w: Vec<u16> = OsStr::new("Aether Desktop Critical Error")
                .encode_wide()
                .chain(Some(0))
                .collect();
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                msg_w.as_ptr(),
                title_w.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }));

    let logger = RingBufferLogger::new(10000);
    logger.log("INFO", "App", "Aether Desktop initialized");

    let connection_state = Arc::new(RwLock::new(ConnectionState::Disconnected));
    let orchestrator = Arc::new(ConnectionOrchestrator::new(
        connection_state.clone(),
        logger.clone(),
    ));

    let app_state = AppState {
        logger: logger.clone(),
        connection_state: connection_state.clone(),
        orchestrator: orchestrator.clone(),
    };

    let orchestrator_setup = orchestrator.clone();
    let orchestrator_exit = orchestrator.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .setup(move |app| {
            orchestrator_setup.set_app_handle(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::get_connection_state,
            commands::connect_tunnel,
            commands::find_faster_gateway,
            commands::disconnect_tunnel,
            commands::get_health_status,
            commands::get_running_applications,
            commands::inspect_executable_file,
            commands::pick_executable_file,
            commands::validate_aether_path,
            commands::validate_singbox_path,
            commands::generate_singbox_config_preview,
            commands::test_secondary_proxy,
            commands::test_aether_proxy,
            commands::get_logs,
            commands::export_logs,
            commands::save_exported_logs,
            commands::validate_binaries,
            commands::check_dependencies,
            commands::install_aether_dependency,
            commands::install_singbox_dependency,
            commands::get_best_candidate_rtt
        ])
        .build(tauri::generate_context!())
        .expect("error while building aether desktop application")
        .run(move |_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                let _ = tokio::runtime::Runtime::new()
                    .map(|rt| rt.block_on(orchestrator_exit.disconnect()));
            }
        });
}
