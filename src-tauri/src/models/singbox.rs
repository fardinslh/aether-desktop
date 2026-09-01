use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SingBoxConfig {
    pub log: LogConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
    pub inbounds: Vec<InboundConfig>,
    pub outbounds: Vec<OutboundConfig>,
    pub route: RouteConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogConfig {
    pub level: String,
    pub timestamp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DnsConfig {
    pub servers: Vec<DnsServer>,
    pub strategy: String,
    pub independent_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DnsServer {
    #[serde(rename = "udp")]
    Udp(UdpDnsServer),
    #[serde(rename = "tcp")]
    Tcp(TcpDnsServer),
    #[serde(rename = "https")]
    Https(HttpsDnsServer),
    #[serde(rename = "local")]
    Local(LocalDnsServer),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UdpDnsServer {
    pub tag: String,
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TcpDnsServer {
    pub tag: String,
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpsDnsServer {
    pub tag: String,
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalDnsServer {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detour: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum InboundConfig {
    #[serde(rename = "tun")]
    Tun(TunInbound),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TunInbound {
    pub tag: String,
    pub interface_name: String,
    pub address: Vec<String>,
    pub mtu: u32,
    pub auto_route: bool,
    pub strict_route: bool,
    pub stack: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum OutboundConfig {
    #[serde(rename = "socks")]
    Socks(SocksOutbound),
    #[serde(rename = "direct")]
    Direct(DirectOutbound),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocksOutbound {
    pub tag: String,
    pub server: String,
    pub server_port: u16,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectOutbound {
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteConfig {
    pub auto_detect_interface: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_domain_resolver: Option<String>,
    pub rules: Vec<RouteRule>,
    #[serde(rename = "final")]
    pub final_outbound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<Vec<u16>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_range: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_is_private: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbound: Option<String>,
}
