use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionMode { Auto, Manual, EmergencyTor }
impl Default for ConnectionMode { fn default() -> Self { Self::Manual } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionProfile {
    MasqueH2, MasqueH3, Wireguard, Gool, GoolClassic, MasqueInMasque,
    PsiphonAuto, PsiphonCdn, PsiphonReverse, Tor,
}
impl ConnectionProfile {
    pub fn supports_udp(self) -> bool {
        !matches!(self, Self::PsiphonAuto | Self::PsiphonCdn | Self::Tor)
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::MasqueH2 => "masque_h2", Self::MasqueH3 => "masque_h3",
            Self::Wireguard => "wireguard", Self::Gool => "gool",
            Self::GoolClassic => "gool_classic", Self::MasqueInMasque => "masque_in_masque",
            Self::PsiphonAuto => "psiphon_auto", Self::PsiphonCdn => "psiphon_cdn",
            Self::PsiphonReverse => "psiphon_reverse", Self::Tor => "tor",
        }
    }
    pub fn timeout_secs(self) -> u64 {
        match self { Self::PsiphonAuto | Self::PsiphonCdn => 180,
            Self::PsiphonReverse => 240, Self::Tor => 420, _ => 60 }
    }
    pub fn flags(self) -> Vec<String> {
        let flags: &[&str] = match self {
            Self::MasqueH2 => &["--masque", "--h2"],
            Self::MasqueH3 => &["--masque", "--h3"],
            Self::Wireguard => &["--wg"], Self::Gool => &["--gool", "--h2"],
            Self::GoolClassic => &["--gool-classic"], Self::MasqueInMasque => &["--mim", "--h2"],
            Self::PsiphonAuto => &["--psiphon-only", "--psiphon-mode", "auto"],
            Self::PsiphonCdn => &["--psiphon-only", "--psiphon-mode", "cdn"],
            Self::PsiphonReverse => &["--masque", "--h2", "--psiphon-reverse"],
            Self::Tor => &["--tor-only", "--tor-bridges"],
        };
        flags.iter().map(|s| s.to_string()).collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDetails {
    pub active_profile: Option<ConnectionProfile>,
    pub supports_udp: bool,
    pub phase: String,
    pub failure_reason: Option<String>,
}

pub fn auto_attempts(cached: Option<ConnectionProfile>) -> Vec<(ConnectionProfile, u64, bool)> {
    let mut attempts = Vec::new();
    if let Some(profile) = cached.filter(|p| !matches!(p, ConnectionProfile::Tor)) {
        attempts.push((profile, 25, true));
    }
    for p in [ConnectionProfile::MasqueH2, ConnectionProfile::MasqueH3,
        ConnectionProfile::PsiphonAuto, ConnectionProfile::PsiphonCdn] {
        attempts.push((p, p.timeout_secs(), false));
    }
    attempts
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn retries_cached_profile_before_fresh_fallback_and_never_auto_tor() {
        let a = auto_attempts(Some(ConnectionProfile::MasqueH2));
        assert_eq!(a[0], (ConnectionProfile::MasqueH2, 25, true));
        assert_eq!(a[1], (ConnectionProfile::MasqueH2, 60, false));
        assert_eq!(a.len(), 5);
        assert_eq!(auto_attempts(Some(ConnectionProfile::Tor)).len(), 4);
    }
    #[test] fn reverse_tunnel_preserves_udp_but_plain_public_proxies_do_not() {
        assert!(ConnectionProfile::PsiphonReverse.supports_udp());
        assert!(!ConnectionProfile::PsiphonAuto.supports_udp());
        assert!(!ConnectionProfile::Tor.supports_udp());
    }
}

#[cfg(test)] mod migration_tests {
    use crate::models::{AppSettings,profile::ConnectionMode};
    #[test] fn old_settings_preserve_manual_protocol_and_rules() {
        let before=AppSettings::default();
        let mut json=serde_json::to_value(&before).unwrap();
        let aether=json["aether"].as_object_mut().unwrap();
        aether.remove("connectionMode"); aether.remove("manualProfile");aether.remove("ech");
        let loaded:AppSettings=serde_json::from_value(json).unwrap();
        assert_eq!(loaded.aether.connection_mode,ConnectionMode::Manual);
        assert_eq!(loaded.aether.protocol,before.aether.protocol);
        assert_eq!(loaded.application_rules,before.application_rules);
    }
}
