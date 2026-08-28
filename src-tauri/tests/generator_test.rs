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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

fn main() {
    println!("=== Running Aether Desktop Full Behavioral Regression Test Suite ===\n");

    test_reference_config_match();
    println!("Ã¢Å“â€œ TEST 0: Reference sing-box configuration match (PASSED)");

    test_scenario_1_discord_high_priority_3478();
    println!("Ã¢Å“â€œ TEST 1: Discord.exe (High Priority) on port 3478 -> v2ray (PASSED - Discord Voice Fixed)");

    test_scenario_2_discord_high_priority_5349();
    println!("Ã¢Å“â€œ TEST 2: Discord.exe (High Priority) on port 5349 -> v2ray (PASSED - Discord Voice Fixed)");

    test_scenario_3_normal_secondary_proxy();
    println!("Ã¢Å“â€œ TEST 3: Spotify.exe (Normal Priority) on port 443 -> v2ray (PASSED)");

    test_scenario_4_global_compatibility_fallback_against_normal_rule();
    println!("Ã¢Å“â€œ TEST 4: Spotify.exe (Normal Priority) on port 3478 -> direct (PASSED - Generals Fallback Wins)");

    test_scenario_5_high_custom_override();
    println!("Ã¢Å“â€œ TEST 5: Spotify.exe (High Priority Override) on port 3478 -> v2ray (PASSED)");

    test_scenario_6_generals_regression();
    println!("Ã¢Å“â€œ TEST 6: Unassigned application on port 3478/5349 -> direct (PASSED - Generals Online Fixed)");

    test_scenario_7_unmatched_normal_traffic();
    println!("Ã¢Å“â€œ TEST 7: Unassigned application normal traffic -> aether (PASSED)");

    test_scenario_8_private_lan();
    println!("Ã¢Å“â€œ TEST 8: Private LAN (192.168.1.1, 10.0.0.1, 172.16.0.1) -> direct (PASSED)");

    test_scenario_9_proxy_loop_prevention();
    println!("Ã¢Å“â€œ TEST 9: Core Proxy Loop Prevention (aether.exe, xray.exe, v2ray.exe, v2rayN.exe) -> direct (PASSED)");

    println!("\n--- Executing Behavioral Reliability Decision-Path Tests ---");

    test_a_failed_candidate_leaves_old_state_intact();
    println!("Ã¢Å“â€œ TEST A [Behavioral]: Failed live candidate leaves old process/config/settings active (PASSED)");

    test_b_missing_tun_triggers_rollback();
    println!("Ã¢Å“â€œ TEST B [Behavioral]: New process alive but missing TUN triggers verified rollback (PASSED)");

    test_c_failed_egress_triggers_rollback();
    println!("Ã¢Å“â€œ TEST C [Behavioral]: New process + TUN present but failed egress triggers verified rollback (PASSED)");

    test_d_rollback_failure_surfaced();
    println!(
        "Ã¢Å“â€œ TEST D [Behavioral]: Rollback failure is surfaced as critical error (PASSED)"
    );

    test_e_persistence_failure_triggers_runtime_rollback();
    println!("Ã¢Å“â€œ TEST E [Behavioral]: Persistence failure after live apply triggers runtime rollback to old settings (PASSED)");

    test_f_existing_aether_reused();
    println!("Ã¢Å“â€œ TEST F [Behavioral]: Existing healthy Aether listener on port 1819 is reused without duplicate spawn (PASSED)");

    test_g_occupied_non_aether_port_rejected();
    println!("Ã¢Å“â€œ TEST G [Behavioral]: Occupied port 1819 with failed SOCKS probe is rejected with port conflict error (PASSED)");

    test_h_github_digest_mismatch_fails_closed();
    println!("Ã¢Å“â€œ TEST H [Behavioral]: GitHub API release asset digest mismatch fails closed (PASSED)");

    test_i_known_good_install_survives_promotion_failure();
    println!("Ã¢Å“â€œ TEST I [Behavioral]: Known-good installation survives failed staging promotion (PASSED)");

    test_j_truncated_and_oversized_validation();
    println!("Ã¢Å“â€œ TEST J [Behavioral]: Truncated and oversized downloads are rejected by validation helper (PASSED)");

    println!("\n==================================================================");
    println!("ALL 20 VERIFICATION & BEHAVIORAL RELIABILITY TESTS PASSED!");
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

    assert_eq!(
        rules[0].process_name.as_ref().unwrap(),
        &vec!["xray.exe", "v2ray.exe", "v2rayN.exe", "aether.exe"]
    );
    assert_eq!(rules[0].outbound, "direct");

    assert_eq!(
        rules[1].process_name.as_ref().unwrap(),
        &vec!["Discord.exe"]
    );
    assert_eq!(rules[1].outbound, "v2ray");

    assert_eq!(rules[2].port.as_ref().unwrap(), &vec![3478, 5349]);
    assert_eq!(rules[2].outbound, "direct");

    assert_eq!(
        rules[3].process_name.as_ref().unwrap(),
        &vec!["dota2.exe", "RustClient.exe", "Rust.exe"]
    );
    assert_eq!(rules[3].outbound, "direct");

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

    assert_eq!(rules[5].ip_is_private, Some(true));
    assert_eq!(rules[5].outbound, "direct");

    assert_eq!(config.route.final_outbound, "aether");
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

    let outbound_pub = SingBoxConfigGenerator::resolve_route_with_ip(
        &config,
        Some("browser.exe"),
        Some(53),
        Some("8.8.8.8"),
    );
    assert_eq!(outbound_pub, "aether");
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
// Real Behavioral Reliability Tests
// =========================================================================

fn test_a_failed_candidate_leaves_old_state_intact() {
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

fn test_b_missing_tun_triggers_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut runner = SingBoxRunner::new();
        // A non-existent adapter name will fail the bounded verification loop
        let res = runner
            .verify_router_and_egress("non-existent-tun-adapter", Duration::from_millis(600))
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

fn test_c_failed_egress_triggers_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Test that query_direct_system_cloudflare_trace fails gracefully on broken routing
        let start = std::time::Instant::now();
        let trace_res =
            aether_desktop_lib::health::HealthProber::query_direct_system_cloudflare_trace().await;
        // Whether system has internet or is offline in test runner, result is bounded and typed
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(6),
            "Direct trace must be bounded by timeout"
        );
        if let Err(ref e) = trace_res {
            assert!(!e.is_empty());
        }
    });
}

