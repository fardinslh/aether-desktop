use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::{AppSettings, ConnectionState};
use crate::process::runner::{AetherRunner, SingBoxRunner};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub struct ConnectionOrchestrator {
    aether_runner: Mutex<AetherRunner>,
    singbox_runner: Mutex<SingBoxRunner>,
    logger: RingBufferLogger,
    connection_state: Arc<RwLock<ConnectionState>>,
}

impl ConnectionOrchestrator {
    pub fn new(logger: RingBufferLogger, connection_state: Arc<RwLock<ConnectionState>>) -> Self {
        Self {
            aether_runner: Mutex::new(AetherRunner::new()),
            singbox_runner: Mutex::new(SingBoxRunner::new()),
            logger,
            connection_state,
        }
    }

    pub fn set_state(&self, state: ConnectionState) {
        *self.connection_state.write() = state;
        self.logger.log("INFO", "State", format!("Connection state transitioned to {:?}", state));
    }

    pub async fn connect(&self, settings: AppSettings) -> Result<(), String> {
        self.set_state(ConnectionState::StartingAether);

        // Step 1: Check or start Aether SOCKS proxy
        let aether_already_running = HealthProber::check_port_open(&settings.aether.host, settings.aether.port, 500).await;
        if !aether_already_running {
            let mut runner = self.aether_runner.lock();
            if let Err(e) = runner.start(&settings, &self.logger) {
                self.set_state(ConnectionState::Error);
                return Err(e);
            }
        } else {
            self.logger.log("INFO", "Aether", "Aether proxy already listening on 127.0.0.1:1819");
        }

        // Step 2: Waiting for Aether SOCKS5 proxy port to become available
        self.set_state(ConnectionState::WaitingForAether);
        let mut aether_ready = false;
        for attempt in 1..=15 {
            if HealthProber::check_port_open(&settings.aether.host, settings.aether.port, 400).await {
                aether_ready = true;
                break;
            }
            sleep(Duration::from_millis(300)).await;
            self.logger.log("DEBUG", "Aether", format!("Waiting for Aether SOCKS5 port (attempt {}/15)...", attempt));
        }

        if !aether_ready {
            self.set_state(ConnectionState::Error);
            let err = format!("Aether failed to listen on {}:{} within timeout", settings.aether.host, settings.aether.port);
            self.logger.log("ERROR", "Aether", &err);
            return Err(err);
        }

        // Step 3: Test Aether SOCKS proxy connectivity
        self.set_state(ConnectionState::TestingAether);
        sleep(Duration::from_millis(300)).await;

        // Step 4: Start sing-box TUN router
        self.set_state(ConnectionState::StartingRouter);
        {
            let mut router = self.singbox_runner.lock();
            if let Err(e) = router.start(&settings, &self.logger) {
                self.set_state(ConnectionState::Error);
                return Err(e);
            }
        }

        // Step 5: Test routing & verify sing-box stayed alive
        self.set_state(ConnectionState::TestingRouting);
        sleep(Duration::from_millis(600)).await;

        {
            let mut router = self.singbox_runner.lock();
            if !router.is_running() {
                self.set_state(ConnectionState::Error);
                let err = "sing-box router process exited unexpectedly. Ensure the application is running with Administrator privileges to configure the TUN interface.".to_string();
                self.logger.log("ERROR", "sing-box", &err);
                return Err(err);
            }
        }

        // Step 6: Fully connected
        self.set_state(ConnectionState::Connected);
        self.logger.log("INFO", "Orchestrator", "All networking components connected successfully");
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        self.set_state(ConnectionState::Disconnecting);

        // Stop sing-box first to restore default routing cleanly
        {
            let mut router = self.singbox_runner.lock();
            router.stop(&self.logger);
        }

        // Stop Aether if managed by this app
        {
            let mut aether = self.aether_runner.lock();
            aether.stop(&self.logger);
        }

        self.set_state(ConnectionState::Disconnected);
        self.logger.log("INFO", "Orchestrator", "Disconnected cleanly");
        Ok(())
    }

    pub async fn apply_live_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let is_connected = *self.connection_state.read() == ConnectionState::Connected;
        if is_connected {
            let mut router = self.singbox_runner.lock();
            router.restart_transparently(settings, &self.logger)?;
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        let mut router = self.singbox_runner.lock();
        router.stop(&self.logger);
        let mut aether = self.aether_runner.lock();
        aether.stop(&self.logger);
    }
}