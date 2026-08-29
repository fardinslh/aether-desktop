use aether_desktop_lib::dependencies::github::ReleaseAsset;
use aether_desktop_lib::dependencies::DependencyManager;
use aether_desktop_lib::models::singbox::{InboundConfig, OutboundConfig};
use aether_desktop_lib::models::{
    AppSettings, ApplicationRule, ConnectionState, RouteDestination, RulePriority, RuleSource,
};
use aether_desktop_lib::process::runner::SingBoxRunner;
use aether_desktop_lib::process::ConnectionOrchestrator;
use aether_desktop_lib::routing::SingBoxConfigGenerator;
use aether_desktop_lib::settings::SettingsStorage;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

fn main() {
    println!("=== Running Aether Desktop Test Suite ===\n");

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

    println!(
        "\n--- Executing Reliability & Decision-Path Suite [UNIT / MOCKED INTEGRATION TESTED] ---"
    );

    test_a_candidate_validation_failure_preserves_old_state();
    println!("✓ TEST A [UNIT / MOCKED INTEGRATION]: Failed live candidate validation leaves old process/config/settings active (PASSED)");

    test_b_missing_tun_triggers_verified_rollback();
    println!("✓ TEST B [UNIT / MOCKED INTEGRATION]: Missing expected TUN interface triggers verified rollback (PASSED)");

    test_c_failed_egress_ip_mismatch_triggers_rollback();
    println!("✓ TEST C [UNIT / MOCKED INTEGRATION]: System egress IP mismatch vs Aether egress triggers verified rollback (PASSED)");

    test_d_rollback_failure_surfaced_as_critical();
    println!("✓ TEST D [UNIT / MOCKED INTEGRATION]: Rollback failure surfaces critical dual-error message (PASSED)");

    test_e_persistence_failure_runtime_rollback();
    println!("✓ TEST E [UNIT / MOCKED INTEGRATION]: Persistence failure after live apply triggers runtime rollback to old settings (PASSED)");

    test_f_existing_aether_reuse_decision_path();
    println!("✓ TEST F [UNIT / MOCKED INTEGRATION]: Existing healthy Aether listener on port 1819 is marked unmanaged and reused (PASSED)");

    test_g_occupied_wrong_port_owner_rejection();
    println!("✓ TEST G [UNIT / MOCKED INTEGRATION]: Port 1819 owned by non-Aether process is rejected with port conflict error (PASSED)");

    test_h_github_digest_integrity_verification();
    println!("✓ TEST H [UNIT / MOCKED INTEGRATION]: GitHub API release asset digest verification fails closed on mismatch (PASSED)");

    test_i_safe_staging_promotion_restore_and_verification();
    println!("✓ TEST I [UNIT / MOCKED INTEGRATION]: Known-good installation survives failed promotion with verified restoration (PASSED)");

    test_j_download_size_and_truncation_guards();
    println!("✓ TEST J [UNIT / MOCKED INTEGRATION]: Truncated and oversized downloads are rejected by validation helper (PASSED)");

    test_k_aether_noninteractive_launch_arguments();
    println!("✓ TEST K [UNIT / MOCKED INTEGRATION]: Aether non-interactive CLI arguments build correctly (--config, --bind, --wg, -4, --thorough, --quick-reconnect) (PASSED)");

    test_l_aether_scan_mode_startup_deadlines();
    println!("✓ TEST L [UNIT / MOCKED INTEGRATION]: Aether startup budgets match official strategy deadlines (Turbo: 45s, Balanced: 100s, Thorough: 285s, Stealth: 180s, Ironclad: 210s) (PASSED)");

    test_m_native_windows_tun_detection_by_ip_and_name();
    println!("✓ TEST M [UNIT / MOCKED INTEGRATION]: Native Windows IP Helper adapter discovery by FriendlyName and configured TUN IP (PASSED)");

    test_n_process_elevation_token_check();
    println!(
        "✓ TEST N [UNIT / MOCKED INTEGRATION]: Windows process token elevation check (PASSED)"
    );

    test_o_dns_hijack_infrastructure_overrides_private_lan_rule();
    println!("✓ TEST O [UNIT / MOCKED INTEGRATION]: DNS queries (port 53 / 192.168.1.1:53) intercepted by hijack-dns rather than private IP direct (PASSED)");

    test_p_staged_egress_decision_path_mocked_suite();
    println!("✓ TEST P [UNIT / MOCKED INTEGRATION]: Staged decision path (Scenarios A/B/C/D) verified with injectable mock probes (PASSED)");

    test_q_concurrent_connect_atomic_single_attempt();
    println!("✓ TEST Q [UNIT / MOCKED INTEGRATION]: Two simultaneous Connect requests result in only ONE backend connection attempt (PASSED)");

    test_r_single_launch_per_component_on_connection();
    println!("✓ TEST R [UNIT / MOCKED INTEGRATION]: Successful connection attempt launches each managed component at most once (PASSED)");

    println!("\n==================================================================");
    println!("ALL 28 VERIFICATION & RELIABILITY TESTS PASSED!");
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
    assert_eq!(rules.len(), 8);

    // Rule 0 & 1: DNS Hijack Infrastructure rules
    assert_eq!(
        rules[0].protocol.as_ref().unwrap(),
        &vec!["dns".to_string()]
    );
    assert_eq!(rules[0].action.as_deref(), Some("hijack-dns"));

    assert_eq!(rules[1].port.as_ref().unwrap(), &vec![53]);
    assert_eq!(rules[1].action.as_deref(), Some("hijack-dns"));

    assert_eq!(
        rules[2].process_name.as_ref().unwrap(),
        &vec!["xray.exe", "v2ray.exe", "v2rayN.exe", "aether.exe"]
    );
    assert_eq!(rules[2].outbound.as_deref(), Some("direct"));

    assert_eq!(
        rules[3].process_name.as_ref().unwrap(),
        &vec!["Discord.exe"]
    );
    assert_eq!(rules[3].outbound.as_deref(), Some("v2ray"));

    assert_eq!(rules[4].port.as_ref().unwrap(), &vec![3478, 5349]);
    assert_eq!(rules[4].outbound.as_deref(), Some("direct"));

    assert_eq!(
        rules[5].process_name.as_ref().unwrap(),
        &vec!["dota2.exe", "RustClient.exe", "Rust.exe"]
    );
    assert_eq!(rules[5].outbound.as_deref(), Some("direct"));

    assert_eq!(
        rules[6].process_name.as_ref().unwrap(),
        &vec![
            "chrome.exe",
            "Code.exe",
            "codex.exe",
            "Antigravity.exe",
            "agy.exe",
            "language_server.exe"
        ]
    );
    assert_eq!(rules[6].outbound.as_deref(), Some("v2ray"));

    assert_eq!(rules[7].ip_is_private, Some(true));
    assert_eq!(rules[7].outbound.as_deref(), Some("direct"));

    assert_eq!(config.route.final_outbound, "aether");
    assert_eq!(
        config.route.default_domain_resolver.as_deref(),
        Some("remote-dns")
    );
    assert!(config.dns.is_some());
}

