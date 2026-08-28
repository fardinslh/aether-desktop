use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SingBoxConfig {
    pub log: LogConfig,
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
    pub rules: Vec<RouteRule>,
    #[serde(rename = "final")]
    pub final_outbound: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<Vec<u16>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_is_private: Option<bool>,

    pub action: String,
    pub outbound: String,
}