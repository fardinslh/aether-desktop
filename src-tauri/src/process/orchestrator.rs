use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::health::HealthStatus;
use crate::models::{AppSettings, ConnectionState};
use crate::process::runner::{AetherRunner, SingBoxRunner};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Duration;

pub struct ConnectionOrchestrator {
    pub aether: Mutex<AetherRunner>,
    pub singbox: Mutex<SingBoxRunner>,
    pub state: Arc<RwLock<ConnectionState>>,
    pub logger: RingBufferLogger,
    pub last_error: Arc<RwLock<Option<String>>>,
}

impl ConnectionOrchestrator {
    pub fn new(state: Arc<RwLock<ConnectionState>>, logger: RingBufferLogger) -> Self {
        Self {
            aether: Mutex::new(AetherRunner::new()),
            singbox: Mutex::new(SingBoxRunner::new()),
            state,
            logger,
            last_error: Arc::new(RwLock::new(None)),
        }
    }

    fn set_state(&self, new_state: ConnectionState) {
        *self.state.write() = new_state;
        self.logger.log(
            "INFO",
            "STATE",
            format!("State changed to: {:?}", new_state),
        );
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

        // 1. Phase 1: Launch Aether
        self.set_state(ConnectionState::StartingAether);
        {
            let mut aether_guard = self.aether.lock();
            if let Err(e) = aether_guard.start(settings, &self.logger) {
                self.set_error(format!("Failed to start Aether: {}", e));
                return Err(e);
            }
        }

        // 2. Phase 2: Probe Aether socket and tunnel connectivity via real SOCKS request
        self.set_state(ConnectionState::TestingAether);
        let aether_host = &settings.aether.host;
        let aether_port = settings.aether.port;

        let mut aether_ready = false;
        for attempt in 1..=20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if HealthProber::check_port_open(aether_host, aether_port, 300).await {
                match HealthProber::query_cloudflare_trace_via_socks5(aether_host, aether_port)
                    .await
                {
                    Ok(trace) => {
                        self.logger.log(
                            "INFO",
                            "Aether",
                            format!(
                                "Aether SOCKS5 tunnel confirmed online (POP: {}, IP: {}, Latency: {} ms)",
                                trace.colo, trace.ip, trace.latency_ms
                            ),
                        );
                        aether_ready = true;
                        break;
                    }
                    Err(err) => {
                        self.logger.log(
                            "DEBUG",
                            "Aether",
                            format!("Trace probe attempt {}/20 pending: {}", attempt, err),
                        );
                    }
                }
            }
        }

        if !aether_ready {
            let err = format!(
                "Aether failed to establish verified proxy tunnel on {}:{} within timeout. Check Aether credentials and server status.",
                aether_host, aether_port
            );
            self.aether.lock().stop(&self.logger);
            self.set_error(err.clone());
            return Err(err);
        }

        // 3. Phase 3: Launch sing-box router
        self.set_state(ConnectionState::StartingRouter);
        {
            let mut sb_guard = self.singbox.lock();
            if let Err(e) = sb_guard.start(settings, &self.logger) {
                self.aether.lock().stop(&self.logger);
                self.set_error(format!("Failed to start sing-box TUN router: {}", e));
                return Err(e);
            }
        }

        // 4. Phase 4: Bounded verification loop for sing-box process, TUN adapter, and direct system egress
        self.set_state(ConnectionState::TestingRouting);
        let interface_name = &settings.sing_box.interface_name;

        let mut tun_detected = false;
        for attempt in 1..=24 {
            tokio::time::sleep(Duration::from_millis(250)).await;

            if !self.singbox.lock().is_running() {
                let err =
                    "sing-box TUN router process exited unexpectedly during startup.".to_string();
                self.singbox.lock().stop(&self.logger);
                self.aether.lock().stop(&self.logger);
                self.set_error(err.clone());
                return Err(err);
            }

            if HealthProber::check_tun_interface_exists(interface_name) {
                self.logger.log(
                    "INFO",
                    "sing-box",
                    format!("TUN network interface '{}' detected active in Windows network stack on attempt {}", interface_name, attempt),
                );
                tun_detected = true;
                break;
            }
        }

        if !tun_detected {
            let err = format!(
                "sing-box TUN interface '{}' failed to appear in Windows network stack within timeout. Ensure the application is running with Administrator elevation.",
                interface_name
            );
            self.singbox.lock().stop(&self.logger);
            self.aether.lock().stop(&self.logger);
            self.set_error(err.clone());
            return Err(err);
        }

        // 5. Phase 5: Real direct system egress verification through Windows TUN routing
        self.logger.log(
            "INFO",
            "ROUTING",
            "Verifying direct system egress routing through TUN adapter...",
        );
        tokio::time::sleep(Duration::from_millis(300)).await;

        let mut system_egress_ok = false;
        let mut last_egress_err = String::new();

        for attempt in 1..=6 {
            match HealthProber::query_direct_system_cloudflare_trace().await {
                Ok(trace) => {
                    self.logger.log(
                        "INFO",
                        "ROUTING",
                        format!(
                            "Direct system egress verified through TUN routing (POP: {}, IP: {}, Latency: {} ms)",
                            trace.colo, trace.ip, trace.latency_ms
                        ),
                    );
                    system_egress_ok = true;
                    break;
                }
                Err(err) => {
                    last_egress_err = err.clone();
                    self.logger.log(
                        "DEBUG",
                        "ROUTING",
                        format!("System egress test attempt {}/6: {}", attempt, err),
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        if !system_egress_ok {
            let err = format!(
                "System egress routing verification failed through TUN adapter: {}. Stopping connection attempt.",
                last_egress_err
            );
            self.singbox.lock().stop(&self.logger);
            self.aether.lock().stop(&self.logger);
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

        self.singbox.lock().stop(&self.logger);
        self.aether.lock().stop(&self.logger);

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
            let mut sb_guard = self.singbox.lock();
            sb_guard.restart_transparently(settings, &self.logger)
        } else {
            Ok(())
        }
    }

    pub async fn check_health(&self, settings: &AppSettings) -> HealthStatus {
        let aether_running = self.aether.lock().is_running();
        let singbox_running = self.singbox.lock().is_running();
        let is_conn = *self.state.read() == ConnectionState::Connected;

        HealthProber::evaluate_health(
            &settings.aether.host,
            settings.aether.port,
            &settings.secondary_proxy.host,
            settings.secondary_proxy.port,
            settings.secondary_proxy.enabled,
            &settings.sing_box.interface_name,
            aether_running,
            singbox_running,
            is_conn,
        )
        .await
    }
}