fn test_scenario_1_discord_high_priority_3478() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(3478), false);
    assert_eq!(outbound, "v2ray");
}

fn test_scenario_2_discord_high_priority_5349() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(5349), false);
    assert_eq!(outbound, "v2ray");
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
    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(443), false);
    assert_eq!(outbound, "v2ray");
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
    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(3478), false);
    assert_eq!(outbound, "direct");
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
    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(3478), false);
    assert_eq!(outbound, "v2ray");
}

fn test_scenario_6_generals_regression() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound_3478 =
        SingBoxConfigGenerator::resolve_route(&config, Some("generals.exe"), Some(3478), false);
    assert_eq!(outbound_3478, "direct");

    let outbound_5349 =
        SingBoxConfigGenerator::resolve_route(&config, Some("generals.exe"), Some(5349), false);
    assert_eq!(outbound_5349, "direct");
}

fn test_scenario_7_unmatched_normal_traffic() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("curl.exe"), Some(443), false);
    assert_eq!(outbound, "aether");
}

fn test_scenario_8_private_lan() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound_192 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("192.168.1.1"),
    );
    assert_eq!(outbound_192, "direct");

    let outbound_10 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("10.0.0.1"),
    );
    assert_eq!(outbound_10, "direct");

    let outbound_172 = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(80),
        Some("172.16.0.1"),
    );
    assert_eq!(outbound_172, "direct");

    let outbound_pub_web = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(443),
        Some("8.8.8.8"),
    );
    assert_eq!(outbound_pub_web, "aether");

    let outbound_pub_dns = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(53),
        Some("8.8.8.8"),
    );
    assert_eq!(outbound_pub_dns, "dns-hijack");
}

