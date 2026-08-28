use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::singbox::SingBoxConfig;
use crate::models::AppSettings;
use crate::routing::SingBoxConfigGenerator;
use crate::settings::storage::atomic_replace_file;
use crate::settings::SettingsStorage;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn classify_log_level(line: &str, is_stderr: bool) -> &'static str {
    let upper = line.to_uppercase();
    if upper.contains("ERROR")
        || upper.contains("[ERR")
        || upper.contains("FATAL")
        || upper.contains("PANIC")
    {
        "ERROR"
    } else if upper.contains("WARN") || upper.contains("[WRN") {
        "WARN"
    } else if upper.contains("DEBUG") || upper.contains("[DBG") || upper.contains("TRACE") {
        "DEBUG"
    } else if upper.contains("INFO")
        || upper.contains("[INF")
        || upper.contains("AETHER V")
        || upper.contains("WIREGUARD")
        || upper.contains("MASQUE")
        || upper.contains("SCAN")
        || upper.contains("CONNECTED")
        || upper.contains("SOCKS")
    {
        "INFO"
    } else if is_stderr {
        // Standard libraries write normal logs to stderr; default to INFO unless explicitly an error
        "INFO"
    } else {
        "INFO"
    }
}

pub struct ProcessHandle {
    name: String,
    child: Option<Child>,
    stop_flag: Arc<AtomicBool>,
    interactive_prompt_detected: Arc<AtomicBool>,
}

impl ProcessHandle {
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(_)) => {
                    self.child = None;
                    false
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn is_interactive_prompt_detected(&self) -> bool {
        self.interactive_prompt_detected.load(Ordering::SeqCst)
    }

    pub fn kill(&mut self, logger: &RingBufferLogger) {
        if let Some(mut child) = self.child.take() {
            self.stop_flag.store(true, Ordering::SeqCst);
            logger.log(
                "INFO",
                &self.name,
                format!("Stopping process (PID: {:?})", child.id()),
            );
            let _ = child.kill();
            let _ = child.wait();
            logger.log("INFO", &self.name, "Process terminated");
        }
    }
}

pub struct AetherRunner {
    handle: Option<ProcessHandle>,
}

impl AetherRunner {
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut h) = self.handle {
            h.is_running()
        } else {
            false
        }
    }

    pub fn is_interactive_prompt_detected(&self) -> bool {
        if let Some(ref h) = self.handle {
            h.is_interactive_prompt_detected()
        } else {
            false
        }
    }

    pub fn start(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.aether.executable_path;
        if !Path::new(exe_path).exists() {
            return Err(format!("Aether executable not found at: {}", exe_path));
        }

        // Dedicated managed AppData directory for Aether configuration & keys
        let aether_data_dir = SettingsStorage::get_aether_data_dir();
        if !aether_data_dir.exists() {
            let _ = std::fs::create_dir_all(&aether_data_dir);
        }

        let aether_config_path = SettingsStorage::get_aether_config_path();
        let cli_args = settings
            .aether
            .build_cli_arguments(Some(&aether_config_path));

        logger.log(
            "INFO",
            "Aether",
            format!(
                "Launching Aether (Non-interactive Profile) from {} with args: {:?}",
                exe_path, cli_args
            ),
        );

        let mut cmd = Command::new(exe_path);
        cmd.args(&cli_args)
            .current_dir(&aether_data_dir)
            .stdin(Stdio::null()) // Run headless without terminal stdin
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Aether: {}", e))?;
        let pid = child.id();
        logger.log(
            "INFO",
            "Aether",
            format!("Aether started with PID: {}", pid),
        );

        let stop_flag = Arc::new(AtomicBool::new(false));
        let interactive_prompt_detected = Arc::new(AtomicBool::new(false));

        if let Some(stdout) = child.stdout.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            let interactive_clone = interactive_prompt_detected.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            let lower = l.to_lowercase();
                            if lower.contains("protocol:")
                                || lower.contains("[1] masque")
                                || lower.contains("[2] wireguard")
                                || lower.contains("scan mode:")
                                || lower.contains("ip version:")
                            {
                                interactive_clone.store(true, Ordering::SeqCst);
                                log_clone.log("ERROR", "Aether", format!("Interactive prompt detected: '{}'. Managed launch arguments were incomplete.", l.trim()));
                            } else {
                                let lvl = classify_log_level(&l, false);
                                log_clone.log(lvl, "Aether", l);
                            }
                        }
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            let interactive_clone = interactive_prompt_detected.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            let lower = l.to_lowercase();
                            if lower.contains("protocol:")
                                || lower.contains("[1] masque")
                                || lower.contains("[2] wireguard")
                                || lower.contains("scan mode:")
                                || lower.contains("ip version:")
                            {
                                interactive_clone.store(true, Ordering::SeqCst);
                                log_clone.log("ERROR", "Aether", format!("Interactive prompt detected: '{}'. Managed launch arguments were incomplete.", l.trim()));
                            } else {
                                let lvl = classify_log_level(&l, true);
                                log_clone.log(lvl, "Aether", l);
                            }
                        }
                    }
                }
            });
        }

        self.handle = Some(ProcessHandle {
            name: "Aether".to_string(),
            child: Some(child),
            stop_flag,
            interactive_prompt_detected,
        });

        Ok(())
    }

    pub fn stop(&mut self, logger: &RingBufferLogger) {
        if let Some(ref mut h) = self.handle {
            h.kill(logger);
        }
        self.handle = None;
    }
}

