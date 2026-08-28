use crate::models::health::{CloudflareTrace, HealthStatus, ServiceHealth};
use std::time::{Duration, Instant};
use sysinfo::Networks;
use tokio::net::TcpStream;

pub struct HealthProber;

impl HealthProber {
    /// Probes if a local TCP / SOCKS5 port is accepting connections
    pub async fn check_port_open(host: &str, port: u16, timeout_ms: u64) -> bool {
        let addr = format!("{}:{}", host, port);
        tokio::time::timeout(Duration::from_millis(timeout_ms), TcpStream::connect(&addr))
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
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let start = Instant::now();
        let resp = client
            .get("https://www.cloudflare.com/cdn-cgi/trace")
            .send()
            .await
            .map_err(|e| format!("Failed to connect via {}: {}", proxy_url, e))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !resp.status().is_success() {
            return Err(format!(
                "Cloudflare trace returned HTTP status {}",
                resp.status()
            ));
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

        if ip.is_empty() {
            return Err("Invalid Cloudflare trace response: missing IP address".to_string());
        }

        Ok(CloudflareTrace {
            ip,
            warp,
            colo,
            loc,
            latency_ms,
        })
    }

    /// Verifies if the singbox-tun adapter interface exists in the Windows network stack
    pub fn check_tun_interface_exists(interface_name: &str) -> bool {
        let networks = Networks::new_with_refreshed_list();
        let target = interface_name.to_lowercase();
        for (name, _) in &networks {
            let n_lower = name.to_lowercase();
            if n_lower == target || n_lower.contains(&target) || n_lower.contains("singbox") {
                return true;
            }
        }
        false
    }

    /// Comprehensive system health check with NO hardcoded success values
    pub async fn evaluate_health(
        aether_host: &str,
        aether_port: u16,
        secondary_host: &str,
        secondary_port: u16,
        secondary_enabled: bool,
        tun_interface_name: &str,
        aether_process_running: bool,
        singbox_process_running: bool,
        is_connected: bool,
    ) -> HealthStatus {
        let now_ms = chrono::Utc::now().timestamp_millis();

        // 1. Aether Process Check
        let aether_socks_open = Self::check_port_open(aether_host, aether_port, 600).await;
        let aether_process = if aether_process_running {
            ServiceHealth::ok("Running (Managed)")
        } else if aether_socks_open {
            ServiceHealth::ok("Active (External Listener)")
        } else {
            ServiceHealth::err("Not running")
        };

        // 2. Aether SOCKS Socket Check
        let aether_socks = if aether_socks_open {
            ServiceHealth::ok(format!("Listening on {}:{}", aether_host, aether_port))
        } else {
            ServiceHealth::err(format!(
                "Port {}:{} closed or unreachable",
                aether_host, aether_port
            ))
        };

        // 3. Aether Tunnel Trace Probe (Actual SOCKS5 HTTP request)
        let (aether_tunnel, trace_opt) = if aether_socks_open {
            match Self::query_cloudflare_trace_via_socks5(aether_host, aether_port).await {
                Ok(trace) => {
                    let lat = trace.latency_ms;
                    let msg = format!("Connected · POP: {} · {} ms", trace.colo, lat);
                    (ServiceHealth::ok_with_latency(msg, lat), Some(trace))
                }
                Err(err) => (
                    ServiceHealth::err(format!("Trace probe failed: {}", err)),
                    None,
                ),
            }
        } else {
            (ServiceHealth::err("Unavailable (SOCKS port closed)"), None)
        };

        // 4. sing-box Process Check
        let singbox_process = if singbox_process_running {
            ServiceHealth::ok("Running (Active)")
        } else if is_connected {
            ServiceHealth::err("Exited unexpectedly")
        } else {
            ServiceHealth::err("Not running (Disconnected)")
        };

        // 5. TUN Interface Check (Real Windows network adapter query)
        let tun_exists = Self::check_tun_interface_exists(tun_interface_name);
        let tun_interface = if tun_exists {
            ServiceHealth::ok(format!("Adapter '{}' active", tun_interface_name))
        } else if is_connected {
            ServiceHealth::err(format!(
                "Adapter '{}' not found in network stack",
                tun_interface_name
            ))
        } else {
            ServiceHealth::err("Inactive (Disconnected)")
        };

        // 6. Secondary Proxy Check
        let secondary_proxy = if secondary_enabled {
            let sec_open = Self::check_port_open(secondary_host, secondary_port, 600).await;
            if sec_open {
                match Self::query_cloudflare_trace_via_socks5(secondary_host, secondary_port).await
                {
                    Ok(trace) => ServiceHealth::ok_with_latency(
                        format!("Connected · POP: {} · IP: {}", trace.colo, trace.ip),
                        trace.latency_ms,
                    ),
                    Err(e) => ServiceHealth::err(format!(
                        "Port reachable but SOCKS request failed: {}",
                        e
                    )),
                }
            } else {
                ServiceHealth::err(format!("{}:{} unreachable", secondary_host, secondary_port))
            }
        } else {
            ServiceHealth::ok("Disabled in settings")
        };

        // 7. System Routing State Check
        let routing = if is_connected && singbox_process_running && tun_exists {
            ServiceHealth::ok("TUN Routing Active")
        } else if is_connected {
            ServiceHealth::err("Degraded / Tunnel Interface Missing")
        } else {
            ServiceHealth::err("Direct (VPN Disconnected)")
        };

        // 8. Overall Internet Egress
        let internet = if aether_tunnel.ok || (secondary_enabled && secondary_proxy.ok) {
            ServiceHealth::ok("Active")
        } else {
            ServiceHealth::err("No verified internet route")
        };

        HealthStatus {
            internet,
            aether_process,
            aether_socks,
            aether_tunnel,
            singbox_process,
            tun_interface,
            secondary_proxy,
            routing,
            cloudflare_trace: trace_opt,
            last_checked_epoch_ms: now_ms,
        }
    }
}