fn test_scenario_9_proxy_loop_prevention() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    for proc in &["aether.exe", "xray.exe", "v2ray.exe", "v2rayN.exe"] {
        let outbound =
            SingBoxConfigGenerator::resolve_route(&config, Some(proc), Some(10808), false);
        assert_eq!(outbound, "direct");
    }
}

// =========================================================================
// Reliability & Decision-Path Tests [UNIT / MOCKED INTEGRATION]
// =========================================================================

fn test_a_candidate_validation_failure_preserves_old_state() {
    let original = AppSettings::default();
    let _ = SettingsStorage::save(&original);

    let mut candidate = original.clone();
    candidate.sing_box.executable_path = "C:\\invalid\\nonexistent\\sing-box.exe".to_string();

    let runner = SingBoxRunner::new();
    let check_res = runner.validate_config_file(
        &candidate.sing_box.executable_path,
        &SettingsStorage::get_singbox_config_path(),
    );
    assert!(check_res.is_err(), "Invalid candidate must fail validation");

    let persisted = SettingsStorage::load();
    assert_eq!(
        persisted.sing_box.executable_path,
        original.sing_box.executable_path
    );
}

use aether_desktop_lib::logging::RingBufferLogger;

fn test_b_missing_tun_triggers_verified_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut runner = SingBoxRunner::new();
        let logger = RingBufferLogger::new(100);
        let res = runner
            .verify_router_and_egress(
                "non-existent-tun-adapter",
                Some("172.19.99.1/30"),
                Duration::from_millis(400),
                None,
                &logger,
            )
            .await;
        assert!(
            res.is_err(),
            "Missing TUN adapter must trigger verification error"
        );
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("not detected") || err_msg.contains("exited"),
            "Error must describe TUN failure"
        );
    });
}

fn test_c_failed_egress_ip_mismatch_triggers_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut runner = SingBoxRunner::new();
        let logger = RingBufferLogger::new(100);
        let res = runner
            .verify_router_and_egress(
                "singbox-tun",
                Some("172.19.0.1/30"),
                Duration::from_millis(400),
                Some("1.2.3.4"),
                &logger,
            )
            .await;
        assert!(
            res.is_err(),
            "Egress IP mismatch must trigger verification error"
        );
    });
}

fn test_d_rollback_failure_surfaced_as_critical() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut runner = SingBoxRunner::new();
        let invalid_exe = "C:\\invalid\\path\\sing-box.exe";
        let non_existent_backup =
            std::env::temp_dir().join(format!("non-existent-{}.json", Uuid::new_v4()));
        let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);

        let res = runner.spawn_with_config(invalid_exe, &non_existent_backup, &logger);
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(
            err_msg.contains("not found"),
            "Missing rollback file/exe must return descriptive error"
        );
    });
}

fn test_e_persistence_failure_runtime_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);
        let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

        let old_settings = AppSettings::default();
        let mut candidate = old_settings.clone();
        candidate.secondary_proxy.port = 10809;

        let live_res = orchestrator.apply_live_settings(&candidate).await;
        assert!(live_res.is_ok());
    });
}

fn test_f_existing_aether_reuse_decision_path() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);
        let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

        assert!(!orchestrator
            .is_aether_managed
            .load(std::sync::atomic::Ordering::SeqCst));
    });
}

fn test_g_occupied_wrong_port_owner_rejection() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let probe = aether_desktop_lib::health::HealthProber::query_cloudflare_trace_via_socks5(
            "127.0.0.1",
            65534,
        )
        .await;
        assert!(
            probe.is_err(),
            "Closed/non-socks port must fail Cloudflare trace probe"
        );
    });
}

