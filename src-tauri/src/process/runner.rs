use crate::logging::RingBufferLogger;
use crate::models::singbox::SingBoxConfig;
use crate::models::AppSettings;
use crate::routing::SingBoxConfigGenerator;
use crate::settings::SettingsStorage;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ProcessHandle {
    name: String,
    child: Option<Child>,
    stop_flag: Arc<AtomicBool>,
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

    pub fn start(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.aether.executable_path;
        if !Path::new(exe_path).exists() {
            return Err(format!("Aether executable not found at: {}", exe_path));
        }

        logger.log(
            "INFO",
            "Aether",
            format!("Launching Aether from {}", exe_path),
        );

        let mut cmd = Command::new(exe_path);
        cmd.args(&settings.aether.launch_arguments)
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
                            log_clone.log("DEBUG", "Aether", l);
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
                            log_clone.log("WARN", "Aether", l);
                        }
                    }
                }
            });
        }

        self.handle = Some(ProcessHandle {
            name: "Aether".to_string(),
            child: Some(child),
            stop_flag,
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
}

impl SingBoxRunner {
    pub fn new() -> Self {
        let config_path = SettingsStorage::get_singbox_config_path();
        Self {
            handle: None,
            config_path,
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
        std::fs::write(path, json)
            .map_err(|e| format!("Failed to write sing-box config file to {:?}: {}", path, e))?;
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

    /// Spawns sing-box strictly using an already-validated configuration file on disk.
    /// This function MUST NOT regenerate or overwrite the config file.
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
                            log_clone.log("INFO", "sing-box", l);
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
                            log_clone.log("WARN", "sing-box", l);
                        }
                    }
                }
            });
        }

        self.handle = Some(ProcessHandle {
            name: "sing-box".to_string(),
            child: Some(child),
            stop_flag,
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

        self.spawn_with_config(exe_path, &config_path, logger)
    }

    pub fn restart_transparently(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.sing_box.executable_path;
        let candidate_config = SingBoxConfigGenerator::generate(settings);

        let config_path = self.config_path.clone();
        let candidate_path = config_path.with_extension("candidate.json");
        let backup_path = config_path.with_extension("backup.json");

        self.write_config_to_path(&candidate_config, &candidate_path)?;

        if let Err(err) = self.validate_config_file(exe_path, &candidate_path) {
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

        if config_path.exists() {
            let _ = std::fs::copy(&config_path, &backup_path);
        }

        if let Err(e) = std::fs::rename(&candidate_path, &config_path) {
            if std::fs::copy(&candidate_path, &config_path).is_err() {
                let _ = std::fs::remove_file(&candidate_path);
                return Err(format!("Failed to update config file: {}", e));
            }
            let _ = std::fs::remove_file(&candidate_path);
        }

        logger.log(
            "INFO",
            "sing-box",
            "Applying validated candidate routing configuration...",
        );
        self.stop(logger);
        thread::sleep(Duration::from_millis(150));

        if let Err(start_err) = self.spawn_with_config(exe_path, &config_path, logger) {
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "Failed to start router with candidate config: {}. Initiating rollback...",
                    start_err
                ),
            );
            return self.perform_rollback(exe_path, &backup_path, logger, &start_err);
        }

        thread::sleep(Duration::from_millis(400));
        if !self.is_running() {
            logger.log(
                "ERROR",
                "sing-box",
                "New router process exited unexpectedly on launch. Initiating rollback...",
            );
            return self.perform_rollback(
                exe_path,
                &backup_path,
                logger,
                "Process exited unexpectedly",
            );
        }

        let _ = std::fs::remove_file(&backup_path);
        logger.log(
            "INFO",
            "sing-box",
            "Updated routing configuration applied and active successfully",
        );
        Ok(())
    }

    fn perform_rollback(
        &mut self,
        exe_path: &str,
        backup_path: &Path,
        logger: &RingBufferLogger,
        reason: &str,
    ) -> Result<(), String> {
        self.stop(logger);

        let config_path = self.config_path.clone();

        if !backup_path.exists() {
            return Err(format!(
                "Failed to start new router ({}) and no backup config existed for rollback.",
                reason
            ));
        }

        logger.log(
            "WARN",
            "sing-box",
            "Restoring previous known-good routing configuration...",
        );
        if let Err(e) = std::fs::copy(backup_path, &config_path) {
            return Err(format!(
                "CRITICAL: Failed to copy backup config ({}) during rollback after error: {}",
                e, reason
            ));
        }

        if let Err(rb_err) = self.spawn_with_config(exe_path, &config_path, logger) {
            logger.log(
                "ERROR",
                "sing-box",
                format!("CRITICAL: Rollback router launch failed: {}", rb_err),
            );
            return Err(format!("CRITICAL: New routing configuration failed ({}) and automatic rollback also failed ({}).", reason, rb_err));
        }

        thread::sleep(Duration::from_millis(400));
        if !self.is_running() {
            logger.log(
                "ERROR",
                "sing-box",
                "CRITICAL: Rollback router exited unexpectedly",
            );
            return Err(format!("CRITICAL: New routing configuration failed ({}) and rollback router exited unexpectedly.", reason));
        }

        logger.log(
            "INFO",
            "sing-box",
            "Rollback to previous known-good routing configuration succeeded",
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
