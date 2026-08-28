use aether_desktop_lib::models::singbox::{InboundConfig, OutboundConfig};
use aether_desktop_lib::models::{ApplicationRule, AppSettings, RouteDestination, RulePriority, RuleSource};
use aether_desktop_lib::routing::SingBoxConfigGenerator;

fn main() {
    println!("=== Running Aether Desktop Complete Regression Test Suite ===");

    test_reference_config_match();
    println!("✓ TEST 0: Reference sing-box configuration match (PASSED)");

    test_scenario_1_discord_high_priority_3478();
    println!("✓ TEST 1: Discord.exe (High Priority) on port 3478 -> v2ray (PASSED - Discord Voice Fixed)");

    test_scenario_2_discord_high_priority_5349();
    println!("✓ TEST 2: Discord.exe (High Priority) on port 5349 -> v2ray (PASSED - Discord Voice Fixed)");

    test_scenario_3_normal_secondary_proxy();
    println!("✓ TEST 3: Spotify.exe (Normal Priority) on port 443 -> v2ray (PASSED)");

    test_scenario_4_global_compatibility_fallback_against_normal_rule();
    println!("✓ TEST 4: Spotify.exe (Normal Priority) on port 3478 -> direct (PASSED - Generals Fallback Wins)");

    test_scenario_5_high_custom_override();
    println!("✓ TEST 5: Spotify.exe (High Priority Override) on port 3478 -> v2ray (PASSED)");

    test_scenario_6_generals_regression();
    println!("✓ TEST 6: Unassigned application on port 3478/5349 -> direct (PASSED - Generals Online Fixed)");

    test_scenario_7_unmatched_normal_traffic();
    println!("✓ TEST 7: Unassigned application normal traffic -> aether (PASSED)");

    test_scenario_8_private_lan();
    println!("✓ TEST 8: Private LAN (192.168.1.1, 10.0.0.1, 172.16.0.1) -> direct (PASSED)");

    test_scenario_9_proxy_loop_prevention();
    println!("✓ TEST 9: Core Proxy Loop Prevention (aether.exe, xray.exe, v2ray.exe, v2rayN.exe) -> direct (PASSED)");

    println!("\n==================================================================");
    println!("ALL 10 VERIFICATION TESTS PASSED SUCCESSFULLY!");
    println!("==================================================================");
}

fn test_reference_config_match() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    assert_eq!(config.log.level, "info");
    assert!(config.log.timestamp);

    assert_eq!(config.inbounds.len(), 1);
    match &config.inbounds[0] {
        InboundConfig::Tun(tun) => {
            assert_eq!(tun.tag, "tun-in");
            assert_eq!(tun.interface_name, "singbox-tun");
            assert_eq!(tun.address, vec!["172.19.0.1/30"]);
            assert_eq!(tun.mtu, 1500);
            assert!(tun.auto_route);
            assert!(tun.strict_route);
            assert_eq!(tun.stack, "system");
        }
    }

    assert_eq!(config.outbounds.len(), 3);
    match &config.outbounds[0] {
        OutboundConfig::Socks(s) => {
            assert_eq!(s.tag, "aether");
            assert_eq!(s.server, "127.0.0.1");
            assert_eq!(s.server_port, 1819);
        }
        _ => panic!("Expected socks outbound aether"),
    }
    match &config.outbounds[1] {
        OutboundConfig::Socks(s) => {
            assert_eq!(s.tag, "v2ray");
            assert_eq!(s.server, "127.0.0.1");
            assert_eq!(s.server_port, 10808);
        }
        _ => panic!("Expected socks outbound v2ray"),
    }
    match &config.outbounds[2] {
        OutboundConfig::Direct(d) => {
            assert_eq!(d.tag, "direct");
        }
        _ => panic!("Expected direct outbound"),
    }

    let rules = &config.route.rules;
    assert_eq!(rules.len(), 6);

    // Rule 0: Process loop prevention -> direct
    assert_eq!(
        rules[0].process_name.as_ref().unwrap(),
        &vec!["xray.exe", "v2ray.exe", "v2rayN.exe", "aether.exe"]
    );
    assert_eq!(rules[0].outbound, "direct");

    // Rule 1: High Priority Secondary Proxy (Discord.exe) -> v2ray
    assert_eq!(
        rules[1].process_name.as_ref().unwrap(),
        &vec!["Discord.exe"]
    );
    assert_eq!(rules[1].outbound, "v2ray");

    // Rule 2: Global Generals Online STUN/TURN fallback -> direct
    assert_eq!(rules[2].port.as_ref().unwrap(), &vec![3478, 5349]);
    assert_eq!(rules[2].outbound, "direct");

    // Rule 3: Normal Priority Direct applications -> direct
    assert_eq!(
        rules[3].process_name.as_ref().unwrap(),
        &vec!["dota2.exe", "RustClient.exe", "Rust.exe"]
    );
    assert_eq!(rules[3].outbound, "direct");

    // Rule 4: Normal Priority Secondary Proxy applications -> v2ray
    assert_eq!(
        rules[4].process_name.as_ref().unwrap(),
        &vec![
            "chrome.exe",
            "Code.exe",
            "codex.exe",
            "Antigravity.exe",
            "agy.exe",
            "language_server.exe"
        ]
    );
    assert_eq!(rules[4].outbound, "v2ray");

    // Rule 5: Private IP network bypass -> direct
    assert_eq!(rules[5].ip_is_private, Some(true));
    assert_eq!(rules[5].outbound, "direct");

    // Final route -> aether
    assert_eq!(config.route.final_outbound, "aether");
}

