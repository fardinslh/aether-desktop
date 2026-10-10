//! Bounded route selection. Only Aether is replaced during recovery; the TUN stays owned.
use super::orchestrator::ConnectionOrchestrator;
use crate::{models::{AppSettings, ConnectionState, profile::*}, health::HealthProber, settings::SettingsStorage};
use crate::models::settings::{AetherLaunchOptions, AetherScanMode, AetherProtocol};
use std::{sync::atomic::Ordering, time::{Duration, Instant}};
use sha2::{Digest, Sha256};
use tauri::Emitter;

fn network_key() -> Option<String> {
    #[cfg(target_os="macos")]
    let bytes = std::process::Command::new("/sbin/route").args(["-n","get","default"]).output().ok()?.stdout;
    #[cfg(windows)]
    let bytes = std::process::Command::new("ipconfig").output().ok()?.stdout;
    #[cfg(not(any(windows,target_os="macos")))]
    let bytes = std::process::Command::new("ip").args(["route","show","default"]).output().ok()?.stdout;
    if bytes.is_empty() { return None; }
    let text=String::from_utf8_lossy(&bytes);
    #[cfg(target_os="macos")]
    let stable={
        let fields=text.lines().filter_map(|l|l.trim().split_once(':')).collect::<std::collections::HashMap<_,_>>();
        let gateway=fields.get("gateway").map(|s|s.trim()).unwrap_or("");
        let interface=fields.get("interface").map(|s|s.trim()).unwrap_or("");
        let arp=std::process::Command::new("/usr/sbin/arp").args(["-n",gateway]).output().ok().map(|o|String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
        let mac=arp.split_whitespace().skip_while(|v|*v!="at").nth(1).unwrap_or("");
        format!("{}:{}:{}",interface,gateway,mac)
    };
    #[cfg(not(target_os="macos"))]
    let stable=text.to_string();
    Some(format!("{:x}", Sha256::digest(stable.as_bytes())))
}
fn cache_path() -> Option<std::path::PathBuf> { Some(SettingsStorage::get_aether_data_dir().join(format!("route-{}.json",network_key()?))) }

impl ConnectionOrchestrator {
    pub fn publish_details(&self, profile: Option<ConnectionProfile>, phase: &str, reason: Option<String>) {
        let details = ConnectionDetails { active_profile: profile, supports_udp: profile.map(|p| p.supports_udp()).unwrap_or(false), phase: phase.into(), failure_reason: reason };
        *self.connection_details.write() = details.clone();
        if let Some(app) = self.app_handle.read().as_ref() { let _ = app.emit("connection-details-changed",details); }
    }
    pub async fn connect_adaptive(&self, settings: &AppSettings) -> Result<(),String> {
        let mut active = settings.clone();
        let aether_path=active.aether.executable_path.clone();let router_path=active.sing_box.executable_path.clone();
        let (aether,router)=tokio::task::spawn_blocking(move || (
            crate::dependencies::DependencyManager::discover_aether_binary(&aether_path),
            crate::dependencies::DependencyManager::discover_singbox_binary(&router_path)
        )).await.map_err(|e|e.to_string())?;
        active.aether.executable_path=aether.ok_or("Bundled Aether missing or checksum invalid; repair the installation")?.0.to_string_lossy().into();
        active.sing_box.executable_path=router.ok_or("Bundled sing-box missing or checksum invalid; repair the installation")?.0.to_string_lossy().into();
        #[cfg(target_os="macos")]
        { active.sing_box.interface_name = "utun99".into(); }
        let cache = cache_path();
        let cached = cache.as_ref().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice::<ConnectionProfile>(&b).ok());
        let attempts = match active.aether.connection_mode {
            ConnectionMode::Auto => auto_attempts(cached),
            ConnectionMode::EmergencyTor => vec![(ConnectionProfile::Tor,420,false)],
            ConnectionMode::Manual => { let p = active.aether.manual_profile.unwrap_or(match active.aether.protocol { AetherProtocol::Wireguard => ConnectionProfile::Wireguard, AetherProtocol::WarpInWarp => ConnectionProfile::GoolClassic, AetherProtocol::Masque => if active.aether.masque_http2 {ConnectionProfile::MasqueH2} else {ConnectionProfile::MasqueH3} }); vec![(p,crate::models::settings::aether_startup_timeout(&active.aether.scan_mode).as_secs().max(p.timeout_secs()),active.aether.quick_reconnect)] }
        };
        let mut failures = Vec::new();
        for (profile, seconds, quick) in attempts {
            if self.cancel_requested.load(Ordering::SeqCst) { self.force_shutdown(); return Err("Connection cancelled".into()); }
            self.aether.lock().await.stop(&self.logger);
            active.aether.manual_profile = Some(profile);
            self.publish_details(Some(profile),"connecting",None);
            self.set_state(ConnectionState::ScanningAether);
            let options = if quick { AetherLaunchOptions::force_quick_reconnect() } else { AetherLaunchOptions::force_fresh(if active.aether.connection_mode == ConnectionMode::Auto {Some(AetherScanMode::Turbo)} else {None}) };
            let started = self.aether.lock().await.start_with_options(&active,&options,&self.logger);
            self.is_aether_managed.store(true,Ordering::SeqCst);
            let result = match started {
                Err(e) => Err(e),
                Ok(()) => tokio::time::timeout(Duration::from_secs(seconds), async {
                    loop {
                        if self.cancel_requested.load(Ordering::SeqCst) { return Err("Connection cancelled".into()); }
                        if !self.aether.lock().await.is_running() { return Err("Aether process exited".into()); }
                        if HealthProber::check_port_open(&active.aether.host,active.aether.port,150).await {
                            if let Ok(trace) = HealthProber::query_cloudflare_trace_via_socks5(&active.aether.host,active.aether.port).await {
                                if (!active.aether.prevent_iran_exit || trace.loc.len() == 2 && !trace.loc.eq_ignore_ascii_case("IR")) && verify_independent_https(&active.aether.host,active.aether.port).await.is_ok() { return Ok(trace.ip); }
                            }
                        }
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }).await.unwrap_or_else(|_| Err(format!("{} exceeded {}s",profile.id(),seconds)))
            };
            match result {
                Ok(ip) => {
                    *self.active_aether_ip.write() = Some(ip.clone());
                    if !self.singbox.lock().await.is_running() {
                        self.set_state(ConnectionState::StartingRouter);
                        if let Err(e) = self.singbox.lock().await.start(&active,&self.logger) { self.aether.lock().await.stop(&self.logger); self.set_error(e.clone()); return Err(e); }
                    }
                    let verified = self.singbox.lock().await.verify_router_and_egress(&active.sing_box.interface_name,Some(&active.sing_box.tun_address),Duration::from_secs(10),Some(&ip),&self.logger).await;
                    if let Err(e) = verified { failures.push(format!("{}: {}",profile.id(),e)); continue; }
                    if let Some(p) = &cache { let _ = std::fs::write(p,serde_json::to_vec(&profile).unwrap()); }
                    *self.last_error.write() = None; self.health_window.lock().0 = 0;
                    self.publish_details(Some(profile),if profile.supports_udp() {"connected"} else {"tcp_only"},None);
                    self.recovery_allowed.store(true,Ordering::SeqCst);
                    self.set_state(ConnectionState::Connected); return Ok(());
                }
                Err(e) => { self.logger.log("WARN","Auto",&e); self.publish_details(Some(profile),"failed",Some(e.clone())); failures.push(e); }
            }
        }
        self.aether.lock().await.stop(&self.logger);
        let e=failures.join("; ");
        // A running TUN deliberately remains active on failure and rejects protected traffic.
        self.set_error(e.clone()); Err(e)
    }
}
/// A second independent TLS service must return its documented success response.
pub async fn verify_independent_https(host:&str,port:u16) -> Result<(),String> {
    let client=reqwest::Client::builder().proxy(reqwest::Proxy::all(format!("socks5h://{}:{}",host,port)).map_err(|e|e.to_string())?).timeout(Duration::from_secs(6)).redirect(reqwest::redirect::Policy::none()).build().map_err(|e|e.to_string())?;
    let start=Instant::now();
    let response=client.get("https://www.gstatic.com/generate_204").send().await.map_err(|e|e.to_string())?;
    if response.status()!=reqwest::StatusCode::NO_CONTENT { return Err(format!("Independent HTTPS probe: {}",response.status())); }
    let _=start; Ok(())
}
