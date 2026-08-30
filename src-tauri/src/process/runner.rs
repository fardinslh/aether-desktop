use crate::health::HealthProber;
use crate::logging::RingBufferLogger;
use crate::models::singbox::SingBoxConfig;
use crate::models::AppSettings;
use crate::routing::SingBoxConfigGenerator;
use crate::settings::storage::atomic_replace_file;
use crate::settings::SettingsStorage;
use crate::models::settings::AetherLaunchOptions;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
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

pub fn parse_candidate_rtt_from_line(line: &str) -> Option<u32> {
    let lower = line.to_lowercase();
    let candidate_pos = lower
        .find("rtt")
        .or_else(|| lower.find("latency"))
        .or_else(|| lower.find("ping"))?;

    let after = &lower[candidate_pos..];
    let start_idx = after.find(|c: char| c.is_ascii_digit())?;
    let num_slice = &after[start_idx..];

    let mut digits = String::new();
    for ch in num_slice.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            break;
        }
    }

    digits.parse::<u32>().ok().filter(|&v| v > 0 && v < 10000)
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

    pub fn child_pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
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
    best_candidate_rtt: Arc<AtomicU32>,
}

impl AetherRunner {
    pub fn new() -> Self {
        Self {
            handle: None,
            best_candidate_rtt: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut h) = self.handle {
            h.is_running()
        } else {
            false
        }
    }

    pub fn pid(&self) -> Option<u32> {
        self.handle.as_ref().and_then(|h| h.child_pid())
    }

    pub fn is_interactive_prompt_detected(&self) -> bool {
        if let Some(ref h) = self.handle {
            h.is_interactive_prompt_detected()
        } else {
            false
        }
    }

    pub fn get_best_candidate_rtt(&self) -> Option<u32> {
        let v = self.best_candidate_rtt.load(Ordering::SeqCst);
        if v > 0 {
            Some(v)
        } else {
            None
        }
    }

    pub fn start(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        self.start_with_options(settings, &AetherLaunchOptions::default(), logger)
    }

