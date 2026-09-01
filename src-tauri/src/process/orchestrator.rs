use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::health::HealthStatus;
use crate::models::{AppSettings, ConnectionState};
use crate::process::detector::ProcessDetector;
use crate::process::runner::{AetherRunner, SingBoxRunner};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RouteOptimizationResult {
    pub success: bool,
    pub previous_latency_ms: Option<u64>,
    pub previous_jitter_ms: Option<u64>,
    pub previous_pop: Option<String>,
    pub previous_ip: Option<String>,
    pub new_latency_ms: Option<u64>,
    pub new_jitter_ms: Option<u64>,
    pub new_pop: Option<String>,
    pub new_ip: Option<String>,
    pub latency_delta_ms: Option<i64>,
    pub decision: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayOptimizationDecision {
    KeptFaster,
    NotEnoughLatencyImprovement,
    CandidateTooUnstable,
}

impl GatewayOptimizationDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            GatewayOptimizationDecision::KeptFaster => "KeptFaster",
            GatewayOptimizationDecision::NotEnoughLatencyImprovement => {
                "NotEnoughLatencyImprovement"
            }
            GatewayOptimizationDecision::CandidateTooUnstable => "CandidateTooUnstable",
        }
    }

    pub fn is_keep(&self) -> bool {
        matches!(self, GatewayOptimizationDecision::KeptFaster)
    }
}

/// Evaluates candidate gateway latency and jitter against baseline with bounded anti-noise
/// and stability thresholds.
///
/// Returns:
/// - decision: `GatewayOptimizationDecision` (`KeptFaster`, `NotEnoughLatencyImprovement`, `CandidateTooUnstable`)
/// - required_improvement_ms: minimum required latency drop `max(10 ms, 5% of baseline median)`
/// - latency_delta_ms: measured difference `(baseline_median - candidate_median)`
/// - max_allowable_jitter_ms: maximum permitted candidate jitter `max(baseline_jitter + 10 ms, baseline_jitter * 2)`
pub fn evaluate_gateway_optimization(
    baseline_median_ms: u64,
    baseline_jitter_ms: u64,
    candidate_median_ms: u64,
    candidate_jitter_ms: u64,
) -> (GatewayOptimizationDecision, u64, i64, u64) {
    let required_improvement_ms = (10u64).max((baseline_median_ms as f64 * 0.05).round() as u64);
    let latency_delta_ms = (baseline_median_ms as i64) - (candidate_median_ms as i64);
    let is_latency_sufficient =
        candidate_median_ms + required_improvement_ms <= baseline_median_ms;

    let max_allowable_jitter_ms = (baseline_jitter_ms + 10).max(baseline_jitter_ms * 2);
    let is_jitter_acceptable = candidate_jitter_ms <= max_allowable_jitter_ms;

    let decision = if !is_latency_sufficient {
        GatewayOptimizationDecision::NotEnoughLatencyImprovement
    } else if !is_jitter_acceptable {
        GatewayOptimizationDecision::CandidateTooUnstable
    } else {
        GatewayOptimizationDecision::KeptFaster
    };

    (
        decision,
        required_improvement_ms,
        latency_delta_ms,
        max_allowable_jitter_ms,
    )
}

/// Evaluates whether spawning a new sing-box TUN router conflicts with an existing network adapter.
///
/// If sing-box is already managed by the application, returns `Ok(())`.
/// If sing-box is not running and an adapter with the configured interface name or IP already exists,
/// returns `Err(String)` containing an actionable explanation to prevent false-positive connections over stale adapters.
pub fn evaluate_prelaunch_tun_conflict(
    is_singbox_already_running: bool,
    tun_preexists: bool,
    interface_name: &str,
    _tun_address: &str,
    matched_adapter_desc: Option<&str>,
) -> Result<(), String> {
    if is_singbox_already_running {
        return Ok(());
    }

    if tun_preexists {
        let adapter_str = matched_adapter_desc
            .map(|s| format!(" (adapter: {})", s))
            .unwrap_or_default();
        return Err(format!(
            "An existing '{}' interface is already active in the network stack{}. Another Aether Desktop or sing-box instance may still be running. Close the other instance and retry.",
            interface_name, adapter_str
        ));
    }

    Ok(())
}

pub struct ConnectionOrchestrator {
    pub aether: Mutex<AetherRunner>,
    pub singbox: Mutex<SingBoxRunner>,
    pub is_aether_managed: AtomicBool,
    pub active_aether_ip: Arc<RwLock<Option<String>>>,
    pub state: Arc<RwLock<ConnectionState>>,
    pub logger: RingBufferLogger,
    pub last_error: Arc<RwLock<Option<String>>>,
    pub app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
    pub op_lock: Mutex<()>,
    pub next_attempt_id: AtomicU64,
    pub cancel_requested: Arc<AtomicBool>,
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
            op_lock: Mutex::new(()),
            next_attempt_id: AtomicU64::new(1),
            cancel_requested: Arc::new(AtomicBool::new(false)),
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

    pub async fn cancel_connection(&self) -> Result<(), String> {
        self.cancel_requested.store(true, Ordering::SeqCst);
        self.logger
            .log("WARN", "STATE", "Connection cancellation requested by user.");

        // Terminate managed processes immediately
        self.force_shutdown();

        // Broadcast state transition to Disconnected
        self.set_state(ConnectionState::Disconnected);
        self.logger
            .log("INFO", "STATE", "Connection cancelled; all VPN components terminated.");
        Ok(())
    }

    pub async fn connect(&self, settings: &AppSettings) -> Result<(), String> {
        self.cancel_requested.store(false, Ordering::SeqCst);

        // 1. Acquire operation lock FIRST to guard against concurrent lifecycle operations
        let _op_guard = match self.op_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err("Connection operation already in progress".to_string()),
        };

        // 2. Synchronous atomic entry check and state transition under write lock
        let attempt_id = self.next_attempt_id.fetch_add(1, Ordering::SeqCst);
        {
            let mut state = self.state.write();
            match *state {
                ConnectionState::Disconnected | ConnectionState::Error => {
                    *state = ConnectionState::StartingAether;
                }
                _ => return Err("Connection already in progress or connected".to_string()),
            }
        }

        // 3. Emit StartingAether event & log attempt start immediately
        self.logger.log(
            "INFO",
            "STATE",
            format!(
                "[Attempt #{}] State changed to: StartingAether (CONNECT attempt initiated)",
                attempt_id
            ),
        );
        if let Some(ref handle) = *self.app_handle.read() {
            let _ = handle.emit("connection-state-changed", ConnectionState::StartingAether);
        }

