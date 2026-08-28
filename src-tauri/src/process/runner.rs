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
            logger.log("INFO", &self.name, format!("Stopping process (PID: {:?})", child.id()));
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

    pub fn start(&mut self, settings: &AppSettings, logger: &RingBufferLogger) -> Result<(), String> {
        let exe_path = &settings.aether.executable_path;
        if !Path::new(exe_path).exists() {
            return Err(format!("Aether executable not found at: {}", exe_path));
        }

        logger.log("INFO", "Aether", format!("Launching Aether from {}", exe_path));

        let mut cmd = Command::new(exe_path);
        cmd.args(&settings.aether.launch_arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn Aether: {}", e))?;
        let pid = child.id();
        logger.log("INFO", "Aether", format!("Aether started with PID: {}", pid));

        let stop_flag = Arc::new(AtomicBool::new(false));

        // Pipe stdout/stderr to logger
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

    pub fn write_config(&self, config: &SingBoxConfig) -> Result<(), String> {
        let parent = self.config_path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = SingBoxConfigGenerator::to_json_string(config)
            .map_err(|e| format!("Failed to serialize sing-box config: {}", e))?;
        std::fs::write(&self.config_path, json)
            .map_err(|e| format!("Failed to write sing-box config file: {}", e))?;
        Ok(())
    }

    pub fn validate_config(&self, singbox_exe: &str) -> Result<(), String> {
        if !Path::new(singbox_exe).exists() {
            return Err(format!("sing-box executable not found at: {}", singbox_exe));
        }

        let mut cmd = Command::new(singbox_exe);
        cmd.arg("check")
            .arg("-c")
            .arg(&self.config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run sing-box check: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("sing-box check failed: {}", err));
        }

        Ok(())
    }

    pub fn start(&mut self, settings: &AppSettings, logger: &RingBufferLogger) -> Result<(), String> {
        let exe_path = &settings.sing_box.executable_path;
        let config = SingBoxConfigGenerator::generate(settings);
        self.write_config(&config)?;

        // Validate configuration first
        if let Err(err) = self.validate_config(exe_path) {
            logger.log("WARN", "sing-box", format!("Config check warning/error: {}", err));
        }

        logger.log("INFO", "sing-box", format!("Launching sing-box TUN router from {}", exe_path));

        let mut cmd = Command::new(exe_path);
        cmd.arg("run")
            .arg("-c")
            .arg(&self.config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn sing-box: {}", e))?;
        let pid = child.id();
        logger.log("INFO", "sing-box", format!("sing-box started with PID: {}", pid));

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

    pub fn restart_transparently(
        &mut self,
        settings: &AppSettings,
        logger: &RingBufferLogger,
    ) -> Result<(), String> {
        logger.log("INFO", "sing-box", "Applying updated routing configuration...");
        self.stop(logger);
        std::thread::sleep(std::time::Duration::from_millis(150));
        self.start(settings, logger)?;
        logger.log("INFO", "sing-box", "Updated routing configuration active");
        Ok(())
    }

    pub fn stop(&mut self, logger: &RingBufferLogger) {
        if let Some(ref mut h) = self.handle {
            h.kill(logger);
        }
        self.handle = None;
    }
}