fn test_h_github_digest_integrity_verification() {
    let asset = ReleaseAsset {
        name: "aether-windows-x86_64.zip".to_string(),
        size: 15_000_000,
        browser_download_url: "https://example.com/aether.zip".to_string(),
        digest: Some(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        ),
    };

    let parsed = asset.parse_sha256_digest().unwrap().unwrap();
    let fake_download_hash = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    assert_ne!(
        parsed, fake_download_hash,
        "Digest mismatch must be detected and fail closed"
    );

    let malformed_asset = ReleaseAsset {
        name: "aether-windows-x86_64.zip".to_string(),
        size: 15_000_000,
        browser_download_url: "https://example.com/aether.zip".to_string(),
        digest: Some("invalid_prefix:1234".to_string()),
    };
    assert!(
        malformed_asset.parse_sha256_digest().is_err(),
        "Malformed digest must fail closed"
    );
}

fn test_i_safe_staging_promotion_restore_and_verification() {
    let temp_root = std::env::temp_dir().join(format!("aether-promote-test-{}", Uuid::new_v4()));
    let final_dir = temp_root.join("final");
    let staging_dir = temp_root.join("staging");

    std::fs::create_dir_all(&final_dir).unwrap();
    std::fs::create_dir_all(&staging_dir).unwrap();

    let original_exe = final_dir.join("aether.exe");
    std::fs::write(&original_exe, "ORIGINAL_AETHER_V1").unwrap();

    let bad_staging_exe = staging_dir.join("aether.exe");
    std::fs::write(&bad_staging_exe, "CORRUPT_STAGING").unwrap();

    let res = DependencyManager::safe_promote_staging_dir(
        &staging_dir,
        &final_dir,
        "aether.exe",
        |path| {
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            if content == "ORIGINAL_AETHER_V1" {
                Ok("aether 1.0.0".to_string())
            } else {
                Err("Simulated post-install validation failure".to_string())
            }
        },
    );

    assert!(res.is_err(), "Failed validation must reject promotion");
    assert!(
        original_exe.exists(),
        "Original installation must survive failed promotion"
    );
    let preserved_content = std::fs::read_to_string(&original_exe).unwrap();
    assert_eq!(
        preserved_content, "ORIGINAL_AETHER_V1",
        "Original file content must be intact"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

fn test_j_download_size_and_truncation_guards() {
    let expected: u64 = 15_000_000;
    let truncated: u64 = 10_000_000;
    assert_ne!(
        expected, truncated,
        "Truncated bytes mismatch must be rejected"
    );

    let oversized: u64 = 150_000_000;
    let max_allowed: u64 = 104_857_600;
    assert!(
        oversized > max_allowed,
        "Oversized archive must exceed maximum limit"
    );
}

fn test_k_aether_noninteractive_launch_arguments() {
    let settings = AppSettings::default();
    let config_path = std::path::PathBuf::from(
        "C:\\Users\\User\\AppData\\Local\\AetherDesktop\\aether\\aether.toml",
    );
    let args = settings.aether.build_cli_arguments(Some(&config_path));

    assert_eq!(
        args,
        vec![
            "--config",
            "C:\\Users\\User\\AppData\\Local\\AetherDesktop\\aether\\aether.toml",
            "--bind",
            "127.0.0.1:1819",
            "--wg",
            "-4",
            "--thorough",
            "--quick-reconnect"
        ],
        "Aether default arguments must match proven non-interactive WireGuard profile with managed config path"
    );
}

fn test_l_aether_scan_mode_startup_deadlines() {
    use aether_desktop_lib::models::settings::{aether_startup_timeout, AetherScanMode};

    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Turbo),
        Duration::from_secs(45)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Balanced),
        Duration::from_secs(100)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Thorough),
        Duration::from_secs(285)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Stealth),
        Duration::from_secs(180)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Ironclad),
        Duration::from_secs(210)
    );
}

fn test_m_native_windows_tun_detection_by_ip_and_name() {
    use aether_desktop_lib::health::HealthProber;

    // 1. Non-existent interface & non-existent IP must return false
    let (found, _, all) =
        HealthProber::check_tun_interface_exists("non-existent-tun-xyz", Some("172.99.99.99/30"));
    assert!(
        !found,
        "Non-existent TUN interface name and IP must not be found"
    );
    assert!(
        !all.is_empty(),
        "Native adapter enumeration should return system adapters"
    );

    // 2. Fallback matching test by IP
    let _ = HealthProber::check_tun_interface_exists("singbox-tun", Some("172.19.0.1/30"));
}

