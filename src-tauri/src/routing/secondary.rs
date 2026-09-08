use crate::models::singbox::{
    OutboundConfig, OutboundReality, OutboundTls, OutboundUtls, ShadowsocksOutbound,
    TrojanOutbound, VlessOutbound, VmessOutbound,
};
use crate::models::settings::SecondaryProxyProfile;
use base64::Engine;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use url::Url;

const SECONDARY_TAG: &str = "v2ray";
const MAX_SUBSCRIPTION_PROFILES: usize = 200;

pub struct ParsedSubscription {
    pub profiles: Vec<SecondaryProxyProfile>,
    pub skipped: usize,
}

pub fn parse_share_link(input: &str) -> Result<OutboundConfig, String> {
    let link = input.trim();
    if link.is_empty() {
        return Err("Paste a VLESS, VMess, Trojan, or Shadowsocks share link.".to_string());
    }
    if link.lines().count() != 1 {
        return Err("Paste one secondary proxy share link at a time.".to_string());
    }

    if link.starts_with("vless://") {
        parse_vless(link)
    } else if link.starts_with("vmess://") {
        parse_vmess(link)
    } else if link.starts_with("trojan://") {
        parse_trojan(link)
    } else if link.starts_with("ss://") {
        parse_shadowsocks(link)
    } else {
        Err(
            "Unsupported config. Use a vless://, vmess://, trojan://, or ss:// share link."
                .to_string(),
        )
    }
}

pub fn parse_subscription_payload(payload: &str) -> Result<ParsedSubscription, String> {
    let raw = payload.trim().trim_start_matches('\u{feff}');
    if raw.is_empty() {
        return Err("Subscription returned an empty response.".to_string());
    }

    let decoded;
    let profile_text = if raw.contains("://") {
        raw
    } else {
        let compact: String = raw.chars().filter(|character| !character.is_whitespace()).collect();
        decoded = String::from_utf8(
            decode_base64(&compact)
                .map_err(|_| "Subscription is neither a share-link list nor valid base64.".to_string())?,
        )
        .map_err(|_| "Decoded subscription is not valid UTF-8 text.".to_string())?;
        decoded.trim().trim_start_matches('\u{feff}')
    };

    let mut profiles = Vec::new();
    let mut seen = HashSet::new();
    let mut skipped = 0usize;
    let mut first_error = None;

    for raw_line in profile_text.lines() {
        let link = raw_line.trim().trim_start_matches("- ").trim();
        if link.is_empty() || link.starts_with('#') {
            continue;
        }
        if !link.contains("://") {
            continue;
        }
        if link.len() > 65_536 {
            skipped += 1;
            first_error.get_or_insert_with(|| "A subscription entry is too large.".to_string());
            continue;
        }
        if !seen.insert(link.to_string()) {
            continue;
        }

        match parse_share_link(link) {
            Ok(_) => profiles.push(SecondaryProxyProfile {
                name: share_link_name(link, profiles.len() + 1),
                share_link: link.to_string(),
            }),
            Err(error) => {
                skipped += 1;
                first_error.get_or_insert(error);
            }
        }
    }

    if profiles.is_empty() {
        return Err(first_error.unwrap_or_else(|| {
            "Subscription contains no supported VLESS, VMess, Trojan, or Shadowsocks profiles."
                .to_string()
        }));
    }
    if profiles.len() > MAX_SUBSCRIPTION_PROFILES {
        return Err(format!(
            "Subscription contains {} supported profiles; maximum is {MAX_SUBSCRIPTION_PROFILES}.",
            profiles.len()
        ));
    }

    Ok(ParsedSubscription { profiles, skipped })
}

fn share_link_name(link: &str, index: usize) -> String {
    let explicit_name = if let Some(encoded) = link.strip_prefix("vmess://") {
        decode_base64(encoded)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|value| value.get("ps").and_then(Value::as_str).map(str::to_string))
    } else {
        Url::parse(link)
            .ok()
            .and_then(|url| url.fragment().map(decode))
    };

    if let Some(name) = explicit_name.filter(|name| !name.trim().is_empty()) {
        return name.trim().chars().take(120).collect();
    }

    let protocol = link
        .split_once("://")
        .map(|(protocol, _)| protocol.to_ascii_uppercase())
        .unwrap_or_else(|| "Proxy".to_string());
    let endpoint = Url::parse(link).ok().and_then(|url| {
        url.host_str().map(|host| match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        })
    });
    endpoint.map_or_else(
        || format!("{protocol} {index}"),
        |endpoint| format!("{protocol} · {endpoint}"),
    )
}

