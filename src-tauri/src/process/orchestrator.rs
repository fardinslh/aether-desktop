use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::health::HealthStatus;
use crate::models::{AppSettings, ConnectionState};
use crate::process::detector::ProcessDetector;
use crate::process::runner::{AetherRunner, SingBoxRunner};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Mutex;

pub struct ConnectionOrchestrator {
    pub aether: Mutex<AetherRunner>,
    pub singbox: Mutex<SingBoxRunner>,
    pub is_aether_managed: AtomicBool,
    pub active_aether_ip: Arc<RwLock<Option<String>>>,
    pub state: Arc<RwLock<ConnectionState>>,
    pub logger: RingBufferLogger,
    pub last_error: Arc<RwLock<Option<String>>>,
    pub app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
}

impl ConnectionOrchestrator {
    pub fn new(state: Arc<RwLock<ConnectionState>>, logger: RingBufferLogger) -> Self {
        Self {
            aether: Mutex::new(AetherRunner::new()),
            singbox: Mutex::new(SingBoxRunner::new()),
            is_aether_managed: AtomicBool::new(false),
            active_aether_ip: Arc::new(RwLock::new(None)),
            state,
            logger,
            last_error: Arc::new(RwLock::new(None)),
            app_handle: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.write() = Some(handle);
    }

    fn set_state(&self, new_state: ConnectionState) {
        *self.state.write() = new_state;
        self.logger.log(
            "INFO",
            "STATE",
            format!("State changed to: {:?}", new_state),
        );
        if let Some(ref handle) = *self.app_handle.read() {
            let _ = handle.emit("connection-state-changed", new_state);
        }
    }

    fn set_error(&self, err_msg: String) {
        *self.last_error.write() = Some(err_msg.clone());
        self.logger
            .log("ERROR", "STATE", format!("Connection error: {}", err_msg));
        self.set_state(ConnectionState::Error);
    }

    pub fn get_last_error(&self) -> Option<String> {
        self.last_error.read().clone()
    }

    pub async fn connect(&self, settings: &AppSettings) -> Result<(), String> {
        let current = *self.state.read();
        if current != ConnectionState::Disconnected && current != ConnectionState::Error {
            return Err("Connection already in progress or connected".to_string());
        }

        *self.last_error.write() = None;
        *self.active_aether_ip.write() = None;

        let aether_host = &settings.aether.host;
        let aether_port = settings.aether.port;

        // 1. Safe Existing-Aether Check with Process Owner Validation
        let is_port_occupied = HealthProber::check_port_open(aether_host, aether_port, 300).await;
        if is_port_occupied {
            let owner_info = ProcessDetector::get_process_for_tcp_port(aether_port);
            match owner_info {
                Some((pid, proc_name)) => {
                    let proc_lower = proc_name.to_lowercase();
                    if !proc_lower.starts_with("aether") {
                        let err = format!(
                            "Port {}:{} is in use by another process ('{}', PID: {}), which is not Aether. Resolve port conflict.",
                            aether_host, aether_port, proc_name, pid
                        );
                        self.set_error(err.clone());
                        return Err(err);
                    }

                    self.logger.log(
                        "INFO",
                        "Aether",
                        format!("Existing Aether process detected (PID: {}, '{}') on {}:{}. Validating proxy health...", pid, proc_name, aether_host, aether_port),
                    );

                    match HealthProber::query_cloudflare_trace_via_socks5(aether_host, aether_port)
                        .await
                    {
                        Ok(trace) => {
                            self.logger.log(
                                "INFO",
                                "Aether",
                                format!(
                                    "Reusing existing healthy external Aether instance (PID: {}) on {}:{} (POP: {}, IP: {}, Latency: {} ms)",
                                    pid, aether_host, aether_port, trace.colo, trace.ip, trace.latency_ms
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            self.is_aether_managed.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            let err = format!(
                                "Port {}:{} is owned by an existing Aether process (PID: {}), but SOCKS5 validation failed: {}. Ensure Aether is connected.",
                                aether_host, aether_port, pid, e
                            );
                            self.set_error(err.clone());
                            return Err(err);
                        }
                    }
                }
                None => {
                    // Could not query PID owner, probe SOCKS directly
                    match HealthProber::query_cloudflare_trace_via_socks5(aether_host, aether_port)
                        .await
                    {
                        Ok(trace) => {
                            self.logger.log(
                                "INFO",
                                "Aether",
                                format!(
                                    "Reusing existing healthy external listener on {}:{} (POP: {}, IP: {})",
                                    aether_host, aether_port, trace.colo, trace.ip
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            self.is_aether_managed.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            let err = format!(
                                "Port {}:{} is in use, but proxy validation failed: {}. Resolve port conflict.",
                                aether_host, aether_port, e
                            );
                            self.set_error(err.clone());
                            return Err(err);
                        }
                    }
                }
            }
        } else {
            // Port is free: launch managed Aether instance
            self.set_state(ConnectionState::StartingAether);
            {
                let mut aether_guard = self.aether.lock().await;
                if let Err(e) = aether_guard.start(settings, &self.logger) {
                    self.set_error(format!("Failed to start Aether: {}", e));
                    return Err(e);
                }
            }
            self.is_aether_managed.store(true, Ordering::SeqCst);

            // Probe managed Aether with scan-mode-aware startup deadline
            self.set_state(ConnectionState::ScanningAether);
            let startup_deadline =
                crate::models::settings::aether_startup_timeout(&settings.aether.scan_mode);
            let start_instant = std::time::Instant::now();
            let check_interval = Duration::from_millis(400);

            self.logger.log(
                "INFO",
                "Aether",
                format!(
                    "Waiting for Aether SOCKS proxy on {}:{} (Scan Mode: {:?}, Deadline: {}s)...",
                    aether_host,
                    aether_port,
                    settings.aether.scan_mode,
                    startup_deadline.as_secs()
                ),
            );

            let mut aether_ready = false;
            let mut attempt: u64 = 0;
            while start_instant.elapsed() < startup_deadline {
                tokio::time::sleep(check_interval).await;
                attempt += 1;

                // 1. Immediate interactive prompt abort check & process liveness
                {
                    let mut aether_guard = self.aether.lock().await;
                    if aether_guard.is_interactive_prompt_detected() {
                        let err = "Aether entered interactive mode. Managed launch arguments are incomplete.".to_string();
                        aether_guard.stop(&self.logger);
                        self.set_error(err.clone());
                        return Err(err);
                    }
                    if !aether_guard.is_running() {
                        let err =
                            "Aether process stopped unexpectedly. View Diagnostics for details."
                                .to_string();
                        aether_guard.stop(&self.logger);
                        self.set_error(err.clone());
                        return Err(err);
                    }
                }

                // 2. SOCKS5 probe
                if HealthProber::check_port_open(aether_host, aether_port, 250).await {
                    match HealthProber::query_cloudflare_trace_via_socks5(aether_host, aether_port)
                        .await
                    {
                        Ok(trace) => {
                            self.logger.log(
                                "INFO",
                                "Aether",
                                format!(
                                    "Aether SOCKS5 tunnel confirmed online (POP: {}, IP: {}, Latency: {} ms, Elapsed: {:.1}s)",
                                    trace.colo, trace.ip, trace.latency_ms, start_instant.elapsed().as_secs_f32()
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            aether_ready = true;
                            break;
                        }
                        Err(err) => {
                            if attempt % 15 == 0 {
                                self.logger.log(
                                    "DEBUG",
                                    "Aether",
                                    format!(
                                        "Waiting for SOCKS5 proxy initialization ({:.1}s elapsed): {}",
                                        start_instant.elapsed().as_secs_f32(),
                                        err
                                    ),
                                );
                            }
                        }
                    }
                }
            }

            if !aether_ready {
                let err = format!(
                    "Aether started, but local proxy on {}:{} did not become ready within {}s deadline (Scan Mode: {:?}). View Diagnostics for details.",
                    aether_host, aether_port, startup_deadline.as_secs(), settings.aether.scan_mode
                );
                self.aether.lock().await.stop(&self.logger);
                self.set_error(err.clone());
                return Err(err);
            }
        }

        // Check Windows administrator elevation before launching TUN
        if !HealthProber::is_process_elevated() {
            let err = "Administrator privileges are required to create the Windows TUN adapter."
                .to_string();
            self.logger.log("ERROR", "UAC", &err);
            if self.is_aether_managed.load(Ordering::SeqCst) {
                self.aether.lock().await.stop(&self.logger);
            }
            self.set_error(err.clone());
            return Err(err);
        }

        // 2. Launch sing-box router
        self.set_state(ConnectionState::StartingRouter);
        {
            let mut sb_guard = self.singbox.lock().await;
            if let Err(e) = sb_guard.start(settings, &self.logger) {
                if self.is_aether_managed.load(Ordering::SeqCst) {
                    self.aether.lock().await.stop(&self.logger);
                }
                self.set_error(format!("Failed to start sing-box TUN router: {}", e));
                return Err(e);
            }
        }

        // 3. Bounded routing and system egress verification matching Aether IP
        self.set_state(ConnectionState::TestingRouting);
        let interface_name = &settings.sing_box.interface_name;
        let tun_address = &settings.sing_box.tun_address;
        let expected_aether_ip = self.active_aether_ip.read().clone();

        let mut sb_guard = self.singbox.lock().await;
        if let Err(verify_err) = sb_guard
            .verify_router_and_egress(
                interface_name,
                Some(tun_address),
                Duration::from_secs(6),
                expected_aether_ip.as_deref(),
                &self.logger,
            )
            .await
        {
            let err = format!(
                "Routing verification failed: {}. Stopping connection attempt.",
                verify_err
            );
            sb_guard.stop(&self.logger);
            drop(sb_guard);
            if self.is_aether_managed.load(Ordering::SeqCst) {
                self.aether.lock().await.stop(&self.logger);
            }
            self.set_error(err.clone());
            return Err(err);
        }

        self.set_state(ConnectionState::Connected);
        self.logger.log(
            "INFO",
            "STATE",
            "Connection established and fully verified successfully.",
        );
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        let current = *self.state.read();
        if current == ConnectionState::Disconnected {
            return Ok(());
        }

        self.set_state(ConnectionState::Disconnecting);

        // Always stop sing-box TUN router
        self.singbox.lock().await.stop(&self.logger);

        // Only stop Aether if it was spawned and managed by this app
        if self.is_aether_managed.load(Ordering::SeqCst) {
            self.logger
                .log("INFO", "Aether", "Terminating managed Aether process...");
            self.aether.lock().await.stop(&self.logger);
        } else {
            self.logger.log(
                "INFO",
                "Aether",
                "Preserving external Aether instance on disconnect.",
            );
        }

        *self.active_aether_ip.write() = None;
        self.set_state(ConnectionState::Disconnected);
        self.logger.log(
            "INFO",
            "STATE",
            "All VPN components stopped and disconnected.",
        );
        Ok(())
    }

    pub async fn apply_live_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let current = *self.state.read();
        if current == ConnectionState::Connected {
            self.logger.log(
                "INFO",
                "ROUTING",
                "Applying live settings to running sing-box router...",
            );
            let expected_aether_ip = self.active_aether_ip.read().clone();
            let mut sb_guard = self.singbox.lock().await;
            sb_guard
                .restart_transparently(settings, &self.logger, expected_aether_ip.as_deref())
                .await
        } else {
            Ok(())
        }
    }

    pub async fn check_health(&self, settings: &AppSettings) -> HealthStatus {
        let aether_running = self.aether.lock().await.is_running();
        let singbox_running = self.singbox.lock().await.is_running();
        let is_conn = *self.state.read() == ConnectionState::Connected;

        HealthProber::evaluate_health(
            &settings.aether.host,
            settings.aether.port,
            &settings.secondary_proxy.host,
            settings.secondary_proxy.port,
            settings.secondary_proxy.enabled,
            &settings.sing_box.interface_name,
            Some(&settings.sing_box.tun_address),
            aether_running,
            singbox_running,
            is_conn,
        )
        .await
    }
}
