use serde::{Deserialize, Serialize};
use super::app_rule::{ApplicationRule, RouteDestination};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AetherSettings {
    pub executable_path: String,
    pub host: String,
    pub port: u16,
    pub launch_arguments: Vec<String>,
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
    /// Generals Online STUN/TURN fallback: ports 3478, 5349 -> Direct (fallback only, evaluated after explicit application rules)
    pub generals_stun_turn_fallback: bool,
    /// Private LAN IP bypass (RFC 1918) -> Direct
    pub private_ip_bypass: bool,
    /// Custom app-scoped or global fallback compatibility rules
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