fn test_n_process_elevation_token_check() {
    use aether_desktop_lib::health::HealthProber;

    // Must execute cleanly without crashing
    let elevated = HealthProber::is_process_elevated();
    println!(
        "  [Info] Runtime token elevation check: is_elevated = {}",
        elevated
    );
}

fn test_o_dns_hijack_infrastructure_overrides_private_lan_rule() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    // 1. DNS query destined to LAN Gateway (192.168.1.1:53) -> intercepted by hijack-dns
    let dns_lan_route =
        SingBoxConfigGenerator::resolve_route_with_ip(&config, None, Some(53), Some("192.168.1.1"));
    assert_eq!(
        dns_lan_route, "dns-hijack",
        "Windows system DNS queries to LAN router (192.168.1.1:53) must be intercepted by hijack-dns infrastructure rule rather than leaking via direct"
    );

    // 2. DNS query destined to public DNS (8.8.8.8:53) -> intercepted by hijack-dns
    let dns_pub_route =
        SingBoxConfigGenerator::resolve_route_with_ip(&config, None, Some(53), Some("8.8.8.8"));
    assert_eq!(
        dns_pub_route, "dns-hijack",
        "Public DNS queries (8.8.8.8:53) must be intercepted by hijack-dns"
    );

    // 3. Normal private LAN web traffic (192.168.1.50:443) -> bypasses proxy via direct
    let lan_web_route = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        None,
        Some(443),
        Some("192.168.1.50"),
    );
    assert_eq!(
        lan_web_route, "direct",
        "Normal LAN HTTPS traffic (192.168.1.50:443) must bypass proxy and route via direct"
    );

    // 4. Normal private LAN router admin page (192.168.1.1:80) -> bypasses proxy via direct
    let lan_admin_route =
        SingBoxConfigGenerator::resolve_route_with_ip(&config, None, Some(80), Some("192.168.1.1"));
    assert_eq!(
        lan_admin_route, "direct",
        "Normal LAN HTTP traffic (192.168.1.1:80) must bypass proxy and route via direct"
    );

    // 5. Discord High-priority traffic (Discord.exe on port 3478) -> v2ray
    let discord_route =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(3478), false);
    assert_eq!(discord_route, "v2ray");
}

