use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub internet: ServiceHealth,
    pub aether_process: ServiceHealth,
    pub aether_socks: ServiceHealth,
    pub aether_tunnel: ServiceHealth,
    pub singbox_process: ServiceHealth,
    pub tun_interface: ServiceHealth,
    pub secondary_proxy: ServiceHealth,
    pub routing: ServiceHealth,
    pub cloudflare_trace: Option<CloudflareTrace>,
    pub last_checked_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealth {
    pub ok: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

impl ServiceHealth {
    pub fn ok(msg: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: msg.into(),
            latency_ms: None,
        }
    }

    pub fn ok_with_latency(msg: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            ok: true,
            message: msg.into(),
            latency_ms: Some(latency_ms),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: msg.into(),
            latency_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CloudflareTrace {
    pub ip: String,
    pub warp: String,
    pub colo: String,
    pub loc: String,
    pub latency_ms: u64,
}
