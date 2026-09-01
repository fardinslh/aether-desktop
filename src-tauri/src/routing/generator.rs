use crate::models::settings::{CompatibilityScope, NetworkProtocol};
use crate::models::singbox::{
    DirectOutbound, DnsConfig, DnsServer, InboundConfig, LocalDnsServer, LogConfig,
    OutboundConfig, RouteConfig, RouteRule, SingBoxConfig, SocksOutbound, TunInbound,
};
use crate::models::{AppSettings, RouteDestination, RulePriority};
use crate::routing::presets::{GENERALS_STUN_TURN_PORTS, LOOP_PREVENTION_PROCESSES};

pub struct SingBoxConfigGenerator;

impl SingBoxConfigGenerator {
    /// Generates a complete sing-box configuration from AppSettings following verified strict precedence rules.
    ///
    /// PRECEDENCE HIERARCHY:
    /// 0. DNS INFRASTRUCTURE HIJACK: DNS protocol & port 53 -> hijack-dns (intercepted into sing-box DNS engine)
    /// 1. CORE LOOP PREVENTION: Proxy binaries (aether.exe, xray.exe, v2ray.exe, v2rayN.exe) -> DIRECT
    /// 2. APP-SCOPED COMPATIBILITY: Specific overrides requiring (Process + Ports/Protocol) -> Destination
    /// 3. HIGH-PRIORITY APPLICATION OVERRIDES: (e.g. Discord.exe -> aether)
    /// 4. GENERIC COMPATIBILITY FALLBACK: Generals Online STUN/TURN (ports 3478, 5349) -> DIRECT
    /// 5. NORMAL-PRIORITY APPLICATION ROUTING:
    ///    - Normal Direct apps (dota2.exe, Rust.exe, etc.) -> DIRECT
    ///    - Normal Secondary Proxy apps (chrome.exe, Code.exe, Antigravity.exe, agy.exe, Spotify.exe, etc.) -> v2ray
    ///    - Normal Aether apps -> aether
    /// 6. PRIVATE NETWORK / LAN: ip_is_private: true -> DIRECT (Non-DNS private traffic bypasses proxy)
    /// 7. FINAL FALLBACK: All other traffic -> aether
    pub fn generate(settings: &AppSettings) -> SingBoxConfig {
        // 1. Log configuration
        let log = LogConfig {
            level: settings.sing_box.log_level.clone(),
            timestamp: true,
        };

        // 2. DNS configuration (Remote UDP/TCP DNS over Aether, Local fallback over Direct)
        let dns = Some(DnsConfig {
            servers: vec![
                DnsServer::Udp(crate::models::singbox::UdpDnsServer {
                    tag: "remote-dns".to_string(),
                    server: "1.1.1.1".to_string(),
                    server_port: Some(53),
                    detour: Some("aether".to_string()),
                }),
                DnsServer::Tcp(crate::models::singbox::TcpDnsServer {
                    tag: "remote-dns-tcp".to_string(),
                    server: "1.1.1.1".to_string(),
                    server_port: Some(53),
                    detour: Some("aether".to_string()),
                }),
                DnsServer::Udp(crate::models::singbox::UdpDnsServer {
                    tag: "remote-dns-backup".to_string(),
                    server: "1.0.0.1".to_string(),
                    server_port: Some(53),
                    detour: Some("aether".to_string()),
                }),
                DnsServer::Local(LocalDnsServer {
                    tag: "local-dns".to_string(),
                    detour: Some("direct".to_string()),
                }),
            ],
            strategy: "prefer_ipv4".to_string(),
            independent_cache: true,
        });

        // 3. TUN Inbound
        let inbounds = vec![InboundConfig::Tun(TunInbound {
            tag: "tun-in".to_string(),
            interface_name: settings.sing_box.interface_name.clone(),
            address: vec![settings.sing_box.tun_address.clone()],
            mtu: settings.sing_box.mtu,
            auto_route: true,
            strict_route: settings.sing_box.strict_route,
            stack: "system".to_string(),
        })];

        // 4. Outbounds: Aether (primary socks), V2Ray (secondary socks), and Direct
        let outbounds = vec![
            OutboundConfig::Socks(SocksOutbound {
                tag: "aether".to_string(),
                server: settings.aether.host.clone(),
                server_port: settings.aether.port,
                version: "5".to_string(),
            }),
            OutboundConfig::Socks(SocksOutbound {
                tag: "v2ray".to_string(),
                server: settings.secondary_proxy.host.clone(),
                server_port: settings.secondary_proxy.port,
                version: "5".to_string(),
            }),
            OutboundConfig::Direct(DirectOutbound {
                tag: "direct".to_string(),
            }),
        ];

        // 5. Build routing rules with strict precedence:
        let mut rules: Vec<RouteRule> = Vec::new();

        // 5.0 Priority 0: DNS Infrastructure Hijack -> hijack-dns (Intercepts DNS queries before private IP rule)
        rules.push(RouteRule {
            protocol: Some(vec!["dns".to_string()]),
            process_name: None,
            port: None,
            port_range: None,
            network: None,
            ip_is_private: None,
            action: Some("hijack-dns".to_string()),
            outbound: None,
        });
        rules.push(RouteRule {
            protocol: None,
            process_name: None,
            port: Some(vec![53]),
            port_range: None,
            network: None,
            ip_is_private: None,
            action: Some("hijack-dns".to_string()),
            outbound: None,
        });

        // 5.1 Priority 1: Core Process Loop Prevention -> DIRECT (Always top priority)
        let loop_processes: Vec<String> = LOOP_PREVENTION_PROCESSES
            .iter()
            .map(|s| s.to_string())
            .collect();
        rules.push(RouteRule {
            protocol: None,
            process_name: Some(loop_processes),
            port: None,
            port_range: None,
            network: None,
            ip_is_private: None,
            action: Some("route".to_string()),
            outbound: Some("direct".to_string()),
        });

        // 5.2 Priority 2: App-Scoped Compatibility Overrides
        for compat_rule in &settings.compatibility.custom_compatibility_rules {
            if !compat_rule.enabled || compat_rule.scope != CompatibilityScope::AppScoped {
                continue;
            }
            if let Some(ref procs) = compat_rule.process_names {
                if !procs.is_empty() {
                    let network_str = match compat_rule.network {
                        Some(NetworkProtocol::Tcp) => Some("tcp".to_string()),
                        Some(NetworkProtocol::Udp) => Some("udp".to_string()),
                        _ => None,
                    };
                    rules.push(RouteRule {
                        protocol: None,
                        process_name: Some(procs.clone()),
                        port: compat_rule.ports.clone(),
                        port_range: compat_rule.port_ranges.clone(),
                        network: network_str,
                        ip_is_private: None,
                        action: Some("route".to_string()),
                        outbound: Some(Self::outbound_tag_for_destination(
                            &compat_rule.destination,
                        )),
                    });
                }
            }
        }

        // Partition enabled application rules into High and Normal priority
        let mut high_direct_apps: Vec<String> = Vec::new();
        let mut high_v2ray_apps: Vec<String> = Vec::new();
        let mut high_aether_apps: Vec<String> = Vec::new();

        let mut normal_direct_apps: Vec<String> = Vec::new();
        let mut normal_v2ray_apps: Vec<String> = Vec::new();
        let mut normal_aether_apps: Vec<String> = Vec::new();

        for rule in &settings.application_rules {
            if !rule.enabled {
                continue;
            }

            let proc = rule.process_name.trim().to_string();
            if proc.is_empty() {
                continue;
            }

            if rule.priority == RulePriority::High {
                match rule.destination {
                    RouteDestination::Direct => {
                        if !high_direct_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            high_direct_apps.push(proc);
                        }
                    }
                    RouteDestination::SecondaryProxy => {
                        if !high_v2ray_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            high_v2ray_apps.push(proc);
                        }
                    }
                    RouteDestination::Aether => {
                        if !high_aether_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            high_aether_apps.push(proc);
                        }
                    }
                }
            } else {
                match rule.destination {
                    RouteDestination::Direct => {
                        if !normal_direct_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            normal_direct_apps.push(proc);
                        }
                    }
                    RouteDestination::SecondaryProxy => {
                        if !normal_v2ray_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            normal_v2ray_apps.push(proc);
                        }
                    }
                    RouteDestination::Aether => {
                        if !normal_aether_apps
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&proc))
                        {
                            normal_aether_apps.push(proc);
                        }
                    }
                }
            }
        }

        // Auto-propagate Steam companion processes (steamwebhelper.exe, steamservice.exe)
        // if steam.exe is routed to a destination and companions are not explicitly routed elsewhere.
        let all_configured_apps: Vec<String> = high_direct_apps
            .iter()
            .chain(high_v2ray_apps.iter())
            .chain(high_aether_apps.iter())
            .chain(normal_direct_apps.iter())
            .chain(normal_v2ray_apps.iter())
            .chain(normal_aether_apps.iter())
            .map(|s| s.to_lowercase())
            .collect();

        let steam_companions = ["steamwebhelper.exe", "steamservice.exe"];

        if all_configured_apps.contains(&"steam.exe".to_string()) {
            if normal_v2ray_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        normal_v2ray_apps.push(comp.to_string());
                    }
                }
            } else if normal_aether_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        normal_aether_apps.push(comp.to_string());
                    }
                }
            } else if normal_direct_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        normal_direct_apps.push(comp.to_string());
                    }
                }
            } else if high_v2ray_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        high_v2ray_apps.push(comp.to_string());
                    }
                }
            } else if high_aether_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        high_aether_apps.push(comp.to_string());
                    }
                }
            } else if high_direct_apps.iter().any(|p| p.eq_ignore_ascii_case("steam.exe")) {
                for comp in &steam_companions {
                    if !all_configured_apps.contains(&comp.to_lowercase()) {
                        high_direct_apps.push(comp.to_string());
                    }
                }
            }
        }

        // 5.3 Priority 3: HIGH-PRIORITY Application Overrides (e.g. Discord.exe -> aether)
        if !high_direct_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(high_direct_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("direct".to_string()),
            });
        }
        if !high_v2ray_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(high_v2ray_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("v2ray".to_string()),
            });
        }
        if !high_aether_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(high_aether_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("aether".to_string()),
            });
        }

        // 5.4 Priority 4: Generic Compatibility Fallback Rules (Generals Online STUN/TURN fallback)
        // 5.4.1 Custom Global Fallback rules
        for compat_rule in &settings.compatibility.custom_compatibility_rules {
            if !compat_rule.enabled || compat_rule.scope != CompatibilityScope::GlobalFallback {
                continue;
            }
            let network_str = match compat_rule.network {
                Some(NetworkProtocol::Tcp) => Some("tcp".to_string()),
                Some(NetworkProtocol::Udp) => Some("udp".to_string()),
                _ => None,
            };
            rules.push(RouteRule {
                protocol: None,
                process_name: compat_rule.process_names.clone(),
                port: compat_rule.ports.clone(),
                port_range: compat_rule.port_ranges.clone(),
                network: network_str,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some(Self::outbound_tag_for_destination(&compat_rule.destination)),
            });
        }

        // 5.4.2 Generals Online STUN/TURN fallback: ports 3478, 5349 -> DIRECT
        if settings.compatibility.generals_stun_turn_fallback {
            rules.push(RouteRule {
                protocol: None,
                process_name: None,
                port: Some(GENERALS_STUN_TURN_PORTS.to_vec()),
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("direct".to_string()),
            });
        }

        // 5.5 Priority 5: NORMAL-PRIORITY Application Routing
        // 5.5.1 NORMAL DIRECT applications (dota2.exe, Rust.exe, etc.)
        if !normal_direct_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(normal_direct_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("direct".to_string()),
            });
        }

        // 5.5.2 NORMAL SECONDARY PROXY applications (chrome.exe, Code.exe, Antigravity.exe, agy.exe, Spotify.exe, etc.)
        if !normal_v2ray_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(normal_v2ray_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("v2ray".to_string()),
            });
        }

        // 5.5.3 NORMAL AETHER explicit applications
        if !normal_aether_apps.is_empty() {
            rules.push(RouteRule {
                protocol: None,
                process_name: Some(normal_aether_apps),
                port: None,
                port_range: None,
                network: None,
                ip_is_private: None,
                action: Some("route".to_string()),
                outbound: Some("aether".to_string()),
            });
        }

        // 5.6 Priority 6: Private IP network bypass -> DIRECT
        if settings.compatibility.private_ip_bypass {
            rules.push(RouteRule {
                protocol: None,
                process_name: None,
                port: None,
                port_range: None,
                network: None,
                ip_is_private: Some(true),
                action: Some("route".to_string()),
                outbound: Some("direct".to_string()),
            });
        }

        // 6. Final route configuration
        let route = RouteConfig {
            auto_detect_interface: true,
            default_domain_resolver: Some("remote-dns".to_string()),
            rules,
            final_outbound: "aether".to_string(),
        };

        SingBoxConfig {
            log,
            dns,
            inbounds,
            outbounds,
            route,
        }
    }

    fn outbound_tag_for_destination(dest: &RouteDestination) -> String {
        match dest {
            RouteDestination::Direct => "direct".to_string(),
            RouteDestination::SecondaryProxy => "v2ray".to_string(),
            RouteDestination::Aether => "aether".to_string(),
        }
    }

    /// Serializes configuration to pretty-printed JSON
    pub fn to_json_string(config: &SingBoxConfig) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(config)
    }

    /// Helper to evaluate semantic top-down routing resolution for a packet
    pub fn resolve_route<'a>(
        config: &'a SingBoxConfig,
        process_name: Option<&str>,
        port: Option<u16>,
        is_private: bool,
    ) -> &'a str {
        Self::resolve_route_full(config, process_name, port, None, is_private)
    }

    /// Helper that evaluates routing resolution with explicit network protocol (e.g. "udp" or "tcp")
    pub fn resolve_route_with_network<'a>(
        config: &'a SingBoxConfig,
        process_name: Option<&str>,
        port: Option<u16>,
        network: Option<&str>,
        is_private: bool,
    ) -> &'a str {
        Self::resolve_route_full(config, process_name, port, network, is_private)
    }

    /// Full semantic top-down routing resolution for a packet
    pub fn resolve_route_full<'a>(
        config: &'a SingBoxConfig,
        process_name: Option<&str>,
        port: Option<u16>,
        network: Option<&str>,
        is_private: bool,
    ) -> &'a str {
        for rule in &config.route.rules {
            // Check hijack-dns infrastructure rule
            if rule.action.as_deref() == Some("hijack-dns") {
                let port_matches = match rule.port {
                    Some(ref ports) => port.map(|p| ports.contains(&p)).unwrap_or(false),
                    None => false,
                };
                if port_matches {
                    return "dns-hijack";
                }
                continue;
            }

            // Check IP private match
            if let Some(true) = rule.ip_is_private {
                if is_private {
                    return rule
                        .outbound
                        .as_deref()
                        .unwrap_or(&config.route.final_outbound);
                } else {
                    continue;
                }
            }

            // Check process match if rule requires it (case-insensitive on Windows)
            let proc_matches = match (&rule.process_name, process_name) {
                (Some(ref procs), Some(p)) => procs.iter().any(|item| item.eq_ignore_ascii_case(p)),
                (Some(_), None) => false,
                (None, _) => true,
            };

            // Check network match if rule requires it (e.g. "udp" vs "tcp")
            let network_matches = match (&rule.network, network) {
                (Some(rule_net), Some(req_net)) => rule_net.eq_ignore_ascii_case(req_net),
                (Some(_), None) => true,
                (None, _) => true,
            };

            // Check port match if rule requires it (either port list or port_range)
            let port_matches = match (port, &rule.port, &rule.port_range) {
                (Some(dst_port), Some(ports), _) if ports.contains(&dst_port) => true,
                (Some(dst_port), _, Some(ranges)) => {
                    ranges.iter().any(|range_str| {
                        if let Some((start_s, end_s)) = range_str.split_once(':') {
                            if let (Ok(start), Ok(end)) = (start_s.parse::<u16>(), end_s.parse::<u16>()) {
                                return dst_port >= start && dst_port <= end;
                            }
                        }
                        false
                    })
                }
                (None, Some(_), _) | (None, _, Some(_)) => false,
                (_, None, None) => true,
                _ => false,
            };

            if proc_matches && network_matches && port_matches {
                return rule
                    .outbound
                    .as_deref()
                    .unwrap_or(&config.route.final_outbound);
            }
        }

        &config.route.final_outbound
    }

    /// Helper that resolves routes given an IP string, automatically determining if it is a private IP
    pub fn resolve_route_with_ip<'a>(
        config: &'a SingBoxConfig,
        process_name: Option<&str>,
        port: Option<u16>,
        ip_str: Option<&str>,
    ) -> &'a str {
        let is_priv = match ip_str {
            Some(ip) => Self::is_private_ip(ip),
            None => false,
        };
        Self::resolve_route(config, process_name, port, is_priv)
    }

    /// Helper to check if an IP string belongs to RFC 1918 / loopback / link-local private ranges
    pub fn is_private_ip(ip_str: &str) -> bool {
        if let Ok(ip) = ip_str.parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(ipv4) => {
                    ipv4.is_private() || ipv4.is_loopback() || ipv4.is_link_local()
                }
                std::net::IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unicast_link_local(),
            }
        } else {
            false
        }
    }
}
