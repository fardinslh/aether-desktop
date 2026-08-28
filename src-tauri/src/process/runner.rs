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

        // Pipe stdout to logger
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

        // Pipe stderr to logger
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

    pub fn start(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.sing_box.executable_path;
        let config = SingBoxConfigGenerator::generate(settings);

        // Write to active config path
        self.write_config_to_path(&config, &self.config_path)?;

        // Pre-flight check MUST succeed; return Err if invalid
        self.validate_config_file(exe_path, &self.config_path)?;

        logger.log(
            "INFO",
            "sing-box",
            format!("Launching sing-box TUN router from {}", exe_path),
        );

        let mut cmd = Command::new(exe_path);
        cmd.arg("run")
            .arg("-c")
            .arg(&self.config_path)
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

    /// Safe Live Apply with Rollback:
    /// Validates updated config against a temp file while leaving the working router running.
    /// Rolls back to previous backup if the new router fails to start.
    pub fn restart_transparently(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        let exe_path = &settings.sing_box.executable_path;
        let new_config = SingBoxConfigGenerator::generate(settings);

        let temp_config_path = self.config_path.with_extension("tmp.json");
        let backup_config_path = self.config_path.with_extension("bak.json");

        // 1. Write new config to temp file
        self.write_config_to_path(&new_config, &temp_config_path)?;

        // 2. Validate temp file with real sing-box binary
        if let Err(err) = self.validate_config_file(exe_path, &temp_config_path) {
            let _ = std::fs::remove_file(&temp_config_path);
            logger.log(
                "ERROR",
                "sing-box",
                format!("Live apply aborted: invalid configuration ({})", err),
            );
            return Err(format!(
                "Configuration check failed: {}. Existing routing kept active.",
                err
            ));
        }

        // 3. Backup currently working config if exists
        if self.config_path.exists() {
            let _ = std::fs::copy(&self.config_path, &backup_config_path);
        }

        // 4. Overwrite active config file
        if let Err(e) = std::fs::rename(&temp_config_path, &self.config_path) {
            // If rename fails (e.g. cross-volume), try copy + remove
            if std::fs::copy(&temp_config_path, &self.config_path).is_err() {
                let _ = std::fs::remove_file(&temp_config_path);
                return Err(format!("Failed to update config file: {}", e));
            }
            let _ = std::fs::remove_file(&temp_config_path);
        }

        // 5. Restart sing-box with validated config
        logger.log(
            "INFO",
            "sing-box",
            "Applying validated routing configuration...",
        );
        self.stop(logger);
        thread::sleep(Duration::from_millis(200));

        if let Err(start_err) = self.start(settings, logger) {
            logger.log(
                "ERROR",
                "sing-box",
                format!(
                    "Failed to start router with new configuration: {}",
                    start_err
                ),
            );

            // 6. Rollback to backup config
            if backup_config_path.exists() {
                logger.log(
                    "WARN",
                    "sing-box",
                    "Rolling back to previous working configuration...",
                );
                let _ = std::fs::copy(&backup_config_path, &self.config_path);
                let _ = self.start(settings, logger);
            }
            return Err(format!(
                "Failed to start router: {}. Rolled back to previous configuration.",
                start_err
            ));
        }

        // Verify router stayed alive
        thread::sleep(Duration::from_millis(300));
        if !self.is_running() {
            logger.log(
                "ERROR",
                "sing-box",
                "New router instance exited unexpectedly. Rolling back...",
            );
            if backup_config_path.exists() {
                let _ = std::fs::copy(&backup_config_path, &self.config_path);
                let _ = self.start(settings, logger);
            }
            return Err("Router process exited unexpectedly on restart. Rolled back.".to_string());
        }

        let _ = std::fs::remove_file(&backup_config_path);
        logger.log(
            "INFO",
            "sing-box",
            "Updated routing configuration active successfully",
        );
        Ok(())
    }

    pub fn stop(&mut self, logger: &RingBufferLogger) {
        if let Some(ref mut h) = self.handle {
            h.kill(logger);
        }
        self.handle = None;
    }
}
