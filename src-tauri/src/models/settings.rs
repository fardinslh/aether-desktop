use super::app_rule::{ApplicationRule, RouteDestination};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub aether: AetherSettings,
    pub secondary_proxy: SecondaryProxySettings,
    pub sing_box: SingBoxSettings,
    pub compatibility: CompatibilitySettings,
    pub general: GeneralSettings,
    pub application_rules: Vec<ApplicationRule>,
    pub first_run_completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AetherProtocol {
    Wireguard,
    Masque,
    WarpInWarp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AetherIpMode {
    Ipv4,
    Ipv6,
    Dual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AetherScanMode {
    Turbo,
    Balanced,
    Thorough,
    Stealth,
    Ironclad,
}

fn default_protocol() -> AetherProtocol {
    AetherProtocol::Wireguard
}

fn default_ip_mode() -> AetherIpMode {
    AetherIpMode::Ipv4
}

fn default_scan_mode() -> AetherScanMode {
    AetherScanMode::Thorough
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AetherSettings {
    pub executable_path: String,
    pub host: String,
    pub port: u16,
    #[serde(default = "default_protocol")]
    pub protocol: AetherProtocol,
    #[serde(default = "default_ip_mode")]
    pub ip_mode: AetherIpMode,
    #[serde(default = "default_scan_mode")]
    pub scan_mode: AetherScanMode,
    #[serde(default = "default_true")]
    pub quick_reconnect: bool,
    #[serde(default)]
    pub additional_arguments: Vec<String>,
    #[serde(default)]
    pub launch_arguments: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AetherLaunchOptions {
    pub quick_reconnect_override: Option<bool>,
    pub scan_mode_override: Option<AetherScanMode>,
}

impl AetherSettings {
    pub fn build_cli_arguments(&self, aether_config_path: Option<&Path>) -> Vec<String> {
        self.build_cli_arguments_with_options(aether_config_path, &AetherLaunchOptions::default())
    }

    pub fn build_cli_arguments_with_options(
        &self,
        aether_config_path: Option<&Path>,
        options: &AetherLaunchOptions,
    ) -> Vec<String> {
        let mut args = Vec::new();

        // 1. Explicit managed config path
        if let Some(config_path) = aether_config_path {
            args.push("--config".to_string());
            args.push(config_path.to_string_lossy().to_string());
        }

        // 2. Explicit SOCKS bind
        args.push("--bind".to_string());
        args.push(format!("{}:{}", self.host, self.port));

        // 3. Protocol
        match self.protocol {
            AetherProtocol::Wireguard => args.push("--wg".to_string()),
            AetherProtocol::Masque => args.push("--masque".to_string()),
            AetherProtocol::WarpInWarp => args.push("--gool".to_string()),
        }

        // 4. IP Mode
        match self.ip_mode {
            AetherIpMode::Ipv4 => args.push("-4".to_string()),
            AetherIpMode::Ipv6 => args.push("-6".to_string()),
            AetherIpMode::Dual => args.push("--dual".to_string()),
        }

        // 5. Scan Mode (override if specified for optimization, without changing persistent settings)
        let effective_scan_mode = options
            .scan_mode_override
            .as_ref()
            .unwrap_or(&self.scan_mode);
        match effective_scan_mode {
            AetherScanMode::Turbo => args.push("--turbo".to_string()),
            AetherScanMode::Balanced => args.push("--balanced".to_string()),
            AetherScanMode::Thorough => args.push("--thorough".to_string()),
            AetherScanMode::Stealth => args.push("--stealth".to_string()),
            AetherScanMode::Ironclad => args.push("--ironclad".to_string()),
        }

        // 6. Quick Reconnect (override if specified for optimization, without changing persistent settings)
        let effective_quick_reconnect = options
            .quick_reconnect_override
            .unwrap_or(self.quick_reconnect);
        if effective_quick_reconnect {
            args.push("--quick-reconnect".to_string());
        }

        // 7. Additional developer arguments
        for arg in &self.additional_arguments {
            if !arg.trim().is_empty() {
                args.push(arg.clone());
            }
        }

        args
    }
}

/// Returns the Aether startup and scan deadline budget based on official upstream Aether strategy budgets + margin:
/// - Turbo: official scan budget = 30s -> desktop deadline = 45s
/// - Balanced: official scan budget = 80s -> desktop deadline = 100s
/// - Thorough: official scan budget = 250s -> desktop deadline = 285s
/// - Stealth: official scan budget = 150s -> desktop deadline = 180s
/// - Ironclad: official scan budget = 180s -> desktop deadline = 210s
pub fn aether_startup_timeout(scan_mode: &AetherScanMode) -> std::time::Duration {
    match scan_mode {
        AetherScanMode::Turbo => std::time::Duration::from_secs(45),
        AetherScanMode::Balanced => std::time::Duration::from_secs(100),
        AetherScanMode::Thorough => std::time::Duration::from_secs(285),
        AetherScanMode::Stealth => std::time::Duration::from_secs(180),
        AetherScanMode::Ironclad => std::time::Duration::from_secs(210),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecondaryProxySettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SingBoxSettings {
    pub executable_path: String,
    pub interface_name: String,
    pub tun_address: String,
    pub mtu: u32,
    pub log_level: String,
    pub strict_route: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityScope {
    AppScoped,
    GlobalFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub process_names: Option<Vec<String>>,
    pub ports: Option<Vec<u16>>,
    pub network: Option<NetworkProtocol>,
    pub destination: RouteDestination,
    pub scope: CompatibilityScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilitySettings {
    pub generals_stun_turn_fallback: bool,
    pub private_ip_bypass: bool,
    pub custom_compatibility_rules: Vec<CompatibilityRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneralSettings {
    pub start_with_windows: bool,
    pub auto_connect: bool,
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
    pub reconnect_automatically: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            aether: AetherSettings {
                executable_path: "C:\\Aether\\aether.exe".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1819,
                protocol: AetherProtocol::Wireguard,
                ip_mode: AetherIpMode::Ipv4,
                scan_mode: AetherScanMode::Thorough,
                quick_reconnect: true,
                additional_arguments: vec![],
                launch_arguments: vec![],
            },
            secondary_proxy: SecondaryProxySettings {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 10808,
            },
            sing_box: SingBoxSettings {
                executable_path: "C:\\sing-box\\sing-box.exe".to_string(),
                interface_name: "singbox-tun".to_string(),
                tun_address: "172.19.0.1/30".to_string(),
                mtu: 1500,
                log_level: "info".to_string(),
                strict_route: true,
            },
            compatibility: CompatibilitySettings {
                generals_stun_turn_fallback: true,
                private_ip_bypass: true,
                custom_compatibility_rules: vec![],
            },
            general: GeneralSettings {
                start_with_windows: false,
                auto_connect: false,
                minimize_to_tray: true,
                start_minimized: false,
                reconnect_automatically: true,
            },
            application_rules: crate::routing::presets::get_default_rules(),
            first_run_completed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a_normal_connect_includes_quick_reconnect_when_configured() {
        let settings = AetherSettings {
            executable_path: "C:\\Aether\\aether.exe".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1819,
            protocol: AetherProtocol::Wireguard,
            ip_mode: AetherIpMode::Ipv4,
            scan_mode: AetherScanMode::Turbo,
            quick_reconnect: true,
            additional_arguments: vec![],
            launch_arguments: vec![],
        };

        let args = settings.build_cli_arguments(None);
        assert!(args.contains(&"--quick-reconnect".to_string()));
        assert!(args.contains(&"--turbo".to_string()));
        assert!(!args.contains(&"--thorough".to_string()));
    }

    #[test]
    fn test_b_optimization_launch_does_not_include_quick_reconnect() {
        let settings = AetherSettings {
            executable_path: "C:\\Aether\\aether.exe".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1819,
            protocol: AetherProtocol::Wireguard,
            ip_mode: AetherIpMode::Ipv4,
            scan_mode: AetherScanMode::Turbo,
            quick_reconnect: true, // Saved user setting is true
            additional_arguments: vec![],
            launch_arguments: vec![],
        };

        let opt_options = AetherLaunchOptions {
            quick_reconnect_override: Some(false),
            scan_mode_override: Some(AetherScanMode::Thorough),
        };

        let args = settings.build_cli_arguments_with_options(None, &opt_options);
        assert!(!args.contains(&"--quick-reconnect".to_string()));
    }

    #[test]
    fn test_c_optimization_launch_forces_thorough_without_mutating_saved_settings() {
        let settings = AetherSettings {
            executable_path: "C:\\Aether\\aether.exe".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1819,
            protocol: AetherProtocol::Wireguard,
            ip_mode: AetherIpMode::Ipv4,
            scan_mode: AetherScanMode::Turbo, // Saved user setting is Turbo
            quick_reconnect: true,
            additional_arguments: vec![],
            launch_arguments: vec![],
        };

        let opt_options = AetherLaunchOptions {
            quick_reconnect_override: Some(false),
            scan_mode_override: Some(AetherScanMode::Thorough),
        };

        let args = settings.build_cli_arguments_with_options(None, &opt_options);
        assert!(args.contains(&"--thorough".to_string()));
        assert!(!args.contains(&"--turbo".to_string()));
        // Saved setting struct was not mutated
        assert_eq!(settings.scan_mode, AetherScanMode::Turbo);
        assert_eq!(settings.quick_reconnect, true);
    }

    #[test]
    fn test_d_after_optimization_normal_connect_again_includes_quick_reconnect() {
        let settings = AetherSettings {
            executable_path: "C:\\Aether\\aether.exe".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1819,
            protocol: AetherProtocol::Wireguard,
            ip_mode: AetherIpMode::Ipv4,
            scan_mode: AetherScanMode::Balanced,
            quick_reconnect: true,
            additional_arguments: vec![],
            launch_arguments: vec![],
        };

        // 1. Simulate optimization run
        let opt_options = AetherLaunchOptions {
            quick_reconnect_override: Some(false),
            scan_mode_override: Some(AetherScanMode::Thorough),
        };
        let opt_args = settings.build_cli_arguments_with_options(None, &opt_options);
        assert!(!opt_args.contains(&"--quick-reconnect".to_string()));
        assert!(opt_args.contains(&"--thorough".to_string()));

        // 2. Subsequent normal Connect invocation
        let normal_args = settings.build_cli_arguments(None);
        assert!(normal_args.contains(&"--quick-reconnect".to_string()));
        assert!(normal_args.contains(&"--balanced".to_string()));
        assert!(!normal_args.contains(&"--thorough".to_string()));
    }
}
