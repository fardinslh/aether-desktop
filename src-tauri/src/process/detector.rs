use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunningProcessInfo {
    pub name: String,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub pid: u32,
    pub icon_base64: Option<String>,
}

pub struct ProcessDetector;

impl ProcessDetector {
    /// Discovers user-facing running desktop processes (filtering out noisy Windows background services)
    pub fn get_running_gui_applications() -> Vec<RunningProcessInfo> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // System background filters to exclude from user list
        let system_exclusions: HashSet<&str> = [
            "svchost.exe",
            "smss.exe",
            "csrss.exe",
            "wininit.exe",
            "services.exe",
            "lsass.exe",
            "fontdrvhost.exe",
            "winlogon.exe",
            "dwm.exe",
            "sihost.exe",
            "taskhostw.exe",
            "explorer.exe",
            "shellexperiencehost.exe",
            "searchhost.exe",
            "startmenuexperiencehost.exe",
            "textinputhost.exe",
            "ctfmon.exe",
            "runtimebroker.exe",
            "dllhost.exe",
            "conhost.exe",
            "audiodg.exe",
            "spoolsv.exe",
            "wlanext.exe",
            "wdfmgr.exe",
            "dashost.exe",
            "securityhealthservice.exe",
            "smartscreen.exe",
            "compattelrunner.exe",
            "sedsvc.exe",
            "searchindexer.exe",
            "system",
            "idle",
            "registry",
            "memory compression",
            "aether.exe",
            "sing-box.exe",
            "xray.exe",
            "v2ray.exe",
        ]
        .iter()
        .cloned()
        .collect();

        let mut seen_process_names = HashSet::new();
        let mut results = Vec::new();

        for (pid, process) in sys.processes() {
            let proc_name_raw = process.name().to_string_lossy().to_string();
            let proc_name_lower = proc_name_raw.to_lowercase();

            let proc_name = if proc_name_lower.ends_with(".exe") {
                proc_name_raw.clone()
            } else {
                format!("{}.exe", proc_name_raw)
            };

            let proc_lower = proc_name.to_lowercase();
            if system_exclusions.contains(proc_lower.as_str()) {
                continue;
            }

            if seen_process_names.contains(&proc_lower) {
                continue;
            }
            seen_process_names.insert(proc_lower);

            let exe_path = process.exe().map(|p| p.to_string_lossy().to_string());
            let friendly_name = Self::get_friendly_name(&proc_name);

            results.push(RunningProcessInfo {
                name: friendly_name,
                process_name: proc_name,
                executable_path: exe_path,
                pid: pid.as_u32(),
                icon_base64: None,
            });
        }

        results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        results
    }

    /// Derives metadata from a file path when browsing for an .exe
    pub fn inspect_executable(file_path: &str) -> (String, String) {
        let path = Path::new(file_path);
        let process_name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown.exe".to_string());

        let friendly_name = Self::get_friendly_name(&process_name);
        (friendly_name, process_name)
    }

    pub fn get_friendly_name(process_name: &str) -> String {
        let lower = process_name.to_lowercase();
        match lower.as_str() {
            "chrome.exe" => "Google Chrome".to_string(),
            "code.exe" => "Visual Studio Code".to_string(),
            "codex.exe" => "Codex".to_string(),
            "antigravity.exe" => "Antigravity".to_string(),
            "agy.exe" => "Antigravity Backend (agy)".to_string(),
            "language_server.exe" => "Language Server".to_string(),
            "discord.exe" => "Discord".to_string(),
            "steam.exe" => "Steam".to_string(),
            "steamwebhelper.exe" => "Steam Web Helper".to_string(),
            "dota2.exe" => "Dota 2".to_string(),
            "rustclient.exe" => "Rust Client".to_string(),
            "rust.exe" => "Rust".to_string(),
            "firefox.exe" => "Mozilla Firefox".to_string(),
            "msedge.exe" => "Microsoft Edge".to_string(),
            "spotify.exe" => "Spotify".to_string(),
            "telegram.exe" => "Telegram".to_string(),
            "slack.exe" => "Slack".to_string(),
            "notion.exe" => "Notion".to_string(),
            "obs64.exe" | "obs32.exe" => "OBS Studio".to_string(),
            "generals.exe" => "C&C Generals".to_string(),
            "battle.net.exe" => "Battle.net".to_string(),
            "epicgameslauncher.exe" => "Epic Games Launcher".to_string(),
            "vlc.exe" => "VLC Media Player".to_string(),
            "devenv.exe" => "Visual Studio".to_string(),
            "cursor.exe" => "Cursor AI Editor".to_string(),
            "postman.exe" => "Postman".to_string(),
            _ => {
                let stripped = process_name.trim_end_matches(".exe");
                let mut chars = stripped.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            }
        }
    }
}
