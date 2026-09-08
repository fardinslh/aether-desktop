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
use tauri::Manager;

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

            #[cfg(target_os = "android")]
            {
                if let Ok(data_dir) = app.path().app_data_dir() {
                    std::env::set_var("AETHER_DESKTOP_CONFIG_DIR", &data_dir);
                    let _ = std::fs::create_dir_all(&data_dir);
                }
            }

            #[cfg(desktop)]
            {
                let show_item = tauri::menu::MenuItem::with_id(
                    app,
                    "show",
                    "Show Aether Desktop",
                    true,
                    None::<&str>,
                )?;
                let quit_item = tauri::menu::MenuItem::with_id(
                    app,
                    "quit",
                    "Exit Aether Desktop",
                    true,
                    None::<&str>,
                )?;
                let tray_menu = tauri::menu::Menu::with_items(app, &[&show_item, &quit_item])?;
                let mut tray = tauri::tray::TrayIconBuilder::new()
                    .tooltip("Aether Desktop")
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::DoubleClick {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.build(app)?;
            }

            let settings = crate::settings::SettingsStorage::load();
            #[cfg(desktop)]
            if settings.general.start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            if settings.general.auto_connect {
                let orchestrator = orchestrator_setup.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = orchestrator.connect(&settings).await;
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_platform,
            commands::get_settings,
            commands::save_settings,
            commands::reset_settings,
            commands::get_connection_state,
            commands::connect_tunnel,
            commands::find_faster_gateway,
            commands::disconnect_tunnel,
            commands::cancel_connection,
            commands::get_health_status,
            commands::get_running_applications,
            commands::inspect_executable_file,
            commands::pick_executable_file,
            commands::validate_aether_path,
            commands::validate_singbox_path,
            commands::generate_singbox_config_preview,
            commands::test_secondary_proxy,
            commands::update_secondary_subscription,
            commands::ping_secondary_profiles,
            commands::test_aether_proxy,
            commands::get_logs,
            commands::export_logs,
            commands::save_exported_logs,
            commands::validate_binaries,
            commands::check_dependencies,
            commands::install_aether_dependency,
            commands::install_singbox_dependency,
            commands::ensure_dependencies_and_complete_setup,
            commands::get_best_candidate_rtt
        ])
        .build(tauri::generate_context!())
        .expect("error while building aether desktop application")
        .run(move |app_handle, event| match event {
            tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::CloseRequested { api, .. },
                ..
            } => {
                #[cfg(desktop)]
                if crate::settings::SettingsStorage::load()
                    .general
                    .minimize_to_tray
                {
                    api.prevent_close();
                    if let Some(window) = app_handle.get_webview_window(&label) {
                        let _ = window.hide();
                    }
                    return;
                }
                #[cfg(not(desktop))]
                {
                    let _ = label;
                    let _ = api;
                }
                orchestrator_exit.force_shutdown();
            }
            tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::Destroyed,
                ..
            }
            | tauri::RunEvent::ExitRequested { .. }
            | tauri::RunEvent::Exit => {
                orchestrator_exit.force_shutdown();
            }
            _ => {}
        });
}