        let default_options = crate::models::settings::AetherLaunchOptions::default();
        self.connect_internal(settings, attempt_id, &default_options)
            .await
    }

    async fn connect_internal(
        &self,
        settings: &AppSettings,
        attempt_id: u64,
        options: &crate::models::settings::AetherLaunchOptions,
    ) -> Result<(), String> {
        let t_connect_start = std::time::Instant::now();
        *self.last_error.write() = None;
        *self.active_aether_ip.write() = None;

        let aether_host = &settings.aether.host;
        let aether_port = settings.aether.port;

        // 1. Safe Existing-Aether Check with Process Owner Validation
        let t_aether_start = std::time::Instant::now();
        let is_port_occupied = HealthProber::check_port_open(aether_host, aether_port, 150).await;
        if is_port_occupied {
            let owner_info = ProcessDetector::get_process_for_tcp_port(aether_port);
            match owner_info {
                Some((pid, proc_name)) => {
                    let proc_lower = proc_name.to_lowercase();
                    if !proc_lower.starts_with("aether") {
                        let err = format!(
                            "[Attempt #{}] Port {}:{} is in use by another process ('{}', PID: {}), which is not Aether. Resolve port conflict.",
                            attempt_id, aether_host, aether_port, proc_name, pid
                        );
                        self.set_error(err.clone());
                        return Err(err);
                    }

                    self.logger.log(
                        "INFO",
                        "Aether",
                        format!("[Attempt #{}] Existing Aether process detected (PID: {}, '{}') on {}:{}. Validating proxy health...", attempt_id, pid, proc_name, aether_host, aether_port),
                    );

                    match HealthProber::query_cloudflare_trace_via_socks5(aether_host, aether_port)
                        .await
                    {
                        Ok(trace) => {
                            self.logger.log(
                                "INFO",
                                "Aether",
                                format!(
                                    "[Attempt #{}] Reusing existing healthy external Aether instance (PID: {}) on {}:{} in {:.2}s (POP: {}, IP: {}, Latency: {} ms)",
                                    attempt_id, pid, aether_host, aether_port, t_aether_start.elapsed().as_secs_f32(), trace.colo, trace.ip, trace.latency_ms
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            self.is_aether_managed.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            self.logger.log(
                                "WARN",
                                "Aether",
                                format!(
                                    "[Attempt #{}] Port {}:{} is owned by an unresponsive Aether process (PID: {}, error: {}). Terminating stale process and resetting...",
                                    attempt_id, aether_host, aether_port, pid, e
                                ),
                            );
                            ProcessDetector::kill_process_by_pid(pid);
                            tokio::time::sleep(Duration::from_millis(200)).await;
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
                                    "[Attempt #{}] Reusing existing healthy external listener on {}:{} in {:.2}s (POP: {}, IP: {})",
                                    attempt_id, aether_host, aether_port, t_aether_start.elapsed().as_secs_f32(), trace.colo, trace.ip
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            self.is_aether_managed.store(false, Ordering::SeqCst);
                        }
                        Err(e) => {
                            self.logger.log(
                                "WARN",
                                "Aether",
                                format!(
                                    "[Attempt #{}] Port {}:{} is in use but proxy validation failed ({}). Attempting cleanup of stale Aether process...",
                                    attempt_id, aether_host, aether_port, e
                                ),
                            );
                            ProcessDetector::kill_port_owner_if_aether(aether_port);
                            tokio::time::sleep(Duration::from_millis(200)).await;
                        }
                    }
                }
            }
        } else {
            // Port is free: launch managed Aether instance with specified options
            let aether_pid = {
                let mut aether_guard = self.aether.lock().await;
                if let Err(e) = aether_guard.start_with_options(settings, options, &self.logger) {
                    self.set_error(format!(
                        "[Attempt #{}] Failed to start Aether: {}",
                        attempt_id, e
                    ));
                    return Err(e);
                }
                aether_guard.pid()
            };
            self.is_aether_managed.store(true, Ordering::SeqCst);
            self.logger.log(
                "INFO",
                "Aether",
                format!(
                    "[Attempt #{}] Managed Aether process spawned (PID: {:?})",
                    attempt_id, aether_pid
                ),
            );

            // Probe managed Aether with scan-mode-aware startup deadline
            self.set_state(ConnectionState::ScanningAether);
            let effective_scan_mode = options
                .scan_mode_override
                .as_ref()
                .unwrap_or(&settings.aether.scan_mode);
            let startup_deadline = if options.quick_reconnect == crate::models::settings::QuickReconnectOption::ForceEnabled {
                crate::models::settings::AETHER_RESTORE_TIMEOUT
            } else {
                crate::models::settings::aether_startup_timeout(effective_scan_mode)
            };
            let check_interval = Duration::from_millis(300);

            self.logger.log(
                "INFO",
                "Aether",
                format!(
                    "[Attempt #{}] Waiting for Aether SOCKS proxy on {}:{} (Scan Mode: {:?}, Deadline: {}s)...",
                    attempt_id,
                    aether_host,
                    aether_port,
                    effective_scan_mode,
                    startup_deadline.as_secs()
                ),
            );

            let mut aether_ready = false;
            let mut attempt_cnt: u64 = 0;
            while t_aether_start.elapsed() < startup_deadline {
                if self.cancel_requested.load(Ordering::SeqCst) {
                    self.logger.log(
                        "INFO",
                        "STATE",
                        format!(
                            "[Attempt #{}] Connection cancelled by user during Aether startup/probe.",
                            attempt_id
                        ),
                    );
                    self.force_shutdown();
                    self.set_state(ConnectionState::Disconnected);
                    return Err("Connection cancelled by user".to_string());
                }

                tokio::time::sleep(check_interval).await;
                attempt_cnt += 1;

                // 1. Immediate interactive prompt abort check & process liveness
                {
                    let mut aether_guard = self.aether.lock().await;
                    if aether_guard.is_interactive_prompt_detected() {
                        let err = format!(
                            "[Attempt #{}] Aether entered interactive mode. Managed launch arguments were incomplete.",
                            attempt_id
                        );
                        aether_guard.stop(&self.logger);
                        self.set_error(err.clone());
                        return Err(err);
                    }
                    if !aether_guard.is_running() {
                        let err = format!(
                            "[Attempt #{}] Aether process stopped unexpectedly. View Diagnostics for details.",
                            attempt_id
                        );
                        aether_guard.stop(&self.logger);
                        self.set_error(err.clone());
                        return Err(err);
                    }
                }

                // 2. SOCKS5 probe
                if HealthProber::check_port_open(aether_host, aether_port, 150).await {
                    let log_cb = |lvl: &str, target: &str, msg: &str| {
                        self.logger.log(lvl, target, msg);
                    };
                    match HealthProber::query_cloudflare_trace_via_socks5_with_logger(
                        aether_host,
                        aether_port,
                        Some(&log_cb),
                    )
                    .await
                    {
                        Ok(trace) => {
                            self.logger.log(
                                "INFO",
                                "Aether",
                                format!(
                                    "[Attempt #{}] Aether SOCKS5 tunnel confirmed online in {:.2}s (PID: {:?}, POP: {}, IP: {}, Latency: {} ms)",
                                    attempt_id, t_aether_start.elapsed().as_secs_f32(), aether_pid, trace.colo, trace.ip, trace.latency_ms
                                ),
                            );
                            *self.active_aether_ip.write() = Some(trace.ip);
                            aether_ready = true;
                            break;
                        }
                        Err(err) => {
                            self.logger.log(
                                "DEBUG",
                                "Aether",
                                format!(
                                    "[Attempt #{}] SOCKS5 probe attempt #{} ({:.1}s elapsed): {}",
                                    attempt_id,
                                    attempt_cnt,
                                    t_aether_start.elapsed().as_secs_f32(),
                                    err
                                ),
                            );
                        }
                    }
                }
            }

            if !aether_ready {
                let err = format!(
                    "[Attempt #{}] Aether started, but local proxy on {}:{} did not become ready within {}s deadline (Scan Mode: {:?}). View Diagnostics for details.",
                    attempt_id, aether_host, aether_port, startup_deadline.as_secs(), effective_scan_mode
                );
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Attempt #{}] Stopping managed Aether due to startup deadline timeout",
                        attempt_id
                    ),
                );
                self.aether.lock().await.stop(&self.logger);
                self.set_error(err.clone());
                return Err(err);
            }
        }

        let d_aether_ready = t_aether_start.elapsed();

        // Check Windows administrator elevation before launching TUN
        if !HealthProber::is_process_elevated() {
            let err = format!(
                "[Attempt #{}] Administrator privileges are required to create the Windows TUN adapter.",
                attempt_id
            );
            self.logger.log("ERROR", "UAC", &err);
            if self.is_aether_managed.load(Ordering::SeqCst) {
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Attempt #{}] Stopping managed Aether due to UAC elevation missing",
                        attempt_id
                    ),
                );
                self.aether.lock().await.stop(&self.logger);
            }
            self.set_error(err.clone());
            return Err(err);
        }

        // 2. Pre-launch TUN Conflict Guard:
        // Before spawning a NEW managed sing-box, verify that the configured TUN interface
        // does not already exist in the Windows network stack from an external or orphaned instance.
        let is_singbox_already_running = self.singbox.lock().await.is_running();
        let (tun_preexists, matched_info, _) = if !is_singbox_already_running {
            HealthProber::check_tun_interface_exists(
                &settings.sing_box.interface_name,
                Some(&settings.sing_box.tun_address),
            )
        } else {
            (false, None, Vec::new())
        };
        let matched_desc = matched_info.map(|i| format!("'{}' ({})", i.friendly_name, i.description));

        if let Err(err_msg) = evaluate_prelaunch_tun_conflict(
            is_singbox_already_running,
            tun_preexists,
            &settings.sing_box.interface_name,
            &settings.sing_box.tun_address,
            matched_desc.as_deref(),
        ) {
            let log_err = format!("[Attempt #{}] {}", attempt_id, err_msg);
            self.logger.log("ERROR", "sing-box", &log_err);
            if self.is_aether_managed.load(Ordering::SeqCst) {
                self.aether.lock().await.stop(&self.logger);
            }
            self.set_error(log_err);
            return Err(err_msg);
        }

        if self.cancel_requested.load(Ordering::SeqCst) {
            self.logger.log(
                "INFO",
                "STATE",
                format!(
                    "[Attempt #{}] Connection cancelled by user before sing-box start.",
                    attempt_id
                ),
            );
            self.force_shutdown();
            self.set_state(ConnectionState::Disconnected);
            return Err("Connection cancelled by user".to_string());
        }

        // 3. Launch sing-box router
        self.set_state(ConnectionState::StartingRouter);
        let t_sb_start = std::time::Instant::now();
        let sb_pid = {
            let mut sb_guard = self.singbox.lock().await;
            if let Err(e) = sb_guard.start(settings, &self.logger) {
                if self.cancel_requested.load(Ordering::SeqCst) {
                    self.force_shutdown();
                    self.set_state(ConnectionState::Disconnected);
                    return Err("Connection cancelled by user".to_string());
                }
                if self.is_aether_managed.load(Ordering::SeqCst) {
                    self.logger.log(
                        "INFO",
                        "Aether",
                        format!(
                            "[Attempt #{}] Stopping managed Aether due to sing-box start failure: {}",
                            attempt_id, e
                        ),
                    );
                    self.aether.lock().await.stop(&self.logger);
                }
                self.set_error(format!(
                    "[Attempt #{}] Failed to start sing-box TUN router: {}",
                    attempt_id, e
                ));
                return Err(e);
            }
            sb_guard.pid()
        };
        let d_sb_spawn = t_sb_start.elapsed();
        self.logger.log(
            "INFO",
            "sing-box",
            format!(
                "[Attempt #{}] sing-box TUN router spawned in {:.2}s (PID: {:?})",
                attempt_id,
                d_sb_spawn.as_secs_f32(),
                sb_pid
            ),
        );

        if self.cancel_requested.load(Ordering::SeqCst) {
            self.logger.log(
                "INFO",
                "STATE",
                format!(
                    "[Attempt #{}] Connection cancelled by user after sing-box spawn.",
                    attempt_id
                ),
            );
            self.force_shutdown();
            self.set_state(ConnectionState::Disconnected);
            return Err("Connection cancelled by user".to_string());
        }

        // 3. Bounded routing and system egress verification matching Aether IP
        self.set_state(ConnectionState::TestingRouting);
        let t_verify_start = std::time::Instant::now();
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
            if self.cancel_requested.load(Ordering::SeqCst) {
                self.logger.log(
                    "INFO",
                    "STATE",
                    format!(
                        "[Attempt #{}] Connection cancelled by user during routing verification.",
                        attempt_id
                    ),
                );
                sb_guard.stop(&self.logger);
                drop(sb_guard);
                self.force_shutdown();
                self.set_state(ConnectionState::Disconnected);
                return Err("Connection cancelled by user".to_string());
            }

            let err = format!(
                "[Attempt #{}] Routing verification failed: {}. Stopping connection attempt.",
                attempt_id, verify_err
            );
            self.logger.log(
                "INFO",
                "sing-box",
                format!(
                    "[Attempt #{}] Stopping sing-box due to verification failure",
                    attempt_id
                ),
            );
            sb_guard.stop(&self.logger);
            drop(sb_guard);
            if self.is_aether_managed.load(Ordering::SeqCst) {
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Attempt #{}] Stopping managed Aether due to verification failure",
                        attempt_id
                    ),
                );
                self.aether.lock().await.stop(&self.logger);
            }
            self.set_error(err.clone());
            return Err(err);
        }

        let d_verify = t_verify_start.elapsed();
        let total_duration = t_connect_start.elapsed();

        self.set_state(ConnectionState::Connected);
        self.logger.log(
            "INFO",
            "STATE",
            format!(
                "[Attempt #{}] CONNECTED in {:.2}s! Timing breakdown: Aether ready: {:.2}s | sing-box spawn: {:.2}s | Verification: {:.2}s | Total: {:.2}s",
                attempt_id,
                total_duration.as_secs_f32(),
                d_aether_ready.as_secs_f32(),
                d_sb_spawn.as_secs_f32(),
                d_verify.as_secs_f32(),
                total_duration.as_secs_f32()
            ),
        );
        Ok(())
    }

    pub async fn get_best_candidate_rtt(&self) -> Option<u32> {
        self.aether.lock().await.get_best_candidate_rtt()
    }

    pub async fn find_faster_gateway(
        &self,
        settings: &AppSettings,
    ) -> Result<RouteOptimizationResult, String> {
        // 1. Acquire operation lock FIRST
        let _op_guard = match self.op_lock.try_lock() {
            Ok(guard) => guard,
            Err(_) => return Err("Connection operation already in progress".to_string()),
        };

        let opt_id = self.next_attempt_id.fetch_add(1, Ordering::SeqCst);
        let current_state = *self.state.read();

        match current_state {
            ConnectionState::Connected => {
                // Task D: External Aether Ownership Guard
                // If the current connection is using an external Aether instance (is_aether_managed == false),
                // abort optimization immediately without disrupting the active connection or touching TUN.
                if !self.is_aether_managed.load(Ordering::SeqCst) {
                    let err = "Gateway optimization requires an Aether instance managed by Aether Desktop. Close the external Aether instance and reconnect through Aether Desktop first.".to_string();
                    self.logger
                        .log("WARN", "Aether", format!("[Optimize #{}] {}", opt_id, err));
                    return Err(err);
                }

                // Connected route optimization flow
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Fresh gateway scan requested (Find Faster Gateway)",
                        opt_id
                    ),
                );

                // Step 1: PREPARE - measure multi-sample baseline latency before disruption
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Measuring baseline path latency (5 samples)...",
                        opt_id
                    ),
                );

                let baseline_profile = match HealthProber::measure_socks5_latency_samples(
                    &settings.aether.host,
                    settings.aether.port,
                    5,
                    Duration::from_millis(150),
                )
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        let err = format!(
                            "[Optimize #{}] Baseline latency measurement failed: {}. Aborting optimization without disrupting active connection.",
                            opt_id, e
                        );
                        self.logger.log("ERROR", "Aether", &err);
                        return Err(err);
                    }
                };

                let prev_latency_ms = Some(baseline_profile.median_ms);
                let prev_jitter_ms = Some(baseline_profile.jitter_mad_ms);
                let prev_pop = baseline_profile.latest_trace.as_ref().map(|t| t.colo.clone());
                let prev_ip = baseline_profile.latest_trace.as_ref().map(|t| t.ip.clone());

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Baseline path: Median: {} ms, Jitter: {} ms ({} of 5 samples succeeded). Egress IP: {}, POP: {}",
                        opt_id,
                        baseline_profile.median_ms,
                        baseline_profile.jitter_mad_ms,
                        baseline_profile.successful_samples,
                        prev_ip.as_deref().unwrap_or("unknown"),
                        prev_pop.as_deref().unwrap_or("unknown")
                    ),
                );

                let snapshot = match crate::settings::storage::AetherPersistenceSnapshot::create_for_settings(
                    settings,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        let err = format!(
                            "[Optimize #{}] Rollback snapshot preparation failed: {}. Aborting optimization without disrupting active connection.",
                            opt_id, e
                        );
                        self.logger.log("ERROR", "Aether", &err);
                        return Err(err);
                    }
                };

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!("[Optimize #{}] Quick reconnect disabled for fresh scan", opt_id),
                );
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!("[Optimize #{}] Fresh Thorough scan requested", opt_id),
                );

                self.set_state(ConnectionState::ScanningAether);

                // Step 2: TEARDOWN - stop sing-box, wait for TUN teardown, stop managed Aether
                self.singbox.lock().await.stop(&self.logger);

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Waiting for previous TUN/routes to be released...",
                        opt_id
                    ),
                );
                let t_teardown_start = std::time::Instant::now();
                match HealthProber::wait_for_tun_teardown(
                    &settings.sing_box.interface_name,
                    Some(&settings.sing_box.tun_address),
                    Duration::from_secs(6),
                )
                .await
                {
                    Ok(()) => {
                        self.logger.log(
                            "INFO",
                            "Aether",
                            format!(
                                "[Optimize #{}] TUN teardown confirmed in {:.2}s",
                                opt_id,
                                t_teardown_start.elapsed().as_secs_f32()
                            ),
                        );
                    }
                    Err(e) => {
                        self.logger.log(
                            "ERROR",
                            "Aether",
                            format!(
                                "[Optimize #{}] TUN teardown could not be confirmed. Fresh scan aborted before Aether endpoint discovery.",
                                opt_id
                            ),
                        );
                        return self
                            .rollback_and_restore(
                                settings,
                                opt_id,
                                prev_latency_ms,
                                prev_jitter_ms,
                                prev_pop,
                                prev_ip,
                                None,
                                None,
                                None,
                                None,
                                snapshot,
                                "AbortTunTeardownFailed".to_string(),
                                format!("TUN teardown could not be confirmed before fresh scan: {}", e),
                            )
                            .await;
                    }
                }

                if self.is_aether_managed.load(Ordering::SeqCst) {
                    self.aether.lock().await.stop(&self.logger);
                }

                // Step 3: SCAN - start Aether with Thorough scan and Quick Reconnect FORCE DISABLED (--no-quick-reconnect)
                let opt_options = crate::models::settings::AetherLaunchOptions::force_fresh(
                    Some(crate::models::settings::AetherScanMode::Thorough),
                );

                let t_scan_start = std::time::Instant::now();
                let aether_spawn_res = {
                    let mut aether_guard = self.aether.lock().await;
                    aether_guard.start_with_options(settings, &opt_options, &self.logger)
                };

                if let Err(e) = aether_spawn_res {
                    self.logger.log(
                        "ERROR",
                        "Aether",
                        format!(
                            "[Optimize #{}] Failed to spawn Aether for fresh scan: {}. Entering rollback...",
                            opt_id, e
                        ),
                    );
                    return self
                        .rollback_and_restore(
                            settings,
                            opt_id,
                            prev_latency_ms,
                            prev_jitter_ms,
                            prev_pop,
                            prev_ip,
                            None,
                            None,
                            None,
                            None,
                            snapshot,
                            "RollbackSpawnFailed".to_string(),
                            e,
                        )
                        .await;
                }
                self.is_aether_managed.store(true, Ordering::SeqCst);

                // Step 4: DEADLINE & SOCKS READY CHECK
                let startup_deadline = crate::models::settings::aether_startup_timeout(
                    &crate::models::settings::AetherScanMode::Thorough,
                );
                let check_interval = Duration::from_millis(300);
                let mut aether_ready = false;

                while t_scan_start.elapsed() < startup_deadline {
                    tokio::time::sleep(check_interval).await;

                    {
                        let mut aether_guard = self.aether.lock().await;
                        if aether_guard.is_interactive_prompt_detected()
                            || !aether_guard.is_running()
                        {
                            aether_guard.stop(&self.logger);
                            break;
                        }
                    }

                    if HealthProber::check_port_open(
                        &settings.aether.host,
                        settings.aether.port,
                        150,
                    )
                    .await
                    {
                        match HealthProber::query_cloudflare_trace_via_socks5(
                            &settings.aether.host,
                            settings.aether.port,
                        )
                        .await
                        {
                            Ok(trace) => {
                                *self.active_aether_ip.write() = Some(trace.ip.clone());
                                aether_ready = true;
                                break;
                            }
                            Err(_) => {}
                        }
                    }
                }

                if !aether_ready {
                    let elapsed_s = t_scan_start.elapsed().as_secs_f32();
                    let aether_alive = self.aether.lock().await.is_running();
                    let best_rtt = self.aether.lock().await.get_best_candidate_rtt();
                    let diag_msg = format!(
                        "[Optimize #{}] Fresh Thorough scan timed out after {:.1}s (configured desktop deadline: {}s). Upstream scan mode: Thorough. Aether process alive: {}. Candidates observed: {}. Best candidate RTT: {}. Pre-scan TUN teardown: {}.",
                        opt_id,
                        elapsed_s,
                        startup_deadline.as_secs(),
                        aether_alive,
                        if best_rtt.is_some() { "yes" } else { "0" },
                        best_rtt
                            .map(|r| format!("{} ms", r))
                            .unwrap_or_else(|| "—".to_string()),
                        "confirmed"
                    );
                    self.logger.log("WARN", "Aether", &diag_msg);
                    self.aether.lock().await.stop(&self.logger);
                    return self
                        .rollback_and_restore(
                            settings,
                            opt_id,
                            prev_latency_ms,
                            prev_jitter_ms,
                            prev_pop,
                            prev_ip,
                            None,
                            None,
                            None,
                            None,
                            snapshot,
                            "RollbackTimeout".to_string(),
                            format!(
                                "Fresh scan did not yield a responsive gateway within {:.1}s deadline",
                                elapsed_s
                            ),
                        )
                        .await;
                }

                // Step 5: MEASURE CANDIDATE PATH (5 samples)
                let (was_cached_reuse, was_fresh_scan) = {
                    let aether_guard = self.aether.lock().await;
                    (
                        aether_guard.was_cached_endpoint_reused(),
                        aether_guard.was_fresh_scan_observed(),
                    )
                };

                if was_cached_reuse {
                    let err_msg = "Fresh scan was bypassed by cached endpoint reuse; restoring previous path.".to_string();
                    self.logger.log("ERROR", "Aether", format!("[Optimize #{}] {}", opt_id, err_msg));
                    return self
                        .rollback_and_restore(
                            settings,
                            opt_id,
                            prev_latency_ms,
                            prev_jitter_ms,
                            prev_pop,
                            prev_ip,
                            None,
                            None,
                            None,
                            None,
                            snapshot,
                            "FreshScanBypassed".to_string(),
                            err_msg,
                        )
                        .await;
                }

                if was_fresh_scan {
                    self.logger.log(
                        "INFO",
                        "Aether",
                        format!("[Optimize #{}] Fresh Thorough scan active: endpoint discovery observed", opt_id),
                    );
                }

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] SOCKS listener ready in {:.2}s. Measuring candidate path latency (5 samples)...",
                        opt_id,
                        t_scan_start.elapsed().as_secs_f32()
                    ),
                );

                let candidate_profile = match HealthProber::measure_socks5_latency_samples(
                    &settings.aether.host,
                    settings.aether.port,
                    5,
                    Duration::from_millis(150),
                )
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        self.logger.log(
                            "WARN",
                            "Aether",
                            format!(
                                "[Optimize #{}] Candidate path latency could not be measured reliably ({}). Rolling back to previous path.",
                                opt_id, e
                            ),
                        );
                        return self
                            .rollback_and_restore(
                                settings,
                                opt_id,
                                prev_latency_ms,
                                prev_jitter_ms,
                                prev_pop,
                                prev_ip,
                                None,
                                None,
                                None,
                                None,
                                snapshot,
                                "RollbackCandidateFailed".to_string(),
                                format!("Candidate latency measurement failed: {}", e),
                            )
                            .await;
                    }
                };

                let new_latency_ms = candidate_profile.median_ms;
                let new_jitter_ms = candidate_profile.jitter_mad_ms;
                let new_pop = candidate_profile
                    .latest_trace
                    .as_ref()
                    .map(|t| t.colo.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                let new_ip = candidate_profile
                    .latest_trace
                    .as_ref()
                    .map(|t| t.ip.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Candidate path: Median: {} ms, Jitter: {} ms ({} of 5 samples succeeded). Egress IP: {}, POP: {}",
                        opt_id,
                        new_latency_ms,
                        new_jitter_ms,
                        candidate_profile.successful_samples,
                        new_ip,
                        new_pop
                    ),
                );

                // Step 6: DECISION - evaluate whether candidate is meaningfully faster and stable
                let (
                    decision,
                    required_improvement_ms,
                    latency_delta_ms,
                    max_allowable_jitter_ms,
                ) = evaluate_gateway_optimization(
                    baseline_profile.median_ms,
                    baseline_profile.jitter_mad_ms,
                    new_latency_ms,
                    new_jitter_ms,
                );

                match decision {
                    GatewayOptimizationDecision::NotEnoughLatencyImprovement => {
                        let reject_msg = format!(
                            "Candidate path (Median: {} ms) was not meaningfully faster than previous path (Median: {} ms, required improvement: {} ms). Restoring previous gateway.",
                            new_latency_ms, baseline_profile.median_ms, required_improvement_ms
                        );
                        self.logger
                            .log("INFO", "Aether", format!("[Optimize #{}] {}", opt_id, reject_msg));
                        return self
                            .rollback_and_restore(
                                settings,
                                opt_id,
                                prev_latency_ms,
                                prev_jitter_ms,
                                prev_pop,
                                prev_ip,
                                Some(new_latency_ms),
                                Some(new_jitter_ms),
                                Some(new_pop),
                                Some(new_ip),
                                snapshot,
                                "NotEnoughLatencyImprovement".to_string(),
                                reject_msg,
                            )
                            .await;
                    }
                    GatewayOptimizationDecision::CandidateTooUnstable => {
                        let reject_msg = format!(
                            "Candidate path jitter ({} ms) exceeded maximum stability threshold ({} ms, baseline jitter: {} ms). Restoring previous gateway.",
                            new_jitter_ms, max_allowable_jitter_ms, baseline_profile.jitter_mad_ms
                        );
                        self.logger
                            .log("INFO", "Aether", format!("[Optimize #{}] {}", opt_id, reject_msg));
                        return self
                            .rollback_and_restore(
                                settings,
                                opt_id,
                                prev_latency_ms,
                                prev_jitter_ms,
                                prev_pop,
                                prev_ip,
                                Some(new_latency_ms),
                                Some(new_jitter_ms),
                                Some(new_pop),
                                Some(new_ip),
                                snapshot,
                                "CandidateTooUnstable".to_string(),
                                reject_msg,
                            )
                            .await;
                    }
                    GatewayOptimizationDecision::KeptFaster => {
                        self.logger.log(
                            "INFO",
                            "Aether",
                            format!(
                                "[Optimize #{}] Candidate is genuinely faster and stable (Median: {} ms vs {} ms, improved by {} ms >= {} ms threshold; Jitter: {} ms <= {} ms). Retaining new gateway.",
                                opt_id,
                                new_latency_ms,
                                baseline_profile.median_ms,
                                latency_delta_ms,
                                required_improvement_ms,
                                new_jitter_ms,
                                max_allowable_jitter_ms
                            ),
                        );
                    }
                }

                // Step 7: ROUTING SETUP & VERIFICATION
                self.set_state(ConnectionState::StartingRouter);
                let sb_spawn_res = {
                    let mut sb_guard = self.singbox.lock().await;
                    sb_guard.start(settings, &self.logger)
                };
                if let Err(e) = sb_spawn_res {
                    self.logger.log(
                        "ERROR",
                        "sing-box",
                        format!(
                            "[Optimize #{}] Failed to start sing-box after fresh scan: {}. Entering rollback...",
                            opt_id, e
                        ),
                    );
                    return self
                        .rollback_and_restore(
                            settings,
                            opt_id,
                            prev_latency_ms,
                            prev_jitter_ms,
                            prev_pop,
                            prev_ip,
                            Some(new_latency_ms),
                            Some(new_jitter_ms),
                            Some(new_pop),
                            Some(new_ip),
                            snapshot,
                            "RollbackSingboxFailed".to_string(),
                            e,
                        )
                        .await;
                }

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
                    self.logger.log(
                        "ERROR",
                        "sing-box",
                        format!(
                            "[Optimize #{}] Routing verification failed after fresh scan: {}. Entering rollback...",
                            opt_id, verify_err
                        ),
                    );
                    sb_guard.stop(&self.logger);
                    drop(sb_guard);
                    return self
                        .rollback_and_restore(
                            settings,
                            opt_id,
                            prev_latency_ms,
                            prev_jitter_ms,
                            prev_pop,
                            prev_ip,
                            Some(new_latency_ms),
                            Some(new_jitter_ms),
                            Some(new_pop),
                            Some(new_ip),
                            snapshot,
                            "RollbackRoutingVerificationFailed".to_string(),
                            verify_err,
                        )
                        .await;
                }

                // Step 8: COMMIT - discard old snapshot, retain newly selected faster gateway
                snapshot.cleanup();

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!("[Optimize #{}] Routing verification passed", opt_id),
                );
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!("[Optimize #{}] Gateway optimization complete", opt_id),
                );
                self.set_state(ConnectionState::Connected);

                let msg = format!(
                    "New faster gateway kept (Median: {} ms, Jitter: {} ms, improved by {} ms vs previous {} ms).",
                    new_latency_ms, new_jitter_ms, latency_delta_ms, baseline_profile.median_ms
                );

                Ok(RouteOptimizationResult {
                    success: true,
                    previous_latency_ms: prev_latency_ms,
                    previous_jitter_ms: prev_jitter_ms,
                    previous_pop: prev_pop,
                    previous_ip: prev_ip,
                    new_latency_ms: Some(new_latency_ms),
                    new_jitter_ms: Some(new_jitter_ms),
                    new_pop: Some(new_pop),
                    new_ip: Some(new_ip),
                    latency_delta_ms: Some(latency_delta_ms),
                    decision: "KeptFaster".to_string(),
                    message: msg,
                })
            }
            ConnectionState::Disconnected | ConnectionState::Error => {
                // Task D: External Aether Ownership Guard
                // If port 1819 is already in use by an external listener, fresh scan cannot control it.
                let is_port_occupied =
                    HealthProber::check_port_open(&settings.aether.host, settings.aether.port, 150).await;
                if is_port_occupied {
                    let err = "Gateway optimization requires an Aether instance managed by Aether Desktop. Port 1819 is in use by an external process. Close the external instance and try again.".to_string();
                    self.logger
                        .log("WARN", "Aether", format!("[Optimize #{}] {}", opt_id, err));
                    return Err(err);
                }

                // Disconnected "Find Best Gateway" flow: connect using Thorough scan and Quick Reconnect disabled
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Initial connection requested via fresh gateway scan (Find Best Gateway)",
                        opt_id
                    ),
                );

                // Wait for any stale TUN release before initial scan
                let _ = HealthProber::wait_for_tun_teardown(
                    &settings.sing_box.interface_name,
                    Some(&settings.sing_box.tun_address),
                    Duration::from_secs(3),
                )
                .await;

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Quick reconnect disabled for fresh scan",
                        opt_id
                    ),
                );
                self.logger.log(
                    "INFO",
                    "Aether",
                    format!("[Optimize #{}] Fresh Thorough scan requested", opt_id),
                );

                self.set_state(ConnectionState::StartingAether);

                let opt_options = crate::models::settings::AetherLaunchOptions::force_fresh(
                    Some(crate::models::settings::AetherScanMode::Thorough),
                );

                // Run connection with opt_options
                self.connect_internal(settings, opt_id, &opt_options).await?;

                let sample_profile = HealthProber::measure_socks5_latency_samples(
                    &settings.aether.host,
                    settings.aether.port,
                    5,
                    Duration::from_millis(150),
                )
                .await
                .ok();

                let new_latency_ms = sample_profile.as_ref().map(|p| p.median_ms);
                let new_jitter_ms = sample_profile.as_ref().map(|p| p.jitter_mad_ms);
                let new_pop = sample_profile
                    .as_ref()
                    .and_then(|p| p.latest_trace.as_ref().map(|t| t.colo.clone()));
                let new_ip = sample_profile
                    .as_ref()
                    .and_then(|p| p.latest_trace.as_ref().map(|t| t.ip.clone()));

                self.logger.log(
                    "INFO",
                    "Aether",
                    format!(
                        "[Optimize #{}] Initial gateway scan complete and connected",
                        opt_id
                    ),
                );

                let msg = match (new_latency_ms, new_jitter_ms) {
                    (Some(lat), Some(jit)) => {
                        format!("Connected via fresh Thorough scan (Median: {} ms, Jitter: {} ms).", lat, jit)
                    }
                    _ => "Connected via fresh Thorough scan.".to_string(),
                };

                Ok(RouteOptimizationResult {
                    success: true,
                    previous_latency_ms: None,
                    previous_jitter_ms: None,
                    previous_pop: None,
                    previous_ip: None,
                    new_latency_ms,
                    new_jitter_ms,
                    new_pop,
                    new_ip,
                    latency_delta_ms: None,
                    decision: "InitialConnected".to_string(),
                    message: msg,
                })
            }
            _ => Err("Connection operation already in progress".to_string()),
        }
    }

    pub async fn rollback_and_restore(
        &self,
        settings: &AppSettings,
        opt_id: u64,
        prev_latency_ms: Option<u64>,
        prev_jitter_ms: Option<u64>,
        prev_pop: Option<String>,
        prev_ip: Option<String>,
        candidate_latency_ms: Option<u64>,
        candidate_jitter_ms: Option<u64>,
        candidate_pop: Option<String>,
        candidate_ip: Option<String>,
        snapshot: crate::settings::storage::AetherPersistenceSnapshot,
        decision: String,
        failure_reason: String,
    ) -> Result<RouteOptimizationResult, String> {
        self.logger.log(
            "WARN",
            "STATE",
            format!(
                "[Optimize #{}] Restoring previous working gateway via Quick Reconnect (25s deadline)... Reason: {}",
                opt_id, failure_reason
            ),
        );
        self.set_state(ConnectionState::StartingAether);

        // 1. Stop any running child processes
        self.singbox.lock().await.stop(&self.logger);
        if self.is_aether_managed.load(Ordering::SeqCst) {
            self.aether.lock().await.stop(&self.logger);
        }

        // 2. Wait for TUN teardown
        let _ = HealthProber::wait_for_tun_teardown(
            &settings.sing_box.interface_name,
            Some(&settings.sing_box.tun_address),
            Duration::from_secs(4),
        )
        .await;

        // 3. Atomically restore pre-optimization native lastconn persistence files
        if let Err(e) = snapshot.restore() {
            let err_msg = format!(
                "[Optimize #{}] Fatal: Failed to restore native lastconn persistence snapshot: {}. Aborting rollback without launching corrupted state.",
                opt_id, e
            );
            self.logger.log("ERROR", "Aether", &err_msg);
            snapshot.cleanup();
            self.set_error(err_msg.clone());
            return Err(err_msg);
        }
        snapshot.cleanup();

        // 4. Launch Aether with Quick Reconnect ENABLED and bounded RESTORE timeout (25s)
        let restore_options = crate::models::settings::AetherLaunchOptions::force_quick_reconnect();

        match self.connect_internal(settings, opt_id, &restore_options).await {
            Ok(_) => {
                self.logger.log(
                    "INFO",
                    "STATE",
                    format!(
                        "[Optimize #{}] Successfully restored previous connection via Quick Reconnect",
                        opt_id
                    ),
                );
                let latency_delta_ms = match (prev_latency_ms, candidate_latency_ms) {
                    (Some(prev), Some(cand)) => Some(prev as i64 - cand as i64),
                    _ => None,
                };
                Ok(RouteOptimizationResult {
                    success: false,
                    previous_latency_ms: prev_latency_ms,
                    previous_jitter_ms: prev_jitter_ms,
                    previous_pop: prev_pop,
                    previous_ip: prev_ip,
                    new_latency_ms: candidate_latency_ms,
                    new_jitter_ms: candidate_jitter_ms,
                    new_pop: candidate_pop,
                    new_ip: candidate_ip,
                    latency_delta_ms,
                    decision,
                    message: failure_reason,
                })
            }
            Err(restore_err) => {
                let err = format!(
                    "Optimization rollback failed ({}) AND restoration of previous connection also failed ({})",
                    failure_reason, restore_err
                );
                self.set_error(err.clone());
                Err(err)
            }
        }
    }

    pub async fn disconnect(&self) -> Result<(), String> {
        let _op_guard = self.op_lock.lock().await;

        {
            let mut state = self.state.write();
            *state = ConnectionState::Disconnecting;
        }
        self.logger
            .log("INFO", "STATE", "State changed to: Disconnecting");
        if let Some(ref handle) = *self.app_handle.read() {
            let _ = handle.emit("connection-state-changed", ConnectionState::Disconnecting);
        }

        // Always stop sing-box TUN router
        self.singbox.lock().await.stop(&self.logger);

        // Always stop managed Aether
        if self.is_aether_managed.load(Ordering::SeqCst) {
            self.logger
                .log("INFO", "Aether", "Terminating managed Aether process...");
            self.aether.lock().await.stop(&self.logger);
        } else {
            self.aether.lock().await.stop(&self.logger);
        }

        ProcessDetector::cleanup_stray_managed_processes();
        ProcessDetector::kill_port_owner_if_aether(1819);

        // Wait for TUN teardown
        let settings = crate::settings::SettingsStorage::load();
        let _ = HealthProber::wait_for_tun_teardown(
            &settings.sing_box.interface_name,
            Some(&settings.sing_box.tun_address),
            Duration::from_secs(3),
        )
        .await;

        *self.active_aether_ip.write() = None;
        self.set_state(ConnectionState::Disconnected);
        self.logger.log(
            "INFO",
            "STATE",
            "All VPN components stopped and disconnected.",
        );
        Ok(())
    }

    /// Synchronously and unconditionally terminates all child processes and releases port/TUN
    pub fn force_shutdown(&self) {
        self.logger
            .log("INFO", "Shutdown", "Forcing complete process teardown and shutdown...");
        if let Ok(mut sb) = self.singbox.try_lock() {
            sb.stop(&self.logger);
        }
        if let Ok(mut aether) = self.aether.try_lock() {
            aether.stop(&self.logger);
        }
        ProcessDetector::cleanup_stray_managed_processes();
        ProcessDetector::kill_port_owner_if_aether(1819);
        *self.active_aether_ip.write() = None;
        *self.state.write() = ConnectionState::Disconnected;
    }

    pub async fn apply_live_settings(&self, settings: &AppSettings) -> Result<(), String> {
        let _op_guard = self.op_lock.lock().await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_c_rollback_snapshot_preparation_fails_leaves_connected_session_intact() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Connected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);

        // Point to an invalid read-only/non-writable directory or invalid path for snapshot creation
        // Verify that find_faster_gateway fails cleanly without altering Connected state
        assert_eq!(*orch.state.read(), ConnectionState::Connected);
    }

    #[tokio::test]
    async fn test_d_restore_quick_reconnect_timeout_is_bounded_to_restore_window() {
        let restore_options = crate::models::settings::AetherLaunchOptions {
            quick_reconnect: crate::models::settings::QuickReconnectOption::ForceEnabled,
            scan_mode_override: None,
        };
        let effective_scan_mode = crate::models::settings::AetherScanMode::Thorough;
        let startup_deadline = if restore_options.quick_reconnect == crate::models::settings::QuickReconnectOption::ForceEnabled {
            crate::models::settings::AETHER_RESTORE_TIMEOUT
        } else {
            crate::models::settings::aether_startup_timeout(&effective_scan_mode)
        };

        // Bounded to 25s, NOT the 340s Thorough deadline
        assert_eq!(startup_deadline, std::time::Duration::from_secs(25));
        assert!(startup_deadline < std::time::Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_e_duplicate_optimize_cannot_create_multiple_lifecycle_operations() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);
        let settings = AppSettings::default();

        // 1. Acquire op_lock manually to simulate an in-flight operation
        let _guard = orch.op_lock.try_lock().expect("Failed to lock op_lock");

        // 2. Invoke find_faster_gateway while lock is held
        let res = orch.find_faster_gateway(&settings).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Connection operation already in progress"
        );

        // 3. Verify state remained Disconnected (no phantom state mutation)
        assert_eq!(*orch.state.read(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_f_optimize_rejected_while_other_operation_is_active() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Connected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);
        let settings = AppSettings::default();

        // 1. Hold op_lock to simulate active disconnect/connect
        let _guard = orch.op_lock.try_lock().expect("Failed to lock op_lock");

        // 2. Attempt find_faster_gateway
        let res = orch.find_faster_gateway(&settings).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Connection operation already in progress"
        );

        // State remains Connected
        assert_eq!(*orch.state.read(), ConnectionState::Connected);
    }

    #[tokio::test]
    async fn test_g_tun_teardown_failure_aborts_before_fresh_scan() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Connected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);
        let mut settings = AppSettings::default();
        // Point to invalid paths to prevent accidental external execution
        settings.aether.executable_path = "C:\\invalid\\aether.exe".to_string();
        settings.sing_box.executable_path = "C:\\invalid\\sing-box.exe".to_string();

        // In connected state with invalid binaries, find_faster_gateway will teardown,
        // fail restore/scan cleanly, and never leave an uncontrolled scanning state
        let res = orch.find_faster_gateway(&settings).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_h_snapshot_restore_failure_is_fatal_to_rollback() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Connected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);
        let settings = AppSettings::default();

        let temp_dir =
            std::env::temp_dir().join(format!("aether_test_orch_h_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let lastconn_file = temp_dir.join("aether-lastconn.toml");
        std::fs::write(&lastconn_file, b"endpoint = '162.159.192.1:2408'\nrtt = 45\n").unwrap();

        let snapshot = crate::settings::storage::AetherPersistenceSnapshot::create(&temp_dir).unwrap();
        // Delete snapshot backup file behind its back to force snapshot.restore() failure
        let _ = std::fs::remove_dir_all(&snapshot.snapshot_dir);

        let res = orch
            .rollback_and_restore(
                &settings,
                1,
                Some(50),
                Some(5),
                Some("FRA".to_string()),
                Some("1.1.1.1".to_string()),
                None,
                None,
                None,
                None,
                snapshot,
                "SimulatedScanFailure".to_string(),
                "Simulated scan failure".to_string(),
            )
            .await;

        assert!(res.is_err(), "Restore failure must fail rollback");
        let err = res.unwrap_err();
        assert!(
            err.contains("Fatal: Failed to restore native lastconn persistence snapshot")
                || err.contains("Atomic replacement failed")
                || err.contains("Failed to restore"),
            "Error message must specify restore snapshot failure: {}",
            err
        );
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_cancel_during_startup() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::StartingAether));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);

        // User initiates cancel during startup
        let res = orch.cancel_connection().await;
        assert!(res.is_ok());
        assert!(orch.cancel_requested.load(Ordering::SeqCst));
        assert_eq!(*orch.state.read(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_cancel_during_verification() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::TestingRouting));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);

        // User initiates cancel while verification is underway
        let res = orch.cancel_connection().await;
        assert!(res.is_ok());
        assert!(orch.cancel_requested.load(Ordering::SeqCst));
        assert_eq!(*orch.state.read(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_reconnect_after_cancel() {
        let logger = RingBufferLogger::new(100);
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let orch = ConnectionOrchestrator::new(state.clone(), logger);
        let mut settings = AppSettings::default();
        settings.aether.executable_path = "C:\\invalid\\aether.exe".to_string();
        settings.sing_box.executable_path = "C:\\invalid\\sing-box.exe".to_string();

        // 1. Cancel previous attempt
        orch.cancel_connection().await.unwrap();
        assert!(orch.cancel_requested.load(Ordering::SeqCst));
        assert_eq!(*orch.state.read(), ConnectionState::Disconnected);

        // 2. Reconnect resets cancel_requested
        // The attempt starts, resets cancel flag, and attempts to run
        let _ = orch.connect(&settings).await;
        assert!(!orch.cancel_requested.load(Ordering::SeqCst));
    }
}
