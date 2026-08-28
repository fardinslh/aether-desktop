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

    /// Performs direct system Cloudflare trace probe (without SOCKS proxy) through Windows network stack/TUN adapter
    pub async fn query_direct_system_cloudflare_trace() -> Result<CloudflareTrace, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("Failed to build HTTP client for system egress test: {}", e))?;

        let start = Instant::now();
        let resp = client
            .get("https://www.cloudflare.com/cdn-cgi/trace")
            .send()
            .await
            .map_err(|e| format!("Direct system trace request failed: {}", e))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !resp.status().is_success() {
            return Err(format!(
                "Direct trace returned HTTP status {}",
                resp.status()
            ));
        }

        let body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read trace response body: {}", e))?;

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
            return Err("Invalid direct trace response: missing IP address".to_string());
        }

        Ok(CloudflareTrace {
            ip,
            warp,
            colo,
            loc,
            latency_ms,
        })
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

            // Match 3: Wintun description with active status or IP
            if (d_lower.contains("wintun")
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
                || n_lower.contains(&target_name)
                || n_lower.contains("singbox")
            {
                return (true, None, all_adapters);
            }
        }

        (false, None, all_adapters)
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