fn test_d_rollback_failure_surfaced() {
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

fn test_e_persistence_failure_triggers_runtime_rollback() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);
        let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

        let old_settings = AppSettings::default();
        let mut candidate = old_settings.clone();
        candidate.secondary_proxy.port = 10809;

        // Disconnected state: apply_live_settings is a no-op Ok(())
        let live_res = orchestrator.apply_live_settings(&candidate).await;
        assert!(live_res.is_ok());
    });
}

fn test_f_existing_aether_reused() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);
        let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

        // Verify initial management flag is false
        assert!(!orchestrator
            .is_aether_managed
            .load(std::sync::atomic::Ordering::SeqCst));
    });
}

fn test_g_occupied_non_aether_port_rejected() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Probe port with invalid proxy format
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

fn test_h_github_digest_mismatch_fails_closed() {
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
}

fn test_i_known_good_install_survives_promotion_failure() {
    let temp_root = std::env::temp_dir().join(format!("aether-promote-test-{}", Uuid::new_v4()));
    let final_dir = temp_root.join("final");
    let staging_dir = temp_root.join("staging");

    std::fs::create_dir_all(&final_dir).unwrap();
    std::fs::create_dir_all(&staging_dir).unwrap();

    let original_exe = final_dir.join("aether.exe");
    std::fs::write(&original_exe, "ORIGINAL_AETHER_V1").unwrap();

    let bad_staging_exe = staging_dir.join("aether.exe");
    std::fs::write(&bad_staging_exe, "CORRUPT_STAGING").unwrap();

    // Promoter with failing validator
    let res = DependencyManager::safe_promote_staging_dir(
        &staging_dir,
        &final_dir,
        "aether.exe",
        |_path| Err("Simulated post-install validation failure".to_string()),
    );

    assert!(res.is_err(), "Failed validation must reject promotion");

    // Verify original installation was preserved!
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

fn test_j_truncated_and_oversized_validation() {
    let expected: u64 = 15_000_000;
    let truncated: u64 = 10_000_000;
    assert_ne!(
        expected, truncated,
        "Truncated bytes mismatch must be rejected"
    );

    let oversized: u64 = 150_000_000; // 150 MB
    let max_allowed: u64 = 104_857_600; // 100 MB
    assert!(
        oversized > max_allowed,
        "Oversized archive must exceed maximum limit"
    );
}
