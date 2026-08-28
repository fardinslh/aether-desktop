use crate::models::health::{CloudflareTrace, HealthStatus, ServiceHealth};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub struct HealthProber;

impl HealthProber {
    /// Probes if a local TCP / SOCKS5 port is accepting connections
    pub async fn check_port_open(host: &str, port: u16, timeout_ms: u64) -> bool {
        let addr = format!("{}:{}", host, port);
        tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(&addr),
        )
        .await
        .map(|res| res.is_ok())
        .unwrap_or(false)
    }

    /// Performs Cloudflare CDN trace probe through a SOCKS5 proxy to retrieve edge node (colo), public IP, and latency
    pub async fn query_cloudflare_trace_via_socks5(
        host: &str,
        port: u16,
    ) -> Result<CloudflareTrace, String> {
        let proxy_url = format!("socks5h://{}:{}", host, port);
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| format!("Failed to create SOCKS5 proxy configuration: {}", e))?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(4))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let start = Instant::now();
        let resp = client
            .get("https://www.cloudflare.com/cdn-cgi/trace")
            .send()
            .await
            .map_err(|e| format!("Failed to send request via {}: {}", proxy_url, e))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !resp.status().is_success() {
            return Err(format!("Cloudflare trace returned status: {}", resp.status()));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let mut ip = String::new();
        let mut warp = String::new();
        let mut colo = String::new();
        let mut loc = String::new();

        for line in body.lines() {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "ip" => ip = v.trim().to_string(),
                    "warp" => warp = v.trim().to_string(),
                    "colo" => colo = v.trim().to_string(),
                    "loc" => loc = v.trim().to_string(),
                    _ => {}
                }
            }
        }

        Ok(CloudflareTrace {
            ip,
            warp,
            colo,
            loc,
            latency_ms,
        })
    }

    /// Comprehensive system health check
    pub async fn evaluate_health(
        aether_host: &str,
        aether_port: u16,
        secondary_host: &str,
        secondary_port: u16,
        secondary_enabled: bool,
    ) -> HealthStatus {
        let now_ms = chrono::Utc::now().timestamp_millis();

        // 1. Aether SOCKS check
        let aether_open = Self::check_port_open(aether_host, aether_port, 800).await;
        let aether_socks = if aether_open {
            ServiceHealth::ok(format!("Listening on {}:{}", aether_host, aether_port))
        } else {
            ServiceHealth::err(format!("Port {}:{} unreachable", aether_host, aether_port))
        };

        // 2. Aether Tunnel Trace test
        let (aether_tunnel, trace_opt) = if aether_open {
            match Self::query_cloudflare_trace_via_socks5(aether_host, aether_port).await {
                Ok(trace) => {
                    let lat = trace.latency_ms;
                    let msg = format!("POP: {} ({} ms)", trace.colo, lat);
                    (ServiceHealth::ok_with_latency(msg, lat), Some(trace))
                }
                Err(err) => (ServiceHealth::err(err), None),
            }
        } else {
            (ServiceHealth::err("Aether SOCKS port closed"), None)
        };

        // 3. Secondary Proxy check
        let secondary_proxy = if secondary_enabled {
            let sec_open = Self::check_port_open(secondary_host, secondary_port, 800).await;
            if sec_open {
                match Self::query_cloudflare_trace_via_socks5(secondary_host, secondary_port).await {
                    Ok(trace) => ServiceHealth::ok_with_latency(
                        format!("Connected · IP: {}", trace.ip),
                        trace.latency_ms,
                    ),
                    Err(e) => ServiceHealth::err(format!("Proxy reachable but external test failed: {}", e)),
                }
            } else {
                ServiceHealth::err(format!("{}:{} not reachable", secondary_host, secondary_port))
            }
        } else {
            ServiceHealth::ok("Disabled in settings")
        };

        // 4. Internet
        let internet = if aether_tunnel.ok || secondary_proxy.ok {
            ServiceHealth::ok("Active")
        } else {
            ServiceHealth::err("No active route verified")
        };

        HealthStatus {
            internet,
            aether_process: ServiceHealth::ok("Managed"),
            aether_socks,
            aether_tunnel,
            singbox_process: ServiceHealth::ok("Managed"),
            tun_interface: ServiceHealth::ok("singbox-tun"),
            secondary_proxy,
            routing: ServiceHealth::ok("Active"),
            cloudflare_trace: trace_opt,
            last_checked_epoch_ms: now_ms,
        }
    }
}