pub struct SingBoxRunner {
    handle: Option<ProcessHandle>,
    config_path: PathBuf,
    active_executable_path: Option<String>,
    active_interface_name: Option<String>,
}

impl SingBoxRunner {
    pub fn new() -> Self {
        let config_path = SettingsStorage::get_singbox_config_path();
        Self {
            handle: None,
            config_path,
            active_executable_path: None,
            active_interface_name: None,
        }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut h) = self.handle {
            h.is_running()
        } else {
            false
        }
    }

    pub fn write_config_to_path(&self, config: &SingBoxConfig, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let json = SingBoxConfigGenerator::to_json_string(config)
            .map_err(|e| format!("Failed to serialize sing-box config: {}", e))?;

        let temp_write_path = path.with_extension(format!("tmp.{}.json", Uuid::new_v4()));
        let mut file = std::fs::File::create(&temp_write_path).map_err(|e| {
            format!(
                "Failed to create temporary config write file {:?}: {}",
                temp_write_path, e
            )
        })?;
        use std::io::Write;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write config content: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("Failed to flush config content: {}", e))?;
        drop(file);

        if let Err(e) = atomic_replace_file(&temp_write_path, path) {
            let _ = std::fs::remove_file(&temp_write_path);
            return Err(e);
        }
        Ok(())
    }

    pub fn validate_config_file(&self, singbox_exe: &str, file_path: &Path) -> Result<(), String> {
        if !Path::new(singbox_exe).exists() {
            return Err(format!("sing-box executable not found at: {}", singbox_exe));
        }

        let mut cmd = Command::new(singbox_exe);
        cmd.arg("check")
            .arg("-c")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run sing-box check: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "sing-box configuration check failed: {}",
                err.trim()
            ));
        }

        Ok(())
    }

    pub fn spawn_with_config(
        &mut self,
        singbox_exe: &str,
        config_path: &Path,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        if !Path::new(singbox_exe).exists() {
            return Err(format!("sing-box executable not found at: {}", singbox_exe));
        }
        if !config_path.exists() {
            return Err(format!(
                "sing-box config file not found at: {:?}",
                config_path
            ));
        }

        logger.log(
            "INFO",
            "sing-box",
            format!("Launching sing-box TUN router from {}", singbox_exe),
        );

        let mut cmd = Command::new(singbox_exe);
        cmd.arg("run")
            .arg("-c")
            .arg(config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn sing-box: {}", e))?;
        let pid = child.id();
        logger.log(
            "INFO",
            "sing-box",
            format!("sing-box started with PID: {}", pid),
        );

        let stop_flag = Arc::new(AtomicBool::new(false));
        let interactive_prompt_detected = Arc::new(AtomicBool::new(false));

        if let Some(stdout) = child.stdout.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            let lvl = classify_log_level(&l, false);
                            log_clone.log(lvl, "sing-box", l);
                        }
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        if !l.trim().is_empty() {
                            let lvl = classify_log_level(&l, true);
                            log_clone.log(lvl, "sing-box", l);
                        }
                    }
                }
            });
        }

        self.handle = Some(ProcessHandle {
            name: "sing-box".to_string(),
            child: Some(child),
            stop_flag,
            interactive_prompt_detected,
        });

        Ok(())
    }

    pub fn start(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.sing_box.executable_path;
        let config = SingBoxConfigGenerator::generate(settings);

        let config_path = self.config_path.clone();
        self.write_config_to_path(&config, &config_path)?;
        self.validate_config_file(exe_path, &config_path)?;

        self.spawn_with_config(exe_path, &config_path, logger)?;

        self.active_executable_path = Some(settings.sing_box.executable_path.clone());
        self.active_interface_name = Some(settings.sing_box.interface_name.clone());
        Ok(())
    }

    /// Complete health and routing egress verification helper.
    /// Verifies process liveness, TUN interface presence, and checks that direct system egress
    /// matches the expected Aether public IP (preventing false positive on native ISP egress).
    /// Complete health and routing egress verification helper.
    /// Verifies process liveness, native TUN interface presence (name and/or IP), and checks that direct system egress
    /// matches the expected Aether public IP (preventing false positive on native ISP egress).
    pub async fn verify_router_and_egress(
        &mut self,
        interface_name: &str,
        tun_address: Option<&str>,
        max_duration: Duration,
        expected_aether_ip: Option<&str>,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let mut tun_detected = false;
        let mut matched_adapter_name = String::new();
        let mut last_detected_adapters = Vec::new();

        while start.elapsed() < max_duration {
            if !self.is_running() {
                return Err("sing-box process exited unexpectedly during verification".to_string());
            }

            let (found, matched_info, all_adapters) =
                HealthProber::check_tun_interface_exists(interface_name, tun_address);
            last_detected_adapters = all_adapters;
            if found {
                tun_detected = true;
                if let Some(info) = matched_info {
                    matched_adapter_name = format!(
                        "'{}' (Description: '{}', Index: {}, IP: {:?})",
                        info.friendly_name, info.description, info.if_index, info.ip_addresses
                    );
                } else {
                    matched_adapter_name = interface_name.to_string();
                }
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        if !tun_detected {
            let mut detected_summary = Vec::new();
            for a in &last_detected_adapters {
                let ips = if a.ip_addresses.is_empty() {
                    "none".to_string()
                } else {
                    a.ip_addresses.join(", ")
                };
                detected_summary.push(format!(
                    "  - '{}' ({}) [Index: {}, Up: {}, IPs: {}]",
                    a.friendly_name, a.description, a.if_index, a.is_up, ips
                ));
            }
            let summary_str = if detected_summary.is_empty() {
                "  (No network adapters reported by IP Helper API)".to_string()
            } else {
                detected_summary.join("\n")
            };

            logger.log(
                "WARN",
                "sing-box",
                format!(
                    "TUN network adapter not detected. Expected: name='{}', IP='{}'. Detected adapters in network stack:\n{}",
                    interface_name,
                    tun_address.unwrap_or("none"),
                    summary_str
                ),
            );

            return Err(format!(
                "TUN network adapter '{}' (IP: '{}') not detected in network stack",
                interface_name,
                tun_address.unwrap_or("none")
            ));
        }

        logger.log(
            "INFO",
            "sing-box",
            format!(
                "Verified TUN network adapter presence: {}",
                matched_adapter_name
            ),
        );

        // Direct system egress test through TUN adapter
        let mut system_trace_opt = None;
        let mut last_err = String::new();
        for _ in 1..=4 {
            match HealthProber::query_direct_system_cloudflare_trace().await {
                Ok(trace) => {
                    system_trace_opt = Some(trace);
                    break;
                }
                Err(e) => {
                    last_err = e;
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }
        }

        let system_trace = match system_trace_opt {
            Some(t) => t,
            None => return Err(format!("Direct system egress failed: {}", last_err)),
        };

        // Strict egress consistency check: system traffic must match Aether proxy egress IP
        if let Some(exp_ip) = expected_aether_ip {
            if !system_trace.ip.is_empty() && !exp_ip.is_empty() && system_trace.ip != exp_ip {
                return Err(format!(
                    "System egress IP ({}) does not match Aether egress IP ({}). Traffic is not traversing Aether outbound.",
                    system_trace.ip, exp_ip
                ));
            }
        }

        Ok(())
    }

    /// Transactional Live Apply with full health and system egress verification:
    /// 1. Captures known-good runtime metadata (old_exe, old_interface, old_config).
    /// 2. Generates candidate config to temporary file.
    /// 3. Pre-validates candidate config with sing-box check while old router remains active.
    /// 4. Backs up the known-good config (REQUIRED - aborts if backup creation fails).
    /// 5. Atomically replaces active config and spawns sing-box without regenerating.
    /// 6. Verifies new router process alive + TUN exists + direct system egress matches Aether IP.
    /// 7. If ANY verification fails: restores backup config file atomically, spawns rollback router
    ///    using OLD executable and OLD interface metadata, and verifies rollback instance health.
    pub async fn restart_transparently(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
        expected_aether_ip: Option<&str>,
    ) -> Result<(), String> {
        let old_exe = self
            .active_executable_path
            .clone()
            .unwrap_or_else(|| settings.sing_box.executable_path.clone());
        let old_interface = self
            .active_interface_name
            .clone()
            .unwrap_or_else(|| settings.sing_box.interface_name.clone());

        let candidate_exe = settings.sing_box.executable_path.clone();
        let candidate_interface = settings.sing_box.interface_name.clone();
        let candidate_tun_address = settings.sing_box.tun_address.clone();
        let candidate_config = SingBoxConfigGenerator::generate(settings);

        let config_path = self.config_path.clone();
        let candidate_path =
            config_path.with_extension(format!("candidate.{}.json", Uuid::new_v4()));
        let backup_path = config_path.with_extension(format!("backup.{}.json", Uuid::new_v4()));

        // 1. Write candidate config
        self.write_config_to_path(&candidate_config, &candidate_path)?;

        // 2. Validate candidate config file (old router still running!)
        if let Err(err) = self.validate_config_file(&candidate_exe, &candidate_path) {
            let _ = std::fs::remove_file(&candidate_path);
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "Candidate config invalid: {}. Existing routing unchanged.",
                    err
                ),
            );
            return Err(format!(
                "Configuration check failed: {}. Existing routing kept active.",
                err
            ));
        }

        // 3. Backup currently active known-good config (REQUIRED)
        if config_path.exists() {
            std::fs::copy(&config_path, &backup_path).map_err(|e| {
                let _ = std::fs::remove_file(&candidate_path);
                format!("Unable to preserve known-good routing config (backup creation failed: {}); live apply aborted.", e)
            })?;
        }

        // 4. Atomically replace active config with candidate config
        if let Err(e) = atomic_replace_file(&candidate_path, &config_path) {
            let _ = std::fs::remove_file(&candidate_path);
            let _ = std::fs::remove_file(&backup_path);
            return Err(format!(
                "Failed to atomically replace active config file: {}",
                e
            ));
        }

        // 5. Stop existing router
        logger.log(
            "INFO",
            "sing-box",
            "Applying validated candidate routing configuration...",
        );
        self.stop(logger);
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 6. Spawn sing-box with the new config file (WITHOUT regenerating from settings)
        if let Err(start_err) = self.spawn_with_config(&candidate_exe, &config_path, logger) {
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "Failed to spawn router with candidate config: {}. Initiating rollback...",
                    start_err
                ),
            );
            return self
                .perform_verified_rollback(
                    &old_exe,
                    &backup_path,
                    &old_interface,
                    Some(&candidate_tun_address),
                    expected_aether_ip,
                    logger,
                    &start_err,
                )
                .await;
        }

        // 7. Full live apply health verification: process alive + TUN interface + direct system egress matching Aether IP
        if let Err(verify_err) = self
            .verify_router_and_egress(
                &candidate_interface,
                Some(&candidate_tun_address),
                Duration::from_secs(6),
                expected_aether_ip,
                logger,
            )
            .await
        {
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "New routing candidate failed health verification ({}). Initiating rollback...",
                    verify_err
                ),
            );
            return self
                .perform_verified_rollback(
                    &old_exe,
                    &backup_path,
                    &old_interface,
                    Some(&candidate_tun_address),
                    expected_aether_ip,
                    logger,
                    &verify_err,
                )
                .await;
        }

        // Succeeded: update active runtime metadata and clean up backup
        self.active_executable_path = Some(candidate_exe);
        self.active_interface_name = Some(candidate_interface);
        let _ = std::fs::remove_file(&backup_path);
        logger.log(
            "INFO",
            "sing-box",
            "Updated routing configuration applied and fully verified successfully",
        );
        Ok(())
    }

    async fn perform_verified_rollback(
        &mut self,
        old_exe: &str,
        backup_path: &Path,
        old_interface: &str,
        tun_address: Option<&str>,
        expected_aether_ip: Option<&str>,
        logger: &RingBufferLogger,
        reason: &str,
    ) -> Result<(), String> {
        self.stop(logger);

        let config_path = self.config_path.clone();

        if !backup_path.exists() {
            return Err(format!("CRITICAL: Failed to start new router ({}) and no backup config existed for rollback.", reason));
        }

        logger.log(
            "WARN",
            "sing-box",
            "Restoring previous known-good routing configuration...",
        );
        if let Err(e) = atomic_replace_file(backup_path, &config_path) {
            return Err(format!("CRITICAL: Failed to atomically restore backup config ({}) during rollback after error: {}", e, reason));
        }

        // Spawn rollback instance FROM RESTORED CONFIG FILE using OLD EXECUTABLE
        if let Err(rb_err) = self.spawn_with_config(old_exe, &config_path, logger) {
            logger.log(
                "ERROR",
                "sing-box",
                format!("CRITICAL: Rollback router launch failed: {}", rb_err),
            );
            return Err(format!("CRITICAL: New routing configuration failed ({}) AND automatic rollback launch also failed ({}).", reason, rb_err));
        }

        // Verify rollback instance health using OLD INTERFACE
        if let Err(rb_health_err) = self
            .verify_router_and_egress(
                old_interface,
                tun_address,
                Duration::from_secs(6),
                expected_aether_ip,
                logger,
            )
            .await
        {
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "CRITICAL: Rollback router failed health verification: {}",
                    rb_health_err
                ),
            );
            return Err(format!("CRITICAL: New routing configuration failed ({}) AND rollback router health verification also failed ({}).", reason, rb_health_err));
        }

        // Rollback verified: restore active runtime metadata to OLD
        self.active_executable_path = Some(old_exe.to_string());
        self.active_interface_name = Some(old_interface.to_string());
        let _ = std::fs::remove_file(backup_path);

        logger.log(
            "INFO",
            "sing-box",
            "Rollback to previous known-good routing configuration verified successfully",
        );
        Err(format!("New routing configuration failed ({}); rolled back to previous working configuration successfully.", reason))
    }

    pub fn stop(&mut self, logger: &RingBufferLogger) {
        if let Some(ref mut h) = self.handle {
            h.kill(logger);
        }
        self.handle = None;
    }
}
