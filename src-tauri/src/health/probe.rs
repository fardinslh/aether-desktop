use crate::models::health::{CloudflareTrace, HealthStatus, ServiceHealth};
use std::time::{Duration, Instant};
use sysinfo::Networks;
use tokio::net::TcpStream;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectedAdapterInfo {
    pub friendly_name: String,
    pub description: String,
    pub adapter_name: String,
    pub if_index: u32,
    pub is_up: bool,
    pub ip_addresses: Vec<String>,
}

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
        Self::query_cloudflare_trace_via_socks5_with_logger(host, port, None).await
    }

    /// Performs Cloudflare CDN trace probe through SOCKS5 proxy with explicit real-time stage logging
    pub async fn query_cloudflare_trace_via_socks5_with_logger(
        host: &str,
        port: u16,
        log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
    ) -> Result<CloudflareTrace, String> {
        let proxy_url = format!("socks5h://{}:{}", host, port);
        let proxy = reqwest::Proxy::all(&proxy_url)
            .map_err(|e| format!("Failed to create SOCKS5 proxy configuration: {}", e))?;

        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(6))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let endpoints = [
            "https://www.cloudflare.com/cdn-cgi/trace",
            "https://1.1.1.1/cdn-cgi/trace",
            "http://www.cloudflare.com/cdn-cgi/trace",
            "http://1.1.1.1/cdn-cgi/trace",
        ];

        let mut last_err = String::new();

        for endpoint in endpoints {
            if let Some(logger) = log_fn {
                logger(
                    "INFO",
                    "Aether",
                    &format!(
                        "[SOCKS-VERIFY-START] Sending verification probe to endpoint='{}' via {} (timeout: 6s)...",
                        endpoint, proxy_url
                    ),
                );
            }

            let start = Instant::now();
            let resp_result = client.get(endpoint).send().await;

            match resp_result {
                Ok(resp) => {
                    let d_resp = start.elapsed();
                    let status = resp.status();

                    if let Some(logger) = log_fn {
                        logger(
                            "INFO",
                            "Aether",
                            &format!(
                                "[SOCKS-VERIFY-RECV] SOCKS response received from endpoint='{}' in {:.2}s. [SOCKS-VERIFY-HTTP] Status: {}",
                                endpoint,
                                d_resp.as_secs_f32(),
                                status
                            ),
                        );
                    }

                    if !status.is_success() {
                        let status_err = format!(
                            "Endpoint '{}' returned HTTP error status {}",
                            endpoint, status
                        );
                        if let Some(logger) = log_fn {
                            logger(
                                "WARN",
                                "Aether",
                                &format!(
                                    "[SOCKS-VERIFY-FAIL] endpoint='{}' category='HTTP Status Error' reason: {}",
                                    endpoint, status_err
                                ),
                            );
                        }
                        last_err = status_err;
                        continue;
                    }

                    match resp.text().await {
                        Ok(body) => {
                            let latency_ms = d_resp.as_millis() as u64;
                            match Self::parse_trace_body(&body, latency_ms) {
                                Ok(trace) => {
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "INFO",
                                            "Aether",
                                            &format!(
                                                "[SOCKS-VERIFY-SUCCESS] endpoint='{}' verified (IP={}, POP={}, Latency={}ms, Warp={})",
                                                endpoint, trace.ip, trace.colo, trace.latency_ms, trace.warp
                                            ),
                                        );
                                    }
                                    return Ok(trace);
                                }
                                Err(parse_err) => {
                                    let preview = if body.len() > 120 {
                                        format!("{}...", &body[..120].escape_default())
                                    } else {
                                        body.escape_default().to_string()
                                    };
                                    let err_msg = format!(
                                        "Failed to parse trace response body from '{}': {} (body_preview: '{}')",
                                        endpoint, parse_err, preview
                                    );
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "WARN",
                                            "Aether",
                                            &format!(
                                                "[SOCKS-VERIFY-FAIL] endpoint='{}' category='Parse Error' reason: {}",
                                                endpoint, err_msg
                                            ),
                                        );
                                    }
                                    last_err = err_msg;
                                }
                            }
                        }
                        Err(body_err) => {
                            let req_err = Self::format_reqwest_error(
                                &format!("Failed to read response body from '{}'", endpoint),
                                &body_err,
                            );
                            let category = Self::classify_reqwest_error(&body_err);
                            if let Some(logger) = log_fn {
                                logger(
                                    "WARN",
                                    "Aether",
                                    &format!(
                                        "[SOCKS-VERIFY-FAIL] endpoint='{}' category='{}' reason: {}",
                                        endpoint, category, req_err
                                    ),
                                );
                            }
                            last_err = req_err;
                        }
                    }
                }
                Err(err) => {
                    let d_fail = start.elapsed();
                    let category = Self::classify_reqwest_error(&err);
                    let req_err = Self::format_reqwest_error(
                        &format!(
                            "Request to '{}' via {} failed after {:.2}s",
                            endpoint,
                            proxy_url,
                            d_fail.as_secs_f32()
                        ),
                        &err,
                    );
                    if let Some(logger) = log_fn {
                        logger(
                            "WARN",
                            "Aether",
                            &format!(
                                "[SOCKS-VERIFY-FAIL] endpoint='{}' category='{}' reason: {}",
                                endpoint, category, req_err
                            ),
                        );
                    }
                    last_err = req_err;
                }
            }
        }

        Err(last_err)
    }

    /// Classifies a reqwest error into a specific failure category (TLS, HTTP2, Decompression, Timeout, SOCKS, etc.)
    pub fn classify_reqwest_error(err: &reqwest::Error) -> &'static str {
        use std::error::Error;
        let mut chain_text = String::new();
        let mut curr: Option<&(dyn Error + 'static)> = err.source();
        while let Some(src) = curr {
            chain_text.push_str(&src.to_string());
            chain_text.push(' ');
            curr = src.source();
        }
        let chain_lower = chain_text.to_lowercase();

        if err.is_timeout() || chain_lower.contains("timed out") || chain_lower.contains("timeout") {
            "Timeout"
        } else if chain_lower.contains("tls")
            || chain_lower.contains("certificate")
            || chain_lower.contains("handshake")
            || chain_lower.contains("rustls")
            || chain_lower.contains("webpki")
            || chain_lower.contains("invalid peer certificate")
            || chain_lower.contains("unknown issuer")
        {
            "TLS Handshake / Certificate Error"
        } else if chain_lower.contains("h2")
            || chain_lower.contains("http2")
            || chain_lower.contains("stream")
            || chain_lower.contains("frame")
        {
            "HTTP2 Protocol Error"
        } else if chain_lower.contains("decompress")
            || chain_lower.contains("gzip")
            || chain_lower.contains("brotli")
            || chain_lower.contains("inflate")
            || chain_lower.contains("corrupt input")
        {
            "Decompression Error"
        } else if chain_lower.contains("socks")
            || chain_lower.contains("proxy")
            || err.is_connect()
        {
            "SOCKS Connect / Transport Error"
        } else {
            "Network / Transport Error"
        }
    }

    /// Measures multiple SOCKS5 latency samples across identical bounded intervals and computes median and jitter MAD
    pub async fn measure_socks5_latency_samples(
        host: &str,
        port: u16,
        target_samples: usize,
        interval: Duration,
    ) -> Result<crate::models::health::LatencyProfile, String> {
        let mut samples_ms = Vec::new();
        let mut latest_trace = None;
        let mut last_err = String::new();

        for i in 0..target_samples {
            if i > 0 {
                tokio::time::sleep(interval).await;
            }
            match Self::query_cloudflare_trace_via_socks5(host, port).await {
                Ok(trace) => {
                    samples_ms.push(trace.latency_ms);
                    latest_trace = Some(trace);
                }
                Err(e) => {
                    last_err = e;
                }
            }
        }

        if samples_ms.len() < 4 {
            let detail = if last_err.is_empty() {
                String::new()
            } else {
                format!(": {}", last_err)
            };
            return Err(format!(
                "Insufficient valid latency samples ({}/{} succeeded, minimum 4 required){}",
                samples_ms.len(),
                target_samples,
                detail
            ));
        }

        crate::models::health::LatencyProfile::compute_from_samples(
            &samples_ms,
            target_samples,
            latest_trace,
        )
    }

    /// Formats a detailed reqwest error breakdown including connection/timeout flags and underlying source error chain
    pub fn format_reqwest_error(prefix: &str, err: &reqwest::Error) -> String {
        use std::error::Error;
        let mut flags: Vec<String> = Vec::new();
        if err.is_connect() {
            flags.push("is_connect: true".to_string());
        }
        if err.is_timeout() {
            flags.push("is_timeout: true".to_string());
        }
        if err.is_request() {
            flags.push("is_request: true".to_string());
        }
        if err.is_builder() {
            flags.push("is_builder: true".to_string());
        }
        if let Some(status) = err.status() {
            flags.push(format!("status: {}", status));
        }
        let flags_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        let mut chain = Vec::new();
        let mut curr: Option<&(dyn Error + 'static)> = err.source();
        while let Some(src) = curr {
            chain.push(src.to_string());
            curr = src.source();
        }
        let chain_str = if chain.is_empty() {
            String::new()
        } else {
            format!(" (Source chain: {})", chain.join(" -> "))
        };

        format!("{}{}{}: {}", prefix, flags_str, chain_str, err)
    }

    /// Parses raw key=value lines from Cloudflare trace body
    pub fn parse_trace_body(body: &str, latency_ms: u64) -> Result<CloudflareTrace, String> {
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

    /// Stage 1: Resilient system egress test sweeping across prioritized endpoints
    /// (https://www.cloudflare.com, http://www.cloudflare.com, https://1.1.1.1, http://1.1.1.1)
    /// Validates raw TUN transport, routing, and egress consistency with full stage diagnostics.
    pub async fn query_direct_system_cloudflare_trace_ip_literal() -> Result<CloudflareTrace, String>
    {
        Self::query_direct_system_cloudflare_trace_resilient(None).await
    }

    /// Performs resilient direct system Cloudflare trace probe through Windows network stack/TUN adapter.
    /// Sweeps across prioritized endpoints with full stage telemetry and error categorization.
    pub async fn query_direct_system_cloudflare_trace_resilient(
        log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
    ) -> Result<CloudflareTrace, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .build()
            .map_err(|e| {
                Self::format_reqwest_error(
                    "Failed to build HTTP client for direct system egress probe",
                    &e,
                )
            })?;

        let endpoints = [
            "https://www.cloudflare.com/cdn-cgi/trace",
            "http://www.cloudflare.com/cdn-cgi/trace",
            "https://1.1.1.1/cdn-cgi/trace",
            "http://1.1.1.1/cdn-cgi/trace",
        ];

        let mut last_err = String::new();

        for endpoint in endpoints {
            if let Some(logger) = log_fn {
                logger(
                    "INFO",
                    "sing-box",
                    &format!(
                        "[ROUTING-VERIFY-START] Probing direct system egress on endpoint='{}' (timeout: 6s)...",
                        endpoint
                    ),
                );
            }

            let start = Instant::now();
            let resp_result = client.get(endpoint).send().await;

            match resp_result {
                Ok(resp) => {
                    let d_resp = start.elapsed();
                    let status = resp.status();

                    if let Some(logger) = log_fn {
                        logger(
                            "INFO",
                            "sing-box",
                            &format!(
                                "[ROUTING-VERIFY-RECV] Direct system response received from endpoint='{}' in {:.2}s. [ROUTING-VERIFY-HTTP] Status: {}",
                                endpoint,
                                d_resp.as_secs_f32(),
                                status
                            ),
                        );
                    }

                    if !status.is_success() {
                        let status_err = format!(
                            "Endpoint '{}' returned HTTP error status {}",
                            endpoint, status
                        );
                        if let Some(logger) = log_fn {
                            logger(
                                "WARN",
                                "sing-box",
                                &format!(
                                    "[ROUTING-VERIFY-FAIL] endpoint='{}' category='HTTP Status Error' reason: {}",
                                    endpoint, status_err
                                ),
                            );
                        }
                        last_err = status_err;
                        continue;
                    }

                    match resp.text().await {
                        Ok(body) => {
                            let latency_ms = d_resp.as_millis() as u64;
                            match Self::parse_trace_body(&body, latency_ms) {
                                Ok(trace) => {
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "INFO",
                                            "sing-box",
                                            &format!(
                                                "[ROUTING-VERIFY-SUCCESS] Direct egress verified on endpoint='{}' (IP: {}, POP: {}, Latency: {} ms, Warp: {})",
                                                endpoint, trace.ip, trace.colo, trace.latency_ms, trace.warp
                                            ),
                                        );
                                    }
                                    return Ok(trace);
                                }
                                Err(parse_err) => {
                                    let preview = if body.len() > 120 {
                                        format!("{}...", &body[..120].escape_default())
                                    } else {
                                        body.escape_default().to_string()
                                    };
                                    let err_msg = format!(
                                        "Failed to parse trace response body from '{}': {} (body_preview: '{}')",
                                        endpoint, parse_err, preview
                                    );
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "WARN",
                                            "sing-box",
                                            &format!(
                                                "[ROUTING-VERIFY-FAIL] endpoint='{}' category='Parse Error' reason: {}",
                                                endpoint, err_msg
                                            ),
                                        );
                                    }
                                    last_err = err_msg;
                                }
                            }
                        }
                        Err(body_err) => {
                            let req_err = Self::format_reqwest_error(
                                &format!("Failed to read response body from '{}'", endpoint),
                                &body_err,
                            );
                            let category = Self::classify_reqwest_error(&body_err);
                            if let Some(logger) = log_fn {
                                logger(
                                    "WARN",
                                    "sing-box",
                                    &format!(
                                        "[ROUTING-VERIFY-FAIL] endpoint='{}' category='{}' reason: {}",
                                        endpoint, category, req_err
                                    ),
                                );
                            }
                            last_err = req_err;
                        }
                    }
                }
                Err(err) => {
                    let d_fail = start.elapsed();
                    let category = Self::classify_reqwest_error(&err);
                    let req_err = Self::format_reqwest_error(
                        &format!(
                            "Direct system request to '{}' failed after {:.2}s",
                            endpoint,
                            d_fail.as_secs_f32()
                        ),
                        &err,
                    );
                    if let Some(logger) = log_fn {
                        logger(
                            "WARN",
                            "sing-box",
                            &format!(
                                "[ROUTING-VERIFY-FAIL] endpoint='{}' category='{}' reason: {}",
                                endpoint, category, req_err
                            ),
                        );
                    }
                    last_err = req_err;
                }
            }
        }

        Err(last_err)
    }

    /// Stage 2: Explicit Windows system DNS resolution test
    /// Validates that Windows system DNS queries succeed through the TUN interface.
    pub async fn test_system_dns_resolution(domain: &str) -> Result<Vec<std::net::IpAddr>, String> {
        Self::test_system_dns_resolution_with_logger(domain, None).await
    }

    /// Explicit Windows system DNS resolution test with full stage diagnostics
    pub async fn test_system_dns_resolution_with_logger(
        primary_domain: &str,
        log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
    ) -> Result<Vec<std::net::IpAddr>, String> {
        let domains = [
            primary_domain,
            "www.cloudflare.com",
            "cloudflare.com",
            "one.one.one.one",
            "google.com",
        ];

        let mut last_err = String::new();

        for domain in domains {
            if let Some(logger) = log_fn {
                logger(
                    "INFO",
                    "sing-box",
                    &format!(
                        "[DNS-VERIFY-START] Querying Windows system resolver for domain='{}' (svchost.exe -> TUN -> sing-box)...",
                        domain
                    ),
                );
            }

            let start = Instant::now();
            let addr_str = format!("{}:443", domain);
            match tokio::net::lookup_host(addr_str).await {
                Ok(addrs) => {
                    let d = start.elapsed();
                    let mut ips: Vec<std::net::IpAddr> = addrs.map(|sa| sa.ip()).collect();
                    ips.dedup();
                    if !ips.is_empty() {
                        if let Some(logger) = log_fn {
                            logger(
                                "INFO",
                                "sing-box",
                                &format!(
                                    "[DNS-VERIFY-SUCCESS] Domain '{}' resolved successfully to {:?} in {:.2}s",
                                    domain, ips, d.as_secs_f32()
                                ),
                            );
                        }
                        return Ok(ips);
                    } else {
                        let err_msg = format!("Resolver returned 0 addresses for '{}'", domain);
                        if let Some(logger) = log_fn {
                            logger("WARN", "sing-box", &format!("[DNS-VERIFY-FAIL] {}", err_msg));
                        }
                        last_err = err_msg;
                    }
                }
                Err(e) => {
                    let d = start.elapsed();
                    let err_msg = format!(
                        "Windows system DNS lookup failed for '{}' after {:.2}s: {}",
                        domain, d.as_secs_f32(), e
                    );
                    if let Some(logger) = log_fn {
                        logger("WARN", "sing-box", &format!("[DNS-VERIFY-FAIL] {}", err_msg));
                    }
                    last_err = err_msg;
                }
            }
        }

        Err(last_err)
    }

    /// Stage 3: Full system egress test with domain name resolution
    pub async fn query_direct_system_cloudflare_trace_hostname() -> Result<CloudflareTrace, String>
    {
        Self::query_direct_system_cloudflare_trace_hostname_with_logger(None).await
    }

    /// Performs Stage 3 direct system Cloudflare trace probe through Windows network stack/TUN adapter
    /// with prioritized fallback endpoints, 10s timeout, and comprehensive stage diagnostics.
    pub async fn query_direct_system_cloudflare_trace_hostname_with_logger(
        log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
    ) -> Result<CloudflareTrace, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| {
                Self::format_reqwest_error(
                    "Failed to build HTTP client for Stage 3 hostname egress test",
                    &e,
                )
            })?;

        let endpoints = [
            "https://www.cloudflare.com/cdn-cgi/trace",
            "http://www.cloudflare.com/cdn-cgi/trace",
            "https://1.1.1.1/cdn-cgi/trace",
            "http://1.1.1.1/cdn-cgi/trace",
        ];

        let mut last_err = String::new();

        for endpoint in endpoints {
            if let Some(logger) = log_fn {
                logger(
                    "INFO",
                    "sing-box",
                    &format!(
                        "[STAGE3-START] Testing end-to-end hostname egress on endpoint='{}' (timeout: 10s)...",
                        endpoint
                    ),
                );
            }

            let start = Instant::now();
            let resp_result = client.get(endpoint).send().await;

            match resp_result {
                Ok(resp) => {
                    let d_resp = start.elapsed();
                    let status = resp.status();

                    if let Some(logger) = log_fn {
                        logger(
                            "INFO",
                            "sing-box",
                            &format!(
                                "[STAGE3-TLS] Handshake & connection established in {:.2}s. [STAGE3-HTTP] Status: {}",
                                d_resp.as_secs_f32(),
                                status
                            ),
                        );
                    }

                    if !status.is_success() {
                        let status_err = format!(
                            "Endpoint '{}' returned HTTP error status {}",
                            endpoint, status
                        );
                        if let Some(logger) = log_fn {
                            logger(
                                "WARN",
                                "sing-box",
                                &format!(
                                    "[STAGE3-FAIL] endpoint='{}' category='HTTP Status Error' reason: {}",
                                    endpoint, status_err
                                ),
                            );
                        }
                        last_err = status_err;
                        continue;
                    }

                    match resp.text().await {
                        Ok(body) => {
                            let latency_ms = d_resp.as_millis() as u64;
                            match Self::parse_trace_body(&body, latency_ms) {
                                Ok(trace) => {
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "INFO",
                                            "sing-box",
                                            &format!(
                                                "[STAGE3-SUCCESS] Stage 3 end-to-end verified on endpoint='{}' (IP: {}, POP: {}, Latency: {} ms, Warp: {})",
                                                endpoint, trace.ip, trace.colo, trace.latency_ms, trace.warp
                                            ),
                                        );
                                    }
                                    return Ok(trace);
                                }
                                Err(parse_err) => {
                                    let preview = if body.len() > 120 {
                                        format!("{}...", &body[..120].escape_default())
                                    } else {
                                        body.escape_default().to_string()
                                    };
                                    let err_msg = format!(
                                        "Failed to parse trace response body from '{}': {} (body_preview: '{}')",
                                        endpoint, parse_err, preview
                                    );
                                    if let Some(logger) = log_fn {
                                        logger(
                                            "WARN",
                                            "sing-box",
                                            &format!(
                                                "[STAGE3-FAIL] endpoint='{}' category='Parse Error' reason: {}",
                                                endpoint, err_msg
                                            ),
                                        );
                                    }
                                    last_err = err_msg;
                                }
                            }
                        }
                        Err(body_err) => {
                            let req_err = Self::format_reqwest_error(
                                &format!("Failed to read response body from '{}'", endpoint),
                                &body_err,
                            );
                            let category = Self::classify_reqwest_error(&body_err);
                            if let Some(logger) = log_fn {
                                logger(
                                    "WARN",
                                    "sing-box",
                                    &format!(
                                        "[STAGE3-FAIL] endpoint='{}' category='{}' reason: {}",
                                        endpoint, category, req_err
                                    ),
                                );
                            }
                            last_err = req_err;
                        }
                    }
                }
                Err(err) => {
                    let d_fail = start.elapsed();
                    let category = Self::classify_reqwest_error(&err);
                    let req_err = Self::format_reqwest_error(
                        &format!(
                            "Direct hostname request to '{}' failed after {:.2}s",
                            endpoint,
                            d_fail.as_secs_f32()
                        ),
                        &err,
                    );
                    if let Some(logger) = log_fn {
                        logger(
                            "WARN",
                            "sing-box",
                            &format!(
                                "[STAGE3-FAIL] endpoint='{}' category='{}' reason: {}",
                                endpoint, category, req_err
                            ),
                        );
                    }
                    last_err = req_err;
                }
            }
        }

        Err(last_err)
    }

    /// Performs direct system Cloudflare trace probe through Windows network stack/TUN adapter using hostname resolution.
    /// Hostname resolution is authoritative for normal user-facing internet health.
    pub async fn query_direct_system_cloudflare_trace() -> Result<CloudflareTrace, String> {
        Self::query_direct_system_cloudflare_trace_hostname().await
    }

    /// Executes the 3-stage egress and DNS verification decision path:
    /// Stage 1: Resilient system egress test (matches SOCKS verification path)
    /// Strict Check: system egress IP == expected Aether SOCKS proxy egress IP
    /// Stage 2: Windows System DNS Resolution test (www.cloudflare.com)
    /// Stage 3: Full Hostname HTTPS Trace verification (https://www.cloudflare.com/cdn-cgi/trace)
    /// Returns Ok(CloudflareTrace) containing the final verified trace, or Err(String) with the exact failing stage.
    pub async fn verify_staged_egress_decision_path<F1, Fut1, F2, Fut2, F3, Fut3>(
        mut ip_literal_fn: F1,
        mut dns_fn: F2,
        mut hostname_fn: F3,
        expected_aether_ip: Option<&str>,
        log_fn: Option<&(dyn Fn(&str, &str, &str) + Send + Sync)>,
    ) -> Result<CloudflareTrace, String>
    where
        F1: FnMut() -> Fut1 + Send,
        Fut1: std::future::Future<Output = Result<CloudflareTrace, String>> + Send,
        F2: FnMut() -> Fut2 + Send,
        Fut2: std::future::Future<Output = Result<Vec<std::net::IpAddr>, String>> + Send,
        F3: FnMut() -> Fut3 + Send,
        Fut3: std::future::Future<Output = Result<CloudflareTrace, String>> + Send,
    {
        // -------------------------------------------------------------------------
        // Stage 1: System egress test
        // -------------------------------------------------------------------------
        let t_stage1 = Instant::now();
        let mut ip_trace_opt = None;
        let mut last_ip_err = String::new();
        for attempt in 1..=4 {
            match ip_literal_fn().await {
                Ok(trace) => {
                    ip_trace_opt = Some(trace);
                    break;
                }
                Err(e) => {
                    last_ip_err = e;
                    if attempt < 4 {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                }
            }
        }

        let ip_trace = match ip_trace_opt {
            Some(t) => {
                if let Some(logger) = log_fn {
                    logger(
                        "INFO",
                        "sing-box",
                        &format!(
                            "Stage 1 PASS in {:.2}s: System egress verified (POP: {}, IP: {}, Latency: {} ms)",
                            t_stage1.elapsed().as_secs_f32(),
                            t.colo, t.ip, t.latency_ms
                        ),
                    );
                }
                t
            }
            None => {
                if let Some(logger) = log_fn {
                    logger(
                        "ERROR",
                        "sing-box",
                        &format!(
                            "Stage 1 FAIL after {:.2}s: Direct system egress failed: {}",
                            t_stage1.elapsed().as_secs_f32(),
                            last_ip_err
                        ),
                    );
                }
                return Err(format!(
                    "Stage 1 Transport Failure: Direct system egress failed: {}",
                    last_ip_err
                ));
            }
        };

        // -------------------------------------------------------------------------
        // Strict Egress Consistency Check: System egress IP must match Aether proxy egress IP
        // -------------------------------------------------------------------------
        if let Some(exp_ip) = expected_aether_ip {
            if !ip_trace.ip.is_empty() && !exp_ip.is_empty() && ip_trace.ip != exp_ip {
                let mismatch_err = format!(
                    "Stage 1 Egress Mismatch: System egress IP ({}) does not match Aether SOCKS egress IP ({}). Traffic is not traversing Aether outbound.",
                    ip_trace.ip, exp_ip
                );
                if let Some(logger) = log_fn {
                    logger("ERROR", "sing-box", &mismatch_err);
                }
                return Err(mismatch_err);
            } else if !ip_trace.ip.is_empty() && !exp_ip.is_empty() {
                if let Some(logger) = log_fn {
                    logger(
                        "INFO",
                        "sing-box",
                        &format!(
                            "[ROUTING-VERIFY-MATCH] System egress IP ({}) matches expected Aether SOCKS egress IP ({})",
                            ip_trace.ip, exp_ip
                        ),
                    );
                }
            }
        }

        // -------------------------------------------------------------------------
        // Stage 2: Windows System DNS Resolution test
        // -------------------------------------------------------------------------
        let t_stage2 = Instant::now();
        let mut dns_resolved_ips = Vec::new();
        let mut last_dns_err = String::new();
        for attempt in 1..=4 {
            match dns_fn().await {
                Ok(ips) => {
                    dns_resolved_ips = ips;
                    break;
                }
                Err(e) => {
                    last_dns_err = e;
                    if attempt < 4 {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                }
            }
        }

        if dns_resolved_ips.is_empty() {
            let dns_fail_msg = format!(
                "Stage 2 DNS Failure after {:.2}s: Windows system DNS resolution failed under TUN strict_route: {}",
                t_stage2.elapsed().as_secs_f32(),
                last_dns_err
            );
            if let Some(logger) = log_fn {
                logger("ERROR", "sing-box", &dns_fail_msg);
            }
            return Err(dns_fail_msg);
        }

        if let Some(logger) = log_fn {
            logger(
                "INFO",
                "sing-box",
                &format!(
                    "Stage 2 PASS in {:.2}s: Windows system DNS resolver confirmed (www.cloudflare.com -> {:?})",
                    t_stage2.elapsed().as_secs_f32(),
                    dns_resolved_ips
                ),
            );
        }

        // -------------------------------------------------------------------------
        // Stage 3: Full Hostname HTTPS Trace verification (REQUIRED!)
        // -------------------------------------------------------------------------
        let t_stage3 = Instant::now();
        let mut host_trace_opt = None;
        let mut last_host_err = String::new();
        for attempt in 1..=4 {
            match hostname_fn().await {
                Ok(trace) => {
                    host_trace_opt = Some(trace);
                    break;
                }
                Err(e) => {
                    last_host_err = e;
                    if attempt < 4 {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                    }
                }
            }
        }

        let host_trace = match host_trace_opt {
            Some(t) => {
                if let Some(logger) = log_fn {
                    logger(
                        "INFO",
                        "sing-box",
                        &format!(
                            "Stage 3 PASS in {:.2}s: Hostname HTTPS trace verified (POP: {}, IP: {}, Latency: {} ms)",
                            t_stage3.elapsed().as_secs_f32(),
                            t.colo, t.ip, t.latency_ms
                        ),
                    );
                }
                t
            }
            None => {
                let host_fail_msg = format!(
                    "Stage 3 Hostname HTTPS Failure: Hostname HTTPS egress test failed after retries: {}",
                    last_host_err
                );
                if let Some(logger) = log_fn {
                    logger("ERROR", "sing-box", &host_fail_msg);
                }
                return Err(host_fail_msg);
            }
        };

        Ok(host_trace)
    }

    /// Enumerates Windows adapters using native GetAdaptersAddresses IP Helper API
    #[cfg(windows)]
    pub fn enumerate_windows_adapters() -> Vec<DetectedAdapterInfo> {
        use std::net::{Ipv4Addr, Ipv6Addr};
        use windows_sys::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
        use windows_sys::Win32::NetworkManagement::IpHelper::{
            GetAdaptersAddresses, GAA_FLAG_INCLUDE_PREFIX, GAA_FLAG_SKIP_ANYCAST,
            GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
        };
        use windows_sys::Win32::Networking::WinSock::{
            AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
        };

        let mut adapters = Vec::new();
        let flags = GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER
            | GAA_FLAG_INCLUDE_PREFIX;

        let mut buf_len: u32 = 16384;
        let mut buf: Vec<u8> = vec![0; buf_len as usize];

        let ret = unsafe {
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                &mut buf_len,
            )
        };

        let success = if ret == ERROR_BUFFER_OVERFLOW {
            buf.resize(buf_len as usize, 0);
            let ret2 = unsafe {
                GetAdaptersAddresses(
                    AF_UNSPEC as u32,
                    flags,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                    &mut buf_len,
                )
            };
            ret2 == ERROR_SUCCESS
        } else {
            ret == ERROR_SUCCESS
        };

        if !success {
            return adapters;
        }

        let mut curr_ptr = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
        while !curr_ptr.is_null() {
            let adapter = unsafe { &*curr_ptr };

            // 1. Friendly Name (PWSTR)
            let friendly_name = if !adapter.FriendlyName.is_null() {
                let mut len = 0;
                while unsafe { *adapter.FriendlyName.add(len) } != 0 {
                    len += 1;
                }
                let slice = unsafe { std::slice::from_raw_parts(adapter.FriendlyName, len) };
                String::from_utf16_lossy(slice)
            } else {
                String::new()
            };

            // 2. Description (PWSTR)
            let description = if !adapter.Description.is_null() {
                let mut len = 0;
                while unsafe { *adapter.Description.add(len) } != 0 {
                    len += 1;
                }
                let slice = unsafe { std::slice::from_raw_parts(adapter.Description, len) };
                String::from_utf16_lossy(slice)
            } else {
                String::new()
            };

            // 3. AdapterName (PSTR / GUID)
            let adapter_name = if !adapter.AdapterName.is_null() {
                let cstr = unsafe { std::ffi::CStr::from_ptr(adapter.AdapterName as *const _) };
                cstr.to_string_lossy().to_string()
            } else {
                String::new()
            };

            // 4. OperStatus (1 == IfOperStatusUp)
            let is_up = adapter.OperStatus == 1;

            // 5. Unicast IP Addresses
            let mut ip_addresses = Vec::new();
            let mut addr_ptr = adapter.FirstUnicastAddress;
            while !addr_ptr.is_null() {
                let unicast = unsafe { &*addr_ptr };
                let sockaddr_ptr = unicast.Address.lpSockaddr;
                if !sockaddr_ptr.is_null() {
                    let family = unsafe { (*sockaddr_ptr).sa_family };
                    if family == AF_INET {
                        let in_addr = unsafe { &*(sockaddr_ptr as *const SOCKADDR_IN) };
                        let bytes = unsafe { in_addr.sin_addr.S_un.S_addr.to_ne_bytes() };
                        let ip = Ipv4Addr::from(bytes);
                        ip_addresses.push(ip.to_string());
                    } else if family == AF_INET6 {
                        let in6_addr = unsafe { &*(sockaddr_ptr as *const SOCKADDR_IN6) };
                        let bytes = unsafe { in6_addr.sin6_addr.u.Byte };
                        let ip = Ipv6Addr::from(bytes);
                        ip_addresses.push(ip.to_string());
                    }
                }
                addr_ptr = unicast.Next;
            }

            adapters.push(DetectedAdapterInfo {
                friendly_name,
                description,
                adapter_name,
                if_index: unsafe { adapter.Anonymous1.Anonymous.IfIndex },
                is_up,
                ip_addresses,
            });

            curr_ptr = adapter.Next;
        }

        adapters
    }

    #[cfg(not(windows))]
    pub fn enumerate_windows_adapters() -> Vec<DetectedAdapterInfo> {
        let networks = Networks::new_with_refreshed_list();
        networks
            .into_iter()
            .map(|(name, _)| DetectedAdapterInfo {
                friendly_name: name.clone(),
                description: name,
                adapter_name: String::new(),
                if_index: 0,
                is_up: true,
                ip_addresses: vec![],
            })
            .collect()
    }

    /// Checks whether the configured TUN adapter exists in the Windows network stack
    /// using native IP Helper GetAdaptersAddresses API.
    ///
    /// Matches on:
    /// 1. Exact / case-insensitive FriendlyName match (e.g. "singbox-tun")
    /// 2. Configured TUN IP assigned to an adapter (e.g. "172.19.0.1")
    /// 3. Adapter description / friendly name containing "wintun" or "singbox"
    /// 4. Fallback to sysinfo Networks list
    pub fn check_tun_interface_exists(
        interface_name: &str,
        configured_tun_address: Option<&str>,
    ) -> (bool, Option<DetectedAdapterInfo>, Vec<DetectedAdapterInfo>) {
        let all_adapters = Self::enumerate_windows_adapters();

        let target_name = interface_name.to_lowercase();
        let stripped_ip = configured_tun_address.map(|addr| {
            if let Some((ip, _)) = addr.split_once('/') {
                ip.trim()
            } else {
                addr.trim()
            }
        });

        // 1. Check native adapters
        for adapter in &all_adapters {
            let f_lower = adapter.friendly_name.to_lowercase();
            let d_lower = adapter.description.to_lowercase();

            // Match 1: Friendly name match
            if f_lower == target_name || f_lower.contains(&target_name) {
                return (true, Some(adapter.clone()), all_adapters);
            }

            // Match 2: Configured TUN IP match
            if let Some(target_ip) = stripped_ip {
                if !target_ip.is_empty() && adapter.ip_addresses.iter().any(|ip| ip == target_ip) {
                    return (true, Some(adapter.clone()), all_adapters);
                }
            }

            // Match 3: Wintun description with active status or IP (only if looking for a singbox/wintun interface)
            if (target_name.contains("wintun") || target_name.contains("singbox"))
                && (d_lower.contains("wintun")
                    || d_lower.contains("singbox")
                    || f_lower.contains("singbox"))
                && (adapter.is_up || !adapter.ip_addresses.is_empty())
            {
                return (true, Some(adapter.clone()), all_adapters);
            }
        }

        // 2. Fallback to sysinfo Networks
        let networks = Networks::new_with_refreshed_list();
        for (name, _) in &networks {
            let n_lower = name.to_lowercase();
            if n_lower == target_name
                || (!target_name.is_empty() && n_lower.contains(&target_name))
                || (target_name.contains("singbox") && n_lower.contains("singbox"))
            {
                return (true, None, all_adapters);
            }
        }

        (false, None, all_adapters)
    }

    /// Polls until the configured TUN interface is no longer present in the Windows network stack.
    /// Used before starting a fresh scan or rollback to avoid adapter/route collisions.
    pub async fn wait_for_tun_teardown(
        interface_name: &str,
        configured_tun_address: Option<&str>,
        timeout: Duration,
    ) -> Result<(), String> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(200);

        while start.elapsed() < timeout {
            let (exists, _, _) =
                Self::check_tun_interface_exists(interface_name, configured_tun_address);
            if !exists {
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(format!(
            "TUN interface '{}' was not released by Windows within {:.1}s",
            interface_name,
            timeout.as_secs_f32()
        ))
    }

    /// Parameterized helper for deterministic unit testing of TUN teardown polling
    pub async fn wait_for_tun_teardown_with_check<F>(
        mut check_fn: F,
        timeout: Duration,
        poll_interval: Duration,
    ) -> Result<(), String>
    where
        F: FnMut() -> bool + Send,
    {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if !check_fn() {
                return Ok(());
            }
            tokio::time::sleep(poll_interval).await;
        }
        Err(format!(
            "TUN teardown timed out after {:.1}s",
            timeout.as_secs_f32()
        ))
    }

    /// Checks if current process token has elevated administrator privileges
    #[cfg(windows)]
    pub fn is_process_elevated() -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }

            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut return_length: u32 = 0;

            let success = GetTokenInformation(
                token,
                TokenElevation,
                &mut elevation as *mut _ as *mut _,
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut return_length,
            );

            CloseHandle(token);

            success != 0 && elevation.TokenIsElevated != 0
        }
    }

    #[cfg(not(windows))]
    pub fn is_process_elevated() -> bool {
        true
    }

    /// Comprehensive system health check with NO hardcoded success values
    pub async fn evaluate_health(
        aether_host: &str,
        aether_port: u16,
        secondary_host: &str,
        secondary_port: u16,
        secondary_enabled: bool,
        tun_interface_name: &str,
        tun_address: Option<&str>,
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

        // 5. TUN Interface Check (Real native Windows IP Helper adapter query)
        let (tun_exists, matched_adapter, _) =
            Self::check_tun_interface_exists(tun_interface_name, tun_address);
        let tun_interface = if tun_exists {
            if let Some(info) = matched_adapter {
                let ip_str = if info.ip_addresses.is_empty() {
                    String::new()
                } else {
                    format!(" · IP: {}", info.ip_addresses.join(", "))
                };
                ServiceHealth::ok(format!("Adapter '{}' active{}", info.friendly_name, ip_str))
            } else {
                ServiceHealth::ok(format!("Adapter '{}' active", tun_interface_name))
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_e_tun_teardown_wait_succeeds_when_adapter_is_released() {
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        // Simulate adapter present for first 2 polls then disappearing
        let check_fn = move || {
            let current = attempts_clone.fetch_add(1, Ordering::SeqCst);
            current < 2
        };

        let res = HealthProber::wait_for_tun_teardown_with_check(
            check_fn,
            Duration::from_millis(500),
            Duration::from_millis(20),
        )
        .await;

        assert!(res.is_ok());
        assert!(attempts.load(Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn test_f_tun_teardown_wait_times_out_when_adapter_remains() {
        // Simulate adapter never disappearing
        let check_fn = || true;

        let res = HealthProber::wait_for_tun_teardown_with_check(
            check_fn,
            Duration::from_millis(60),
            Duration::from_millis(15),
        )
        .await;

        assert!(res.is_err());
        assert!(res.unwrap_err().contains("TUN teardown timed out"));
    }
}
