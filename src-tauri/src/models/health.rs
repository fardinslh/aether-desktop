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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatencyProfile {
    pub total_samples: usize,
    pub successful_samples: usize,
    pub samples_ms: Vec<u64>,
    pub median_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub jitter_mad_ms: u64,
    pub latest_trace: Option<CloudflareTrace>,
}

impl LatencyProfile {
    pub fn compute_from_samples(
        samples: &[u64],
        total_samples: usize,
        latest_trace: Option<CloudflareTrace>,
    ) -> Result<Self, String> {
        if samples.len() < 4 {
            return Err(format!(
                "Insufficient valid latency samples ({}/{} succeeded, minimum 4 required)",
                samples.len(),
                total_samples
            ));
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let len = sorted.len();
        let median_ms = if len % 2 == 1 {
            sorted[len / 2]
        } else {
            (sorted[(len / 2) - 1] + sorted[len / 2]) / 2
        };
        let min_ms = sorted[0];
        let max_ms = sorted[len - 1];

        let mut abs_devs: Vec<u64> = sorted
            .iter()
            .map(|&x| (x as i64 - median_ms as i64).abs() as u64)
            .collect();
        abs_devs.sort_unstable();
        let jitter_mad_ms = if len % 2 == 1 {
            abs_devs[len / 2]
        } else {
            (abs_devs[(len / 2) - 1] + abs_devs[len / 2]) / 2
        };

        Ok(Self {
            total_samples,
            successful_samples: samples.len(),
            samples_ms: samples.to_vec(),
            median_ms,
            min_ms,
            max_ms,
            jitter_mad_ms,
            latest_trace,
        })
    }
}