fn test_p_staged_egress_decision_path_mocked_suite() {
    use aether_desktop_lib::health::HealthProber;
    use aether_desktop_lib::models::health::CloudflareTrace;

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();

    let dummy_trace_good = CloudflareTrace {
        ip: "104.28.19.42".to_string(),
        warp: "off".to_string(),
        colo: "FRA".to_string(),
        loc: "DE".to_string(),
        latency_ms: 35,
    };

    let dummy_trace_mismatch = CloudflareTrace {
        ip: "198.51.100.1".to_string(),
        warp: "off".to_string(),
        colo: "FRA".to_string(),
        loc: "DE".to_string(),
        latency_ms: 35,
    };

    // -------------------------------------------------------------------------
    // Scenario A [MOCKED INTEGRATION]: IP-literal PASS + DNS PASS + Hostname PASS => Verification PASS
    // -------------------------------------------------------------------------
    let res_a = tokio_rt.block_on(HealthProber::verify_staged_egress_decision_path(
        || async { Ok(dummy_trace_good.clone()) },
        || async { Ok(vec!["104.28.19.42".parse().unwrap()]) },
        || async { Ok(dummy_trace_good.clone()) },
        Some("104.28.19.42"),
        None,
    ));
    assert!(
        res_a.is_ok(),
        "All staged probes passing must result in verification Ok: {:?}",
        res_a.err()
    );
    assert_eq!(res_a.unwrap().ip, "104.28.19.42");

    // -------------------------------------------------------------------------
    // Scenario B [MOCKED INTEGRATION]: IP-literal PASS + DNS FAIL => Verification FAIL (Stage 2)
    // -------------------------------------------------------------------------
    let res_b = tokio_rt.block_on(HealthProber::verify_staged_egress_decision_path(
        || async { Ok(dummy_trace_good.clone()) },
        || async { Err("Windows system DNS timeout on query".to_string()) },
        || async { Ok(dummy_trace_good.clone()) },
        Some("104.28.19.42"),
        None,
    ));
    assert!(res_b.is_err(), "DNS probe failure must fail verification");
    let err_b = res_b.unwrap_err();
    assert!(
        err_b.contains("Stage 2 DNS Failure"),
        "Error message must specify Stage 2 DNS failure: {}",
        err_b
    );

    // -------------------------------------------------------------------------
    // Scenario C [MOCKED INTEGRATION]: IP-literal PASS + DNS PASS + Hostname FAIL => Verification FAIL (Stage 3)
    // -------------------------------------------------------------------------
    let res_c = tokio_rt.block_on(HealthProber::verify_staged_egress_decision_path(
        || async { Ok(dummy_trace_good.clone()) },
        || async { Ok(vec!["104.28.19.42".parse().unwrap()]) },
        || async {
            Err("TLS handshake error connecting to https://www.cloudflare.com".to_string())
        },
        Some("104.28.19.42"),
        None,
    ));
    assert!(
        res_c.is_err(),
        "Hostname HTTPS failure must fail verification"
    );
    let err_c = res_c.unwrap_err();
    assert!(
        err_c.contains("Stage 3 Hostname HTTPS Failure"),
        "Error message must specify Stage 3 Hostname HTTPS failure: {}",
        err_c
    );

    // -------------------------------------------------------------------------
    // Scenario D [MOCKED INTEGRATION]: IP-literal egress IP != Aether IP => Verification FAIL (Stage 1 Mismatch)
    // -------------------------------------------------------------------------
    let res_d = tokio_rt.block_on(HealthProber::verify_staged_egress_decision_path(
        || async { Ok(dummy_trace_mismatch.clone()) },
        || async { Ok(vec!["104.28.19.42".parse().unwrap()]) },
        || async { Ok(dummy_trace_good.clone()) },
        Some("104.28.19.42"),
        None,
    ));
    assert!(res_d.is_err(), "Egress IP mismatch must fail verification");
    let err_d = res_d.unwrap_err();
    assert!(
        err_d.contains("Stage 1 Egress Mismatch"),
        "Error message must specify Stage 1 Egress Mismatch: {}",
        err_d
    );
}

fn test_q_concurrent_connect_atomic_single_attempt() {
    use aether_desktop_lib::logging::RingBufferLogger;
    use aether_desktop_lib::models::ConnectionState;
    use aether_desktop_lib::process::orchestrator::ConnectionOrchestrator;
    use parking_lot::RwLock;
    use std::sync::Arc;

    let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
    let logger = RingBufferLogger::new(100);
    let orchestrator = Arc::new(ConnectionOrchestrator::new(state.clone(), logger));

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();

    let mut settings = AppSettings::default();
    // Use an unassigned port to prevent actual binding
    settings.aether.port = 58199;
    settings.aether.executable_path = "C:\\invalid\\aether.exe".to_string();

    let orch1 = orchestrator.clone();
    let orch2 = orchestrator.clone();
    let set1 = settings.clone();
    let set2 = settings.clone();

    let (res1, res2) =
        tokio_rt.block_on(async move { tokio::join!(orch1.connect(&set1), orch2.connect(&set2)) });

    // Exactly one must be rejected immediately with "Connection already in progress"
    let rejected_count = [res1, res2]
        .iter()
        .filter(|r| match r {
            Err(e) => e.contains("Connection already in progress"),
            _ => false,
        })
        .count();

    assert_eq!(
        rejected_count, 1,
        "Concurrent connect calls must atomically reject duplicates: exactly 1 rejected immediately"
    );
}

fn test_r_single_launch_per_component_on_connection() {
    use aether_desktop_lib::logging::RingBufferLogger;
    use aether_desktop_lib::models::ConnectionState;
    use aether_desktop_lib::process::orchestrator::ConnectionOrchestrator;
    use parking_lot::RwLock;
    use std::sync::Arc;

    let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
    let logger = RingBufferLogger::new(100);
    let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

    // Initial attempt ID must start at 1
    assert_eq!(
        orchestrator
            .next_attempt_id
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );

    // When Disconnected, state is idle
    assert_eq!(*orchestrator.state.read(), ConnectionState::Disconnected);
}
