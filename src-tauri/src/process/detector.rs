use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use sysinfo::{Pid, System};

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

    /// Identifies the owning process PID and process name for a listening TCP port on Windows
    #[cfg(windows)]
    pub fn get_process_for_tcp_port(target_port: u16) -> Option<(u32, String)> {
        use windows_sys::Win32::NetworkManagement::IpHelper::{
            GetExtendedTcpTable, TCP_TABLE_OWNER_PID_ALL,
        };

        let mut size: u32 = 0;
        const AF_INET: u32 = 2;

        // First call gets buffer size
        unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
        }

        if size == 0 {
            return None;
        }

        let mut buffer: Vec<u8> = vec![0; size as usize];
        let ret = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if ret != 0 {
            return None;
        }

        #[repr(C)]
        struct MibTcpRowOwnerPid {
            dw_state: u32,
            dw_local_addr: u32,
            dw_local_port: u32,
            dw_remote_addr: u32,
            dw_remote_port: u32,
            dw_owning_pid: u32,
        }

        let num_entries = unsafe { *(buffer.as_ptr() as *const u32) } as usize;
        let rows_ptr =
            unsafe { buffer.as_ptr().add(std::mem::size_of::<u32>()) as *const MibTcpRowOwnerPid };

        let mut found_pid: Option<u32> = None;

        for i in 0..num_entries {
            let row = unsafe { &*rows_ptr.add(i) };
            // Local port is stored in network byte order in high 16 bits of dw_local_port on Windows
            let port_net = (row.dw_local_port & 0xFFFF) as u16;
            let port_host = u16::from_be(port_net);
            const MIB_TCP_STATE_LISTEN: u32 = 2;

            if port_host == target_port
                && (row.dw_state == MIB_TCP_STATE_LISTEN || row.dw_state == 0)
            {
                found_pid = Some(row.dw_owning_pid);
                break;
            }
        }

        let pid = found_pid?;
        let mut sys = System::new_all();
        sys.refresh_processes(
            sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
            true,
        );

        if let Some(process) = sys.process(Pid::from_u32(pid)) {
            let raw_name = process.name().to_string_lossy().to_string();
            let proc_name = if raw_name.to_lowercase().ends_with(".exe") {
                raw_name
            } else {
                format!("{}.exe", raw_name)
            };
            Some((pid, proc_name))
        } else {
            Some((pid, "unknown.exe".to_string()))
        }
    }

    #[cfg(not(windows))]
    pub fn get_process_for_tcp_port(_target_port: u16) -> Option<(u32, String)> {
        None
    }

    /// Forcefully terminates a process by PID
    #[cfg(windows)]
    pub fn kill_process_by_pid(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                let success = TerminateProcess(handle, 1) != 0;
                CloseHandle(handle);
                success
            } else {
                false
            }
        }
    }

    #[cfg(not(windows))]
    pub fn kill_process_by_pid(pid: u32) -> bool {
        let mut sys = System::new_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
        if let Some(process) = sys.process(Pid::from_u32(pid)) {
            process.kill()
        } else {
            false
        }
    }

    /// Kills the process owning a target TCP port if it is an Aether process
    pub fn kill_port_owner_if_aether(port: u16) -> bool {
        if let Some((pid, proc_name)) = Self::get_process_for_tcp_port(port) {
            let lower = proc_name.to_lowercase();
            if lower.contains("aether") {
                Self::kill_process_by_pid(pid)
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Terminates any stray aether.exe or sing-box.exe processes belonging to AetherDesktop
    pub fn cleanup_stray_managed_processes() {
        let mut sys = System::new_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        for (pid, process) in sys.processes() {
            let name_raw = process.name().to_string_lossy().to_string().to_lowercase();
            let exe_path_raw = process
                .exe()
                .map(|p| p.to_string_lossy().to_string().to_lowercase())
                .unwrap_or_default();

            let is_target_name = name_raw == "aether.exe"
                || name_raw == "aether"
                || name_raw == "sing-box.exe"
                || name_raw == "sing-box";

            let is_managed_path = exe_path_raw.contains("aetherdesktop")
                || exe_path_raw.contains("aether-desktop")
                || exe_path_raw.contains("v1.8.0-dev-udp");

            if is_target_name && (is_managed_path || name_raw.contains("aether") || name_raw.contains("sing-box")) {
                let pid_u32 = pid.as_u32();
                Self::kill_process_by_pid(pid_u32);
            }
        }
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