fn parse_vless(link: &str) -> Result<OutboundConfig, String> {
    let url = Url::parse(link).map_err(|error| format!("Invalid VLESS link: {error}"))?;
    let server = required_host(&url, "VLESS")?;
    let server_port = required_port(&url, "VLESS")?;
    let uuid = decode(url.username());
    uuid::Uuid::parse_str(&uuid).map_err(|_| "VLESS link contains an invalid UUID.".to_string())?;
    let query = query_map(&url);

    if query.get("encryption").is_some_and(|value| value != "none") {
        return Err("Only VLESS encryption=none is supported by sing-box.".to_string());
    }

    let security = query.get("security").map(String::as_str).unwrap_or("none");
    let tls = tls_from_query(security, &server, &query)?;
    let transport = transport_from_values(
        query.get("type").map(String::as_str),
        query.get("host").map(String::as_str),
        query.get("path").map(String::as_str),
        query
            .get("servicename")
            .or_else(|| query.get("service_name"))
            .map(String::as_str),
        query.get("headertype").map(String::as_str),
    )?;

    Ok(OutboundConfig::Vless(VlessOutbound {
        tag: SECONDARY_TAG.to_string(),
        server,
        server_port,
        uuid,
        flow: nonempty(query.get("flow").cloned()),
        packet_encoding: nonempty(query.get("packetencoding").cloned()),
        tls,
        transport,
    }))
}

fn parse_trojan(link: &str) -> Result<OutboundConfig, String> {
    let url = Url::parse(link).map_err(|error| format!("Invalid Trojan link: {error}"))?;
    let server = required_host(&url, "Trojan")?;
    let server_port = required_port(&url, "Trojan")?;
    let password = decode(url.username());
    if password.is_empty() {
        return Err("Trojan link is missing its password.".to_string());
    }
    let query = query_map(&url);
    let security = query.get("security").map(String::as_str).unwrap_or("tls");
    let tls = tls_from_query(security, &server, &query)?;
    let transport = transport_from_values(
        query.get("type").map(String::as_str),
        query.get("host").map(String::as_str),
        query.get("path").map(String::as_str),
        query
            .get("servicename")
            .or_else(|| query.get("service_name"))
            .map(String::as_str),
        query.get("headertype").map(String::as_str),
    )?;

    Ok(OutboundConfig::Trojan(TrojanOutbound {
        tag: SECONDARY_TAG.to_string(),
        server,
        server_port,
        password,
        tls,
        transport,
    }))
}

#[derive(Debug, Deserialize)]
struct VmessLink {
    add: String,
    port: Value,
    id: String,
    #[serde(default)]
    aid: Value,
    #[serde(default)]
    scy: String,
    #[serde(default)]
    net: String,
    #[serde(default)]
    host: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    tls: String,
    #[serde(default)]
    sni: String,
    #[serde(default)]
    alpn: String,
    #[serde(default)]
    fp: String,
    #[serde(default, rename = "type")]
    header_type: String,
}

fn parse_vmess(link: &str) -> Result<OutboundConfig, String> {
    let encoded = link.trim_start_matches("vmess://");
    let bytes = decode_base64(encoded)
        .map_err(|_| "VMess link does not contain valid base64 JSON.".to_string())?;
    let decoded = String::from_utf8(bytes)
        .map_err(|_| "VMess link does not contain valid UTF-8 JSON.".to_string())?;
    let vmess: VmessLink =
        serde_json::from_str(&decoded).map_err(|error| format!("Invalid VMess JSON: {error}"))?;

    if vmess.add.trim().is_empty() {
        return Err("VMess link is missing its server address.".to_string());
    }
    uuid::Uuid::parse_str(&vmess.id)
        .map_err(|_| "VMess link contains an invalid UUID.".to_string())?;
    let server_port = value_u16(&vmess.port, "VMess port")?;
    let alter_id = if vmess.aid.is_null() {
        0
    } else {
        value_u32(&vmess.aid, "VMess alter ID")?
    };

    let tls = if vmess.tls.is_empty() || vmess.tls == "none" {
        None
    } else if vmess.tls == "tls" {
        Some(OutboundTls {
            enabled: true,
            server_name: nonempty(Some(if vmess.sni.is_empty() {
                vmess.host.clone()
            } else {
                vmess.sni.clone()
            })),
            insecure: false,
            alpn: split_csv(&vmess.alpn),
            utls: nonempty(Some(vmess.fp.clone())).map(|fingerprint| OutboundUtls {
                enabled: true,
                fingerprint,
            }),
            reality: None,
        })
    } else {
        return Err(format!("Unsupported VMess security mode '{}'.", vmess.tls));
    };

    let transport = transport_from_values(
        nonempty(Some(vmess.net)).as_deref(),
        nonempty(Some(vmess.host)).as_deref(),
        nonempty(Some(vmess.path)).as_deref(),
        None,
        nonempty(Some(vmess.header_type)).as_deref(),
    )?;

    Ok(OutboundConfig::Vmess(VmessOutbound {
        tag: SECONDARY_TAG.to_string(),
        server: vmess.add,
        server_port,
        uuid: vmess.id,
        security: nonempty(Some(vmess.scy)).unwrap_or_else(|| "auto".to_string()),
        alter_id,
        packet_encoding: Some("xudp".to_string()),
        tls,
        transport,
    }))
}