fn test_scenario_1_discord_high_priority_3478() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("Discord.exe"),
        Some(3478),
        false,
    );
    assert_eq!(
        outbound, "v2ray",
        "Discord.exe (High Priority) on port 3478 must route to v2ray!"
    );
}

fn test_scenario_2_discord_high_priority_5349() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("Discord.exe"),
        Some(5349),
        false,
    );
    assert_eq!(
        outbound, "v2ray",
        "Discord.exe (High Priority) on port 5349 must route to v2ray!"
    );
}

fn test_scenario_3_normal_secondary_proxy() {
    let mut settings = AppSettings::default();
    settings.application_rules.push(ApplicationRule::new(
        "Spotify",
        "Spotify.exe",
        RouteDestination::SecondaryProxy,
        None,
        RuleSource::User,
        RulePriority::Normal,
        None,
    ));

    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("Spotify.exe"),
        Some(443),
        false,
    );
    assert_eq!(
        outbound, "v2ray",
        "Spotify.exe (Normal Priority) on port 443 must route to v2ray!"
    );
}

fn test_scenario_4_global_compatibility_fallback_against_normal_rule() {
    let mut settings = AppSettings::default();
    settings.application_rules.push(ApplicationRule::new(
        "Spotify",
        "Spotify.exe",
        RouteDestination::SecondaryProxy,
        None,
        RuleSource::User,
        RulePriority::Normal,
        None,
    ));

    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("Spotify.exe"),
        Some(3478),
        false,
    );
    assert_eq!(
        outbound, "direct",
        "Spotify.exe (Normal Priority) on port 3478 must route to direct because global fallback precedes Normal rules!"
    );
}

fn test_scenario_5_high_custom_override() {
    let mut settings = AppSettings::default();
    settings.application_rules.push(ApplicationRule::new(
        "Spotify",
        "Spotify.exe",
        RouteDestination::SecondaryProxy,
        None,
        RuleSource::User,
        RulePriority::High,
        None,
    ));

    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("Spotify.exe"),
        Some(3478),
        false,
    );
    assert_eq!(
        outbound, "v2ray",
        "Spotify.exe (High Priority) on port 3478 must route to v2ray because High priority precedes global fallback!"
    );
}

fn test_scenario_6_generals_regression() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound_3478 = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("generals.exe"),
        Some(3478),
        false,
    );
    assert_eq!(
        outbound_3478, "direct",
        "Unmatched app on port 3478 must route direct (Generals Online fix)!"
    );

    let outbound_5349 = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("generals.exe"),
        Some(5349),
        false,
    );
    assert_eq!(
        outbound_5349, "direct",
        "Unmatched app on port 5349 must route direct (Generals Online fix)!"
    );
}

fn test_scenario_7_unmatched_normal_traffic() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound = SingBoxConfigGenerator::resolve_route(
        &config,
        Some("curl.exe"),
        Some(443),
        false,
    );
    assert_eq!(
        outbound, "aether",
        "Unmatched app on normal port 443 must fall through to aether!"
    );
}

fn test_scenario_8_private_lan() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    // 192.168.1.1
    let outbound_192 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("192.168.1.1"),
    );
    assert_eq!(
        outbound_192, "direct",
        "Private LAN destination 192.168.1.1 must route direct!"
    );

    // 10.0.0.1
    let outbound_10 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("10.0.0.1"),
    );
    assert_eq!(
        outbound_10, "direct",
        "Private LAN destination 10.0.0.1 must route direct!"
    );

    // 172.16.0.1
    let outbound_172 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("172.16.0.1"),
    );
    assert_eq!(
        outbound_172, "direct",
        "Private LAN destination 172.16.0.1 must route direct!"
    );

    // Public IP (e.g. 8.8.8.8) with unassigned app should NOT be direct
    let outbound_pub = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(53),
        Some("8.8.8.8"),
    );
    assert_eq!(
        outbound_pub, "aether",
        "Public IP 8.8.8.8 must not match private LAN rule!"
    );
}

fn test_scenario_9_proxy_loop_prevention() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    for proc in &["aether.exe", "xray.exe", "v2ray.exe", "v2rayN.exe"] {
        let outbound = SingBoxConfigGenerator::resolve_route(
            &config,
            Some(proc),
            Some(10808),
            false,
        );
        assert_eq!(
            outbound, "direct",
            "Process loop prevention for {} must route direct!",
            proc
        );
    }
}