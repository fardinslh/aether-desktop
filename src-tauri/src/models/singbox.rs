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
    #[serde(rename = "vless")]
    Vless(VlessOutbound),
    #[serde(rename = "vmess")]
    Vmess(VmessOutbound),
    #[serde(rename = "trojan")]
    Trojan(TrojanOutbound),
    #[serde(rename = "shadowsocks")]
    Shadowsocks(ShadowsocksOutbound),
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
pub struct VlessOutbound {
    pub tag: String,
    pub server: String,
    pub server_port: u16,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<OutboundTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmessOutbound {
    pub tag: String,
    pub server: String,
    pub server_port: u16,
    pub uuid: String,
    pub security: String,
    pub alter_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<OutboundTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrojanOutbound {
    pub tag: String,
    pub server: String,
    pub server_port: u16,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<OutboundTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowsocksOutbound {
    pub tag: String,
    pub server: String,
    pub server_port: u16,
    pub method: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_opts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutboundTls {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub insecure: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alpn: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utls: Option<OutboundUtls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality: Option<OutboundReality>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutboundUtls {
    pub enabled: bool,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutboundReality {
    pub enabled: bool,
    pub public_key: String,
    pub short_id: String,
}

fn is_false(value: &bool) -> bool {
    !*value
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