    pub fn start_with_options(
        &mut self,
        settings: &AppSettings,
        options: &AetherLaunchOptions,
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

        self.best_candidate_rtt.store(0, Ordering::SeqCst);

        let aether_config_path = SettingsStorage::get_aether_config_path();
        let cli_args = settings
            .aether
            .build_cli_arguments_with_options(Some(&aether_config_path), options);

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
        let best_rtt_clone = self.best_candidate_rtt.clone();

        if let Some(stdout) = child.stdout.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            let interactive_clone = interactive_prompt_detected.clone();
            let best_rtt_loop = best_rtt_clone.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        process_aether_line(
                            &l,
                            false,
                            &interactive_clone,
                            &best_rtt_loop,
                            &log_clone,
                        );
                    }
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let log_clone = logger.clone();
            let stop_clone = stop_flag.clone();
            let interactive_clone = interactive_prompt_detected.clone();
            let best_rtt_loop = best_rtt_clone.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if stop_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(l) = line {
                        process_aether_line(
                            &l,
                            true,
                            &interactive_clone,
                            &best_rtt_loop,
                            &log_clone,
                        );
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

/// Polls a freshly detected TUN adapter across a stabilization duration (e.g. 400ms)
/// ensuring that both:
/// 1. The child process remains alive (`is_running_fn() == true`)
/// 2. The expected TUN interface remains present in the network stack (`check_tun_fn() == true`)
///
/// Returns `Ok(())` if both conditions hold continuously throughout the window.
/// Returns `Err(String)` if the child process exits or the adapter disappears.
pub async fn verify_tun_stabilization<R, T>(
    mut is_running_fn: R,
    mut check_tun_fn: T,
    duration: Duration,
    poll_interval: Duration,
) -> Result<(), String>
where
    R: FnMut() -> bool,
    T: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < duration {
        tokio::time::sleep(poll_interval).await;
        if !is_running_fn() {
            return Err("sing-box process exited during TUN stabilization window".to_string());
        }
        if !check_tun_fn() {
            return Err("TUN interface disappeared during stabilization window".to_string());
        }
    }
    Ok(())
}

/// Parameterized router and egress verification decision path.
/// Verifies process liveness at every stage (initial, discovery, stabilization, pre-egress, and post-egress).
pub async fn verify_router_and_egress_decision_path<R, C, S, FutS>(
    mut is_running_fn: R,
    mut check_tun_fn: C,
    mut staged_egress_fn: S,
    interface_name: &str,
    tun_address: Option<&str>,
    max_duration: Duration,
    stabilization_duration: Duration,
    stabilization_interval: Duration,
    log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
) -> Result<String, String>
where
    R: FnMut() -> bool,
    C: FnMut(
        &str,
        Option<&str>,
    ) -> (
        bool,
        Option<crate::health::DetectedAdapterInfo>,
        Vec<crate::health::DetectedAdapterInfo>,
    ),
    S: FnMut() -> FutS,
    FutS: std::future::Future<Output = Result<crate::models::health::CloudflareTrace, String>>,
{
    let start = std::time::Instant::now();
    let mut tun_detected = false;
    let mut matched_adapter_name = String::new();
    let mut last_detected_adapters = Vec::new();

    // 1. Initial liveness check before waiting for adapter discovery
    if !is_running_fn() {
        return Err("sing-box process exited before TUN discovery".to_string());
    }

    while start.elapsed() < max_duration {
        if !is_running_fn() {
            return Err("sing-box process exited during TUN initialization".to_string());
        }

        let (found, matched_info, all_adapters) = check_tun_fn(interface_name, tun_address);
        last_detected_adapters = all_adapters;
        if found {
            // Check liveness immediately upon detecting adapter
            if !is_running_fn() {
                return Err("sing-box process exited during TUN initialization".to_string());
            }

            // Stabilization window
            let stab_res = verify_tun_stabilization(
                &mut is_running_fn,
                || {
                    let (stab_found, _, _) = check_tun_fn(interface_name, tun_address);
                    stab_found
                },
                stabilization_duration,
                stabilization_interval,
            )
            .await;

            if let Err(e) = stab_res {
                if !is_running_fn() {
                    return Err(e);
                }
                // TUN flickered/disappeared, keep waiting
                continue;
            }

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
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    if !tun_detected {
        if !is_running_fn() {
            return Err("sing-box process exited during TUN initialization".to_string());
        }
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

        if let Some(logger) = log_fn {
            logger(
                "WARN",
                "sing-box",
                &format!(
                    "TUN network adapter not detected. Expected: name='{}', IP='{}'. Detected adapters in network stack:\n{}",
                    interface_name,
                    tun_address.unwrap_or("none"),
                    summary_str
                ),
            );
        }

        return Err(format!(
            "TUN network adapter '{}' (IP: '{}') not detected in network stack",
            interface_name,
            tun_address.unwrap_or("none")
        ));
    }

    // Liveness check immediately before staged egress verification
    if !is_running_fn() {
        return Err("sing-box process exited before staged egress verification".to_string());
    }

    if let Some(logger) = log_fn {
        logger(
            "INFO",
            "sing-box",
            &format!(
                "Verified TUN network adapter presence in {:.2}s: {}",
                start.elapsed().as_secs_f32(),
                matched_adapter_name
            ),
        );
    }

    let t_stages_start = std::time::Instant::now();
    // Execute staged egress verification
    staged_egress_fn().await?;

    // Authoritative liveness check AFTER all egress stages pass
    if !is_running_fn() {
        return Err("sing-box process exited during routing verification".to_string());
    }

    if let Some(logger) = log_fn {
        logger(
            "INFO",
            "sing-box",
            &format!(
                "All egress verification stages passed in {:.2}s (Total router ready: {:.2}s)",
                t_stages_start.elapsed().as_secs_f32(),
                start.elapsed().as_secs_f32()
            ),
        );
    }

    Ok(matched_adapter_name)
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

    pub fn pid(&self) -> Option<u32> {
        self.handle.as_ref().and_then(|h| h.child_pid())
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
    /// Verifies process liveness, native TUN interface presence (name and/or IP), stabilization window,
    /// and checks that direct system egress matches the expected Aether public IP (preventing false positive on native ISP egress).
    pub async fn verify_router_and_egress(
        &mut self,
        interface_name: &str,
        tun_address: Option<&str>,
        max_duration: Duration,
        expected_aether_ip: Option<&str>,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let log_cb = |lvl: &str, target: &str, msg: &str| {
            logger.log(lvl, target, msg);
        };

        verify_router_and_egress_decision_path(
            || self.is_running(),
            |name, ip| HealthProber::check_tun_interface_exists(name, ip),
            || {
                HealthProber::verify_staged_egress_decision_path(
                    || HealthProber::query_direct_system_cloudflare_trace_ip_literal(),
                    || HealthProber::test_system_dns_resolution("www.cloudflare.com"),
                    || HealthProber::query_direct_system_cloudflare_trace_hostname(),
                    expected_aether_ip,
                    Some(&log_cb),
                )
            },
            interface_name,
            tun_address,
            max_duration,
            Duration::from_millis(400),
            Duration::from_millis(100),
            Some(&log_cb),
        )
        .await
        .map(|_| ())
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

pub fn process_aether_line(
    line: &str,
    is_stderr: bool,
    interactive_flag: &AtomicBool,
    best_rtt: &AtomicU32,
    logger: &RingBufferLogger,
) {
    if line.trim().is_empty() {
        return;
    }
    let lower = line.to_lowercase();
    if lower.contains("protocol:")
        || lower.contains("[1] masque")
        || lower.contains("[2] wireguard")
        || lower.contains("scan mode:")
        || lower.contains("ip version:")
    {
        interactive_flag.store(true, Ordering::SeqCst);
        logger.log(
            "ERROR",
            "Aether",
            format!(
                "Interactive prompt detected: '{}'. Managed launch arguments were incomplete.",
                line.trim()
            ),
        );
    } else {
        if let Some(rtt) = parse_candidate_rtt_from_line(line) {
            let current_best = best_rtt.load(Ordering::SeqCst);
            if current_best == 0 || rtt < current_best {
                best_rtt.store(rtt, Ordering::SeqCst);
                logger.log(
                    "INFO",
                    "Aether",
                    format!("Best candidate so far: {} ms", rtt),
                );
            }
        }
        let lvl = classify_log_level(line, is_stderr);
        logger.log(lvl, "Aether", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_candidate_rtt_various_formats() {
        assert_eq!(
            parse_candidate_rtt_from_line("[+] candidate 162.159.192.1:2408 OK (rtt: 78ms)"),
            Some(78)
        );
        assert_eq!(
            parse_candidate_rtt_from_line("candidate ok ... rtt=42ms"),
            Some(42)
        );
        assert_eq!(
            parse_candidate_rtt_from_line("Endpoint 162.159.193.10:500 ok (rtt: 65ms)"),
            Some(65)
        );
        assert_eq!(
            parse_candidate_rtt_from_line("Testing endpoint latency: 120ms"),
            Some(120)
        );
        assert_eq!(
            parse_candidate_rtt_from_line("Random log line with no timing"),
            None
        );
    }

    #[test]
    fn test_process_aether_line_updates_best_candidate_from_both_stdout_and_stderr() {
        let logger = RingBufferLogger::new(50);
        let interactive = AtomicBool::new(false);
        let best_rtt = AtomicU32::new(0);

        // 1. Candidate line on stdout (is_stderr = false)
        process_aether_line(
            "candidate ok 162.159.192.1:2408 rtt=85ms",
            false,
            &interactive,
            &best_rtt,
            &logger,
        );
        assert_eq!(best_rtt.load(Ordering::SeqCst), 85);
        assert!(!interactive.load(Ordering::SeqCst));

        // 2. Faster candidate line on stderr (is_stderr = true)
        process_aether_line(
            "[+] candidate 162.159.192.5:2408 OK (rtt: 42ms)",
            true,
            &interactive,
            &best_rtt,
            &logger,
        );
        assert_eq!(best_rtt.load(Ordering::SeqCst), 42);

        // 3. Slower candidate on stderr does not overwrite faster
        process_aether_line(
            "Endpoint 162.159.193.10:500 ok (rtt: 65ms)",
            true,
            &interactive,
            &best_rtt,
            &logger,
        );
        assert_eq!(best_rtt.load(Ordering::SeqCst), 42);
    }
}