fn parse_shadowsocks(link: &str) -> Result<OutboundConfig, String> {
    // Legacy SIP002 links encode the complete method:password@host:port value.
    // That base64 text is not always a valid URL host, so parse its optional
    // query independently and decode the body below.
    let query = Url::parse(link)
        .map(|url| query_map(&url))
        .unwrap_or_default();
    let body = link
        .trim_start_matches("ss://")
        .split('#')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();

    let expanded = if body.contains('@') {
        body.to_string()
    } else {
        String::from_utf8(decode_base64(body).map_err(|_| {
            "Shadowsocks link does not contain valid credentials and server details.".to_string()
        })?)
        .map_err(|_| "Shadowsocks link contains invalid text.".to_string())?
    };
    let (encoded_credentials, endpoint) = expanded
        .rsplit_once('@')
        .ok_or_else(|| "Shadowsocks link is missing its server address.".to_string())?;
    let credentials = if encoded_credentials.contains(':') {
        decode(encoded_credentials)
    } else {
        String::from_utf8(
            decode_base64(encoded_credentials)
                .map_err(|_| "Shadowsocks credentials are not valid base64.".to_string())?,
        )
        .map_err(|_| "Shadowsocks credentials contain invalid text.".to_string())?
    };
    let (method, password) = credentials
        .split_once(':')
        .ok_or_else(|| "Shadowsocks credentials must contain method:password.".to_string())?;
    let endpoint_url = Url::parse(&format!("ss://user@{endpoint}"))
        .map_err(|error| format!("Invalid Shadowsocks server: {error}"))?;
    let server = required_host(&endpoint_url, "Shadowsocks")?;
    let server_port = required_port(&endpoint_url, "Shadowsocks")?;

    let (plugin, plugin_opts) = query.get("plugin").map_or((None, None), |plugin_value| {
        let mut parts = plugin_value.splitn(2, ';');
        (
            nonempty(parts.next().map(str::to_string)),
            nonempty(parts.next().map(str::to_string)),
        )
    });

    Ok(OutboundConfig::Shadowsocks(ShadowsocksOutbound {
        tag: SECONDARY_TAG.to_string(),
        server,
        server_port,
        method: method.to_string(),
        password: password.to_string(),
        plugin,
        plugin_opts,
    }))
}

fn tls_from_query(
    security: &str,
    server: &str,
    query: &HashMap<String, String>,
) -> Result<Option<OutboundTls>, String> {
    if security == "none" || security.is_empty() {
        return Ok(None);
    }
    if security != "tls" && security != "reality" {
        return Err(format!("Unsupported TLS security mode '{security}'."));
    }

    let server_name = query
        .get("sni")
        .or_else(|| query.get("servername"))
        .cloned()
        .filter(|value| !value.is_empty())
        .or_else(|| Some(server.to_string()));
    let reality = if security == "reality" {
        let public_key = query
            .get("pbk")
            .or_else(|| query.get("publickey"))
            .filter(|value| !value.is_empty())
            .cloned()
            .ok_or_else(|| "Reality config is missing its public key (pbk).".to_string())?;
        Some(OutboundReality {
            enabled: true,
            public_key,
            short_id: query
                .get("sid")
                .or_else(|| query.get("shortid"))
                .cloned()
                .unwrap_or_default(),
        })
    } else {
        None
    };

    Ok(Some(OutboundTls {
        enabled: true,
        server_name,
        insecure: query
            .get("allowinsecure")
            .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true")),
        alpn: {
            let mut items = query
                .get("alpn")
                .map_or_else(Vec::new, |value| split_csv(value));
            if security == "reality" {
                items.retain(|proto| proto != "h3");
            }
            items
        },
        utls: query
            .get("fp")
            .filter(|value| !value.is_empty())
            .cloned()
            .map(|fingerprint| OutboundUtls {
                enabled: true,
                fingerprint,
            }),
        reality,
    }))
}

fn transport_from_values(
    kind: Option<&str>,
    host: Option<&str>,
    path: Option<&str>,
    service_name: Option<&str>,
    header_type: Option<&str>,
) -> Result<Option<Value>, String> {
    let kind = kind.unwrap_or("tcp").to_ascii_lowercase();
    match kind.as_str() {
        "" | "tcp" | "raw" => {
            if header_type.is_some_and(|value| !value.is_empty() && value != "none") {
                return Err(format!(
                    "TCP header type '{}' is not supported by sing-box share-link import.",
                    header_type.unwrap_or_default()
                ));
            }
            Ok(None)
        }
        "ws" | "websocket" => {
            let mut headers = serde_json::Map::new();
            if let Some(host) = host.filter(|value| !value.is_empty()) {
                headers.insert("Host".to_string(), Value::String(host.to_string()));
            }
            Ok(Some(json!({
                "type": "ws",
                "path": path.unwrap_or(""),
                "headers": headers
            })))
        }
        "grpc" => Ok(Some(json!({
            "type": "grpc",
            "service_name": service_name.or(path).unwrap_or("")
        }))),
        "httpupgrade" => Ok(Some(json!({
            "type": "httpupgrade",
            "host": host.unwrap_or(""),
            "path": path.unwrap_or("")
        }))),
        "http" | "h2" => Ok(Some(json!({
            "type": "http",
            "host": host.filter(|value| !value.is_empty()).map(|value| vec![value]).unwrap_or_default(),
            "path": path.unwrap_or("")
        }))),
        unsupported => Err(format!(
            "Transport '{unsupported}' is not supported. Use TCP, WebSocket, HTTP, HTTPUpgrade, or gRPC."
        )),
    }
}

fn query_map(url: &Url) -> HashMap<String, String> {
    url.query_pairs()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.into_owned()))
        .collect()
}

fn required_host(url: &Url, protocol: &str) -> Result<String, String> {
    url.host_str()
        .filter(|host| !host.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{protocol} link is missing its server address."))
}

fn required_port(url: &Url, protocol: &str) -> Result<u16, String> {
    url.port()
        .ok_or_else(|| format!("{protocol} link is missing its server port."))
}

fn decode(value: &str) -> String {
    percent_decode_str(value).decode_utf8_lossy().into_owned()
}

fn decode_base64(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    let value = value.trim();
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(value))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(value))
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(value))
}

fn value_u16(value: &Value, field: &str) -> Result<u16, String> {
    let parsed = value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
        .ok_or_else(|| format!("{field} is invalid."))?;
    u16::try_from(parsed).map_err(|_| format!("{field} is outside the valid port range."))
}

fn value_u32(value: &Value, field: &str) -> Result<u32, String> {
    let parsed = value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<u64>().ok()))
        .ok_or_else(|| format!("{field} is invalid."))?;
    u32::try_from(parsed).map_err(|_| format!("{field} is too large."))
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vless_reality_link() {
        let link = "vless://123e4567-e89b-12d3-a456-426614174000@example.com:443?encryption=none&security=reality&sni=cdn.example.com&fp=chrome&pbk=public-key&sid=abcd&type=grpc&serviceName=tunnel";
        let outbound = parse_share_link(link).unwrap();
        let value = serde_json::to_value(outbound).unwrap();
        assert_eq!(value["type"], "vless");
        assert_eq!(value["tag"], SECONDARY_TAG);
        assert_eq!(value["tls"]["reality"]["public_key"], "public-key");
        assert_eq!(value["transport"]["type"], "grpc");
    }

    #[test]
    fn parses_vmess_websocket_link() {
        let payload = json!({
            "v": "2", "ps": "test", "add": "example.com", "port": "443",
            "id": "123e4567-e89b-12d3-a456-426614174000", "aid": "0",
            "scy": "auto", "net": "ws", "host": "cdn.example.com",
            "path": "/proxy", "tls": "tls", "sni": "cdn.example.com"
        });
        let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(payload.to_string());
        let outbound = parse_share_link(&format!("vmess://{encoded}")).unwrap();
        let value = serde_json::to_value(outbound).unwrap();
        assert_eq!(value["type"], "vmess");
        assert_eq!(value["transport"]["headers"]["Host"], "cdn.example.com");
    }

    #[test]
    fn parses_base64_subscription_and_preserves_profile_names() {
        let links = "vless://123e4567-e89b-12d3-a456-426614174000@example.com:443?encryption=none&security=tls#London\n\
trojan://secret@example.net:443?security=tls#Amsterdam";
        let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(links);
        let parsed = parse_subscription_payload(&encoded).unwrap();

        assert_eq!(parsed.profiles.len(), 2);
        assert_eq!(parsed.skipped, 0);
        assert_eq!(parsed.profiles[0].name, "London");
        assert_eq!(parsed.profiles[1].name, "Amsterdam");
    }

    #[test]
    fn subscription_skips_unsupported_profiles() {
        let payload = "hysteria2://secret@example.com:443#Unsupported\n\
vless://123e4567-e89b-12d3-a456-426614174000@example.com:443?encryption=none&security=tls#Supported";
        let parsed = parse_subscription_payload(payload).unwrap();

        assert_eq!(parsed.profiles.len(), 1);
        assert_eq!(parsed.skipped, 1);
        assert_eq!(parsed.profiles[0].name, "Supported");
    }
}
