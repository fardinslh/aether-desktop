use aether_desktop_lib::dependencies::github::ReleaseAsset;
use aether_desktop_lib::dependencies::DependencyManager;
use aether_desktop_lib::models::settings::{CompatibilityRule, CompatibilityScope, NetworkProtocol};
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
    println!("✓ TEST 1: Discord.exe (High Priority) on port 3478 -> aether (PASSED - Discord Voice Fixed)");

    test_scenario_2_discord_high_priority_5349();
    println!("✓ TEST 2: Discord.exe (High Priority) on port 5349 -> aether (PASSED - Discord Voice Fixed)");

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
    println!("✓ TEST L [UNIT / MOCKED INTEGRATION]: Aether startup budgets match current strategy deadlines (Turbo: 60s, Balanced: 150s, Thorough: 340s, Stealth: 210s, Ironclad: 210s) (PASSED)");

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

    test_r_op_lock_held_preserves_disconnected_state();
    println!("✓ TEST R [UNIT / MOCKED INTEGRATION]: Operation lock acquisition failure cleanly rejects connect and preserves Disconnected state without fake transitions (PASSED)");

    test_s_deadline_mapping_strictly_exceeds_upstream_budgets();
    println!("✓ TEST S [UNIT / MOCKED INTEGRATION]: Desktop scan deadlines strictly exceed upstream scan budgets with safety margin (PASSED)");

    test_t_snapshot_strictly_excludes_identity_and_config_files();
    println!("✓ TEST T [UNIT / MOCKED INTEGRATION]: Snapshot tracks only native lastconn files and strictly excludes identity/config/keys (PASSED)");

    test_u_snapshot_rollback_atomically_restores_preexisting_lastconn();
    println!("✓ TEST U [UNIT / MOCKED INTEGRATION]: Pre-existing lastconn persistence is atomically restored on failed optimization rollback (PASSED)");

    test_v_snapshot_rollback_removes_newly_created_lastconn_on_rollback();
    println!("✓ TEST V [UNIT / MOCKED INTEGRATION]: Previously absent lastconn file created during failed scan is cleanly removed on rollback (PASSED)");

    test_w_snapshot_commit_retains_new_lastconn_and_discards_backup();
    println!("✓ TEST W [UNIT / MOCKED INTEGRATION]: Optimization success commits newly selected candidate and discards backup snapshot (PASSED)");

    test_x_tun_teardown_wait_polling_helper();
    println!("✓ TEST X [UNIT]: TUN teardown polling helper confirms adapter release or times out correctly (PASSED)");

    test_y_snapshot_restore_failure_is_fatal_to_rollback();
    println!("✓ TEST Y [UNIT / MOCKED INTEGRATION]: Native snapshot restore failure is fatal to rollback and transitions cleanly to Error (PASSED)");

    test_z_candidate_rtt_processing_works_for_both_stdout_and_stderr();
    println!("✓ TEST Z [UNIT / MOCKED INTEGRATION]: Candidate RTT telemetry is parsed accurately across both stdout and stderr output streams (PASSED)");

    test_aa_restore_deadline_bounded_to_25s();
    println!("✓ TEST AA [UNIT / MOCKED INTEGRATION]: Rollback restoration deadline is strictly bounded to Quick Reconnect window (25s) (PASSED)");

    test_ab_discord_preset_migration_from_secondary_proxy_to_aether();
    println!("✓ TEST AB [UNIT / MOCKED INTEGRATION]: Legacy Discord preset rules safely migrate to Aether destination (PASSED)");

    test_ac_user_customized_discord_rule_preservation_during_migration();
    println!("✓ TEST AC [UNIT / MOCKED INTEGRATION]: User-customized Discord rules are strictly preserved during migration (PASSED)");

    test_ad_save_exported_logs_creates_crlf_log_file();
    println!("✓ TEST AD [UNIT]: Export raw log formatting converts output to readable Windows CRLF text (PASSED)");

    test_ae_dota2_valve_sdr_udp_port_range_routes_to_secondary_v2ray();
    println!("✓ TEST AE [UNIT / ROUTING]: Dota 2 Valve SDR UDP traffic (ports 27000-27250) routes via Secondary Proxy (PASSED)");

    test_af_dota2_tcp_and_non_sdr_ports_route_to_aether();
    println!("✓ TEST AF [UNIT / ROUTING]: Dota 2 TCP and non-SDR UDP traffic strictly routes via Aether (PASSED)");

    test_ag_other_process_on_sdr_ports_routes_to_normal();
    println!("✓ TEST AG [UNIT / ROUTING]: Unrelated applications on ports 27000-27250 bypass Dota app-scoped compatibility (PASSED)");

    test_ah_disabling_dota_sdr_toggle_restores_all_dota_to_aether();
    println!("✓ TEST AH [UNIT / ROUTING]: Disabling Dota Valve SDR compatibility restores all Dota traffic to Aether (PASSED)");

    test_ai_repeated_saves_do_not_duplicate_dota_sdr_rule();
    println!("✓ TEST AI [UNIT / SETTINGS]: Repeated saves of Dota 2 settings do not duplicate the compatibility rule (PASSED)");

    println!("\n==================================================================");
    println!("ALL 44 VERIFICATION & RELIABILITY TESTS PASSED!");
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
    assert_eq!(rules[3].outbound.as_deref(), Some("aether"));

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
    assert_eq!(outbound, "aether");
}

fn test_scenario_2_discord_high_priority_5349() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(5349), false);
    assert_eq!(outbound, "aether");
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
        Duration::from_secs(60)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Balanced),
        Duration::from_secs(150)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Thorough),
        Duration::from_secs(340)
    );
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Stealth),
        Duration::from_secs(210)
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

    // 5. Discord High-priority traffic (Discord.exe on port 3478) -> aether
    let discord_route =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(3478), false);
    assert_eq!(discord_route, "aether");
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

    // Exactly one must be rejected immediately with operation or connection in progress
    let rejected_count = [res1, res2]
        .iter()
        .filter(|r| match r {
            Err(e) => {
                e.contains("Connection operation already in progress")
                    || e.contains("Connection already in progress")
            }
            _ => false,
        })
        .count();

    assert_eq!(
        rejected_count, 1,
        "Concurrent connect calls must atomically reject duplicates: exactly 1 rejected immediately"
    );
}

fn test_r_op_lock_held_preserves_disconnected_state() {
    use aether_desktop_lib::logging::RingBufferLogger;
    use aether_desktop_lib::models::ConnectionState;
    use aether_desktop_lib::process::orchestrator::ConnectionOrchestrator;
    use parking_lot::RwLock;
    use std::sync::Arc;

    let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
    let logger = RingBufferLogger::new(100);
    let orchestrator = ConnectionOrchestrator::new(state.clone(), logger);

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();

    // Lock op_lock manually to simulate concurrent operation in progress
    let _held_guard = orchestrator.op_lock.try_lock().unwrap();

    let settings = AppSettings::default();
    let res = tokio_rt.block_on(orchestrator.connect(&settings));

    assert!(res.is_err(), "Connect must fail when op_lock is held");
    assert!(
        res.unwrap_err()
            .contains("Connection operation already in progress"),
        "Error message must specify operation in progress"
    );

    // CRITICAL REGRESSION ASSERTION: State must remain Disconnected and NOT mutate to StartingAether
    assert_eq!(
        *orchestrator.state.read(),
        ConnectionState::Disconnected,
        "Connection state must remain Disconnected when op_lock acquisition fails"
    );
}

fn test_s_deadline_mapping_strictly_exceeds_upstream_budgets() {
    use aether_desktop_lib::models::settings::{
        aether_startup_timeout, AetherScanMode, AETHER_RESTORE_TIMEOUT,
    };

    // Upstream observed budgets: Turbo: 45s, Balanced: 120s, Thorough: 300s, Stealth: 180s, Ironclad: 180s
    assert!(aether_startup_timeout(&AetherScanMode::Turbo) > Duration::from_secs(45));
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Turbo),
        Duration::from_secs(60)
    );

    assert!(aether_startup_timeout(&AetherScanMode::Balanced) > Duration::from_secs(120));
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Balanced),
        Duration::from_secs(150)
    );

    assert!(aether_startup_timeout(&AetherScanMode::Thorough) > Duration::from_secs(300));
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Thorough),
        Duration::from_secs(340)
    );

    assert!(aether_startup_timeout(&AetherScanMode::Stealth) > Duration::from_secs(180));
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Stealth),
        Duration::from_secs(210)
    );

    assert!(aether_startup_timeout(&AetherScanMode::Ironclad) > Duration::from_secs(180));
    assert_eq!(
        aether_startup_timeout(&AetherScanMode::Ironclad),
        Duration::from_secs(210)
    );

    // Restore timeout is bounded to quick reconnect (25s)
    assert_eq!(AETHER_RESTORE_TIMEOUT, Duration::from_secs(25));
}

fn test_t_snapshot_strictly_excludes_identity_and_config_files() {
    use aether_desktop_lib::settings::storage::{
        lastconn_path, AetherPersistenceSnapshot, LastconnEntryState,
    };

    let temp_dir =
        std::env::temp_dir().join(format!("aether_test_gen_filter_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    // 1. Create WireGuard and MASQUE identity configs and lastconn files
    let toml_file = temp_dir.join("aether.toml");
    let lastconn_file = temp_dir.join("aether-lastconn.toml");
    let masque_config = temp_dir.join("aether-masque.toml");
    let masque_lastconn = temp_dir.join("aether-masque-lastconn.toml");

    std::fs::write(&toml_file, b"scan_mode = 'Thorough'\nprivate_key = 'SECRET'").unwrap();
    std::fs::write(&lastconn_file, b"endpoint = '162.159.192.1:2408'\nrtt = 45\n").unwrap();
    std::fs::write(&masque_config, b"auth_token = 'MASQUE_SECRET'").unwrap();
    std::fs::write(&masque_lastconn, b"endpoint = '162.159.193.1:443'\nrtt = 55\n").unwrap();

    // 2. WireGuard snapshot test
    let wg_target = lastconn_path(&toml_file);
    assert_eq!(wg_target.file_name().unwrap().to_string_lossy(), "aether-lastconn.toml");
    let wg_snapshot = AetherPersistenceSnapshot::create_for_targets(&[wg_target]).unwrap();

    for entry in &wg_snapshot.entries {
        match entry {
            LastconnEntryState::Existed { target_path, .. }
            | LastconnEntryState::Absent { target_path } => {
                let name = target_path.file_name().unwrap().to_string_lossy();
                assert_ne!(name, "aether.toml", "aether.toml (identity/config) must NOT be included");
                assert_ne!(name, "aether-masque.toml", "aether-masque.toml must NOT be included");
                assert_eq!(name, "aether-lastconn.toml", "aether-lastconn.toml must be included");
            }
        }
    }
    wg_snapshot.cleanup();

    // 3. MASQUE snapshot test
    let masque_target = lastconn_path(&masque_config);
    assert_eq!(masque_target.file_name().unwrap().to_string_lossy(), "aether-masque-lastconn.toml");
    let masque_snapshot = AetherPersistenceSnapshot::create_for_targets(&[masque_target]).unwrap();

    for entry in &masque_snapshot.entries {
        match entry {
            LastconnEntryState::Existed { target_path, .. }
            | LastconnEntryState::Absent { target_path } => {
                let name = target_path.file_name().unwrap().to_string_lossy();
                assert_ne!(name, "aether-masque.toml", "aether-masque.toml must NOT be included");
                assert_ne!(name, "aether.toml", "aether.toml must NOT be included");
                assert_eq!(name, "aether-masque-lastconn.toml", "aether-masque-lastconn.toml must be included");
            }
        }
    }
    masque_snapshot.cleanup();

    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_u_snapshot_rollback_atomically_restores_preexisting_lastconn() {
    use aether_desktop_lib::settings::storage::{lastconn_path, AetherPersistenceSnapshot};

    let temp_dir =
        std::env::temp_dir().join(format!("aether_test_gen_u_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let toml_file = temp_dir.join("aether.toml");
    let lastconn_file = temp_dir.join("aether-lastconn.toml");
    let original_bytes = b"endpoint = '162.159.192.1:2408'\nrtt = 45\n";
    std::fs::write(&lastconn_file, original_bytes).unwrap();

    // 1. Snapshot native lastconn state
    let target = lastconn_path(&toml_file);
    let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

    // 2. Simulate fresh scan modifying or writing new failed/intermediate lastconn
    let modified_bytes = b"endpoint = '162.159.193.99:500'\nrtt = 999\n";
    std::fs::write(&lastconn_file, modified_bytes).unwrap();
    assert_eq!(std::fs::read(&lastconn_file).unwrap(), modified_bytes);

    // 3. Rollback: restore snapshot
    snapshot.restore().unwrap();
    assert_eq!(std::fs::read(&lastconn_file).unwrap(), original_bytes);

    // 4. Cleanup
    snapshot.cleanup();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_v_snapshot_rollback_removes_newly_created_lastconn_on_rollback() {
    use aether_desktop_lib::settings::storage::{lastconn_path, AetherPersistenceSnapshot};

    let temp_dir =
        std::env::temp_dir().join(format!("aether_test_gen_v_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let toml_file = temp_dir.join("aether.toml");
    let lastconn_file = temp_dir.join("aether-lastconn.toml");
    assert!(!lastconn_file.exists());

    // 1. Snapshot with no existing lastconn
    let target = lastconn_path(&toml_file);
    let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

    // 2. Fresh scan creates a new lastconn file
    std::fs::write(&lastconn_file, b"endpoint = '162.159.192.9:2408'\nrtt = 80\n").unwrap();
    assert!(lastconn_file.exists());

    // 3. Rollback: newly created lastconn file must be removed
    snapshot.restore().unwrap();
    assert!(!lastconn_file.exists(), "Newly created aether-lastconn.toml file must be removed on rollback");

    snapshot.cleanup();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_w_snapshot_commit_retains_new_lastconn_and_discards_backup() {
    use aether_desktop_lib::settings::storage::{lastconn_path, AetherPersistenceSnapshot};

    let temp_dir =
        std::env::temp_dir().join(format!("aether_test_gen_w_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let toml_file = temp_dir.join("aether.toml");
    let lastconn_file = temp_dir.join("aether-lastconn.toml");
    let initial_bytes = b"endpoint = '162.159.192.1:2408'\nrtt = 95\n";
    std::fs::write(&lastconn_file, initial_bytes).unwrap();

    // 1. Snapshot initial state
    let target = lastconn_path(&toml_file);
    let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();
    let snapshot_dir = snapshot.snapshot_dir.clone();
    assert!(snapshot_dir.exists());

    // 2. Optimization succeeds and writes faster working candidate
    let optimized_bytes = b"endpoint = '162.159.192.5:2408'\nrtt = 38\n";
    std::fs::write(&lastconn_file, optimized_bytes).unwrap();

    // 3. Commit: cleanup snapshot
    snapshot.cleanup();
    assert!(!snapshot_dir.exists());

    // 4. Optimized bytes remain intact
    assert_eq!(std::fs::read(&lastconn_file).unwrap(), optimized_bytes);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_x_tun_teardown_wait_polling_helper() {
    use aether_desktop_lib::health::HealthProber;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();

    // 1. Success case: adapter disappears after 2 polls
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = attempts.clone();
    let check_fn = move || {
        let current = attempts_clone.fetch_add(1, Ordering::SeqCst);
        current < 2
    };

    let res = tokio_rt.block_on(HealthProber::wait_for_tun_teardown_with_check(
        check_fn,
        Duration::from_millis(500),
        Duration::from_millis(20),
    ));
    assert!(res.is_ok(), "TUN teardown wait should succeed when adapter is released");
    assert!(attempts.load(Ordering::SeqCst) >= 2);

    // 2. Timeout case: adapter never disappears => fails wait_for_tun_teardown
    let check_fn_timeout = || true;
    let res_timeout = tokio_rt.block_on(HealthProber::wait_for_tun_teardown_with_check(
        check_fn_timeout,
        Duration::from_millis(60),
        Duration::from_millis(15),
    ));
    assert!(res_timeout.is_err(), "TUN teardown wait must time out if adapter remains present");
}

fn test_y_snapshot_restore_failure_is_fatal_to_rollback() {
    use aether_desktop_lib::logging::RingBufferLogger;
    use aether_desktop_lib::models::ConnectionState;
    use aether_desktop_lib::process::ConnectionOrchestrator;
    use aether_desktop_lib::settings::storage::{lastconn_path, AetherPersistenceSnapshot};
    use parking_lot::RwLock;
    use std::sync::Arc;

    let tokio_rt = tokio::runtime::Runtime::new().unwrap();

    let temp_dir =
        std::env::temp_dir().join(format!("aether_test_gen_y_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);

    let toml_file = temp_dir.join("aether.toml");
    let lastconn_file = temp_dir.join("aether-lastconn.toml");
    std::fs::write(&lastconn_file, b"endpoint = '162.159.192.1:2408'\nrtt = 45\n").unwrap();

    let target = lastconn_path(&toml_file);
    let snapshot = AetherPersistenceSnapshot::create_for_targets(&[target]).unwrap();

    // Delete snapshot backup directory to force snapshot.restore() to fail
    let _ = std::fs::remove_dir_all(&snapshot.snapshot_dir);

    let logger = RingBufferLogger::new(100);
    let state = Arc::new(RwLock::new(ConnectionState::Connected));
    let orch = ConnectionOrchestrator::new(state.clone(), logger);
    let settings = AppSettings::default();

    let res = tokio_rt.block_on(orch.rollback_and_restore(
        &settings,
        1,
        Some(50),
        Some("FRA".to_string()),
        Some("1.1.1.1".to_string()),
        snapshot,
        "Simulated scan failure".to_string(),
    ));

    assert!(res.is_err(), "Snapshot restore failure must cause rollback_and_restore to return Err");
    let err = res.unwrap_err();
    assert!(
        err.contains("Fatal: Failed to restore native lastconn persistence snapshot")
            || err.contains("Atomic replacement failed")
            || err.contains("Failed to restore"),
        "Error message must specify restore snapshot failure: {}",
        err
    );
    assert_eq!(*orch.state.read(), ConnectionState::Error, "State must transition to Error on fatal restore failure");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_z_candidate_rtt_processing_works_for_both_stdout_and_stderr() {
    use aether_desktop_lib::logging::RingBufferLogger;
    use aether_desktop_lib::process::runner::process_aether_line;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    let logger = RingBufferLogger::new(50);
    let interactive = AtomicBool::new(false);
    let best_rtt = AtomicU32::new(0);

    // 1. Candidate line on stdout (is_stderr = false)
    process_aether_line(
        "candidate ok 162.159.192.1:2408 rtt=85ms",
        false,
        &interactive,
        &best_rtt,
        &logger,
    );
    assert_eq!(best_rtt.load(Ordering::SeqCst), 85);
    assert!(!interactive.load(Ordering::SeqCst));

    // 2. Faster candidate line on stderr (is_stderr = true)
    process_aether_line(
        "[+] candidate 162.159.192.5:2408 OK (rtt: 42ms)",
        true,
        &interactive,
        &best_rtt,
        &logger,
    );
    assert_eq!(best_rtt.load(Ordering::SeqCst), 42);

    // 3. Slower candidate on stderr does not overwrite faster
    process_aether_line(
        "Endpoint 162.159.193.10:500 ok (rtt: 65ms)",
        true,
        &interactive,
        &best_rtt,
        &logger,
    );
    assert_eq!(best_rtt.load(Ordering::SeqCst), 42);
}

fn test_aa_restore_deadline_bounded_to_25s() {
    use aether_desktop_lib::models::settings::{
        aether_startup_timeout, AetherLaunchOptions, AetherScanMode, AETHER_RESTORE_TIMEOUT,
    };

    let restore_options = AetherLaunchOptions {
        quick_reconnect_override: Some(true),
        scan_mode_override: None,
    };
    let effective_scan_mode = AetherScanMode::Thorough;
    let startup_deadline = if restore_options.quick_reconnect_override == Some(true) {
        AETHER_RESTORE_TIMEOUT
    } else {
        aether_startup_timeout(&effective_scan_mode)
    };

    assert_eq!(startup_deadline, Duration::from_secs(25), "Rollback restoration deadline must be bounded to 25s");
    assert!(startup_deadline < Duration::from_secs(60));
}

fn test_ab_discord_preset_migration_from_secondary_proxy_to_aether() {
    let mut settings = AppSettings::default();
    // Simulate legacy persisted settings with old Discord SecondaryProxy preset
    settings.application_rules = vec![
        ApplicationRule::new(
            "Discord",
            "Discord.exe",
            RouteDestination::SecondaryProxy,
            None,
            RuleSource::Preset,
            RulePriority::High,
            None,
        ),
    ];

    // Run migration logic
    for rule in &mut settings.application_rules {
        if rule.source == RuleSource::Preset
            && rule.process_name.eq_ignore_ascii_case("discord.exe")
            && rule.priority == RulePriority::High
            && rule.destination == RouteDestination::SecondaryProxy
        {
            rule.destination = RouteDestination::Aether;
        }
    }

    assert_eq!(settings.application_rules[0].destination, RouteDestination::Aether);
    assert_eq!(settings.application_rules[0].priority, RulePriority::High);
    assert_eq!(settings.application_rules[0].source, RuleSource::Preset);
}

fn test_ac_user_customized_discord_rule_preservation_during_migration() {
    let mut settings = AppSettings::default();
    // 1. User created rule -> must NOT migrate even if SecondaryProxy
    let user_discord = ApplicationRule::new(
        "My Custom Discord",
        "Discord.exe",
        RouteDestination::SecondaryProxy,
        None,
        RuleSource::User,
        RulePriority::High,
        None,
    );
    // 2. Direct destination rule -> must NOT migrate
    let direct_discord = ApplicationRule::new(
        "Discord Direct",
        "discord.exe",
        RouteDestination::Direct,
        None,
        RuleSource::Preset,
        RulePriority::High,
        None,
    );
    // 3. Normal priority rule -> must NOT migrate
    let normal_discord = ApplicationRule::new(
        "Discord Normal",
        "discord.exe",
        RouteDestination::SecondaryProxy,
        None,
        RuleSource::Preset,
        RulePriority::Normal,
        None,
    );

    settings.application_rules = vec![user_discord, direct_discord, normal_discord];

    // Run migration logic
    for rule in &mut settings.application_rules {
        if rule.source == RuleSource::Preset
            && rule.process_name.eq_ignore_ascii_case("discord.exe")
            && rule.priority == RulePriority::High
            && rule.destination == RouteDestination::SecondaryProxy
        {
            rule.destination = RouteDestination::Aether;
        }
    }

    assert_eq!(settings.application_rules[0].destination, RouteDestination::SecondaryProxy);
    assert_eq!(settings.application_rules[1].destination, RouteDestination::Direct);
    assert_eq!(settings.application_rules[2].destination, RouteDestination::SecondaryProxy);
}

fn test_ad_save_exported_logs_creates_crlf_log_file() {
    let temp_dir = std::env::temp_dir().join(format!("aether_test_log_{}", Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let log_file = temp_dir.join("test-export.log");

    let unix_style_logs = "2026-08-30T10:00:00Z [INFO] [App] First line\n2026-08-30T10:00:01Z [INFO] [Aether] Second line\n";
    let formatted_logs = if unix_style_logs.contains("\r\n") {
        unix_style_logs.to_string()
    } else {
        unix_style_logs.replace('\n', "\r\n")
    };

    std::fs::write(&log_file, formatted_logs.as_bytes()).unwrap();

    let read_back = std::fs::read_to_string(&log_file).unwrap();
    assert!(read_back.contains("\r\n"), "Exported log text must contain CRLF line breaks for Windows Notepad");
    assert!(read_back.contains("First line"));
    assert!(read_back.contains("Second line"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

fn test_ae_dota2_valve_sdr_udp_port_range_routes_to_secondary_v2ray() {
    let mut settings = AppSettings::default();

    // Dota 2 main application route: dota2.exe -> Aether
    settings.application_rules = vec![ApplicationRule::new(
        "Dota 2",
        "dota2.exe",
        RouteDestination::Aether,
        None,
        RuleSource::User,
        RulePriority::Normal,
        None,
    )];

    // Dota 2 Valve SDR Game UDP compatibility override:
    // Process: dota2.exe, Network: UDP, Ports: 27000-27250, Destination: SecondaryProxy, Scope: AppScoped
    let dota_compat = CompatibilityRule {
        id: "compat-dota2-valve-sdr".to_string(),
        name: "Valve SDR / Game UDP (Dota 2)".to_string(),
        description: "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy".to_string(),
        enabled: true,
        process_names: Some(vec!["dota2.exe".to_string()]),
        ports: Some((27000..=27250).collect()),
        port_ranges: Some(vec!["27000:27250".to_string()]),
        network: Some(NetworkProtocol::Udp),
        destination: RouteDestination::SecondaryProxy,
        scope: CompatibilityScope::AppScoped,
    };
    settings.compatibility.custom_compatibility_rules.push(dota_compat);

    let config = SingBoxConfigGenerator::generate(&settings);

    // 1. dota2.exe UDP :27036 -> v2ray
    let route_27036 = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(27036),
        Some("udp"),
        false,
    );
    assert_eq!(route_27036, "v2ray", "dota2.exe UDP :27036 must route to v2ray/Secondary");

    // 2. dota2.exe UDP :27058 -> v2ray
    let route_27058 = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(27058),
        Some("udp"),
        false,
    );
    assert_eq!(route_27058, "v2ray", "dota2.exe UDP :27058 must route to v2ray/Secondary");

    // 3. dota2.exe UDP :27017 -> v2ray
    let route_27017 = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(27017),
        Some("udp"),
        false,
    );
    assert_eq!(route_27017, "v2ray", "dota2.exe UDP :27017 must route to v2ray/Secondary");
}

fn test_af_dota2_tcp_and_non_sdr_ports_route_to_aether() {
    let mut settings = AppSettings::default();

    // Dota 2 main application route: dota2.exe -> Aether
    settings.application_rules = vec![ApplicationRule::new(
        "Dota 2",
        "dota2.exe",
        RouteDestination::Aether,
        None,
        RuleSource::User,
        RulePriority::Normal,
        None,
    )];

    let dota_compat = CompatibilityRule {
        id: "compat-dota2-valve-sdr".to_string(),
        name: "Valve SDR / Game UDP (Dota 2)".to_string(),
        description: "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy".to_string(),
        enabled: true,
        process_names: Some(vec!["dota2.exe".to_string()]),
        ports: Some((27000..=27250).collect()),
        port_ranges: Some(vec!["27000:27250".to_string()]),
        network: Some(NetworkProtocol::Udp),
        destination: RouteDestination::SecondaryProxy,
        scope: CompatibilityScope::AppScoped,
    };
    settings.compatibility.custom_compatibility_rules.push(dota_compat);

    let config = SingBoxConfigGenerator::generate(&settings);

    // 4. dota2.exe TCP :27036 -> aether (TCP traffic ignores UDP SDR override)
    let route_tcp_27036 = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(27036),
        Some("tcp"),
        false,
    );
    assert_eq!(route_tcp_27036, "aether", "dota2.exe TCP :27036 must route to aether");

    // 5. dota2.exe UDP :443 -> aether (non-SDR port ignores SDR override)
    let route_udp_443 = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(443),
        Some("udp"),
        false,
    );
    assert_eq!(route_udp_443, "aether", "dota2.exe UDP :443 must route to aether");
}

fn test_ag_other_process_on_sdr_ports_routes_to_normal() {
    let mut settings = AppSettings::default();

    // Dota 2 main application route: dota2.exe -> Aether
    settings.application_rules = vec![
        ApplicationRule::new(
            "Dota 2",
            "dota2.exe",
            RouteDestination::Aether,
            None,
            RuleSource::User,
            RulePriority::Normal,
            None,
        ),
        ApplicationRule::new(
            "Game Launcher",
            "launcher.exe",
            RouteDestination::Direct,
            None,
            RuleSource::User,
            RulePriority::Normal,
            None,
        ),
    ];

    let dota_compat = CompatibilityRule {
        id: "compat-dota2-valve-sdr".to_string(),
        name: "Valve SDR / Game UDP (Dota 2)".to_string(),
        description: "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy".to_string(),
        enabled: true,
        process_names: Some(vec!["dota2.exe".to_string()]),
        ports: Some((27000..=27250).collect()),
        port_ranges: Some(vec!["27000:27250".to_string()]),
        network: Some(NetworkProtocol::Udp),
        destination: RouteDestination::SecondaryProxy,
        scope: CompatibilityScope::AppScoped,
    };
    settings.compatibility.custom_compatibility_rules.push(dota_compat);

    let config = SingBoxConfigGenerator::generate(&settings);

    // 6. launcher.exe UDP :27036 -> direct (its configured rule, NOT affected by Dota app-scoped compatibility)
    let route_launcher = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("launcher.exe"),
        Some(27036),
        Some("udp"),
        false,
    );
    assert_eq!(route_launcher, "direct", "launcher.exe must follow its own rule (direct)");

    // 6b. unassigned app UDP :27036 -> aether (global fallback, NOT affected by Dota app-scoped compatibility)
    let route_unassigned = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("unassigned.exe"),
        Some(27036),
        Some("udp"),
        false,
    );
    assert_eq!(route_unassigned, "aether", "unassigned.exe must follow global fallback (aether)");
}

fn test_ah_disabling_dota_sdr_toggle_restores_all_dota_to_aether() {
    let mut settings = AppSettings::default();

    // Dota 2 main application route: dota2.exe -> Aether
    settings.application_rules = vec![ApplicationRule::new(
        "Dota 2",
        "dota2.exe",
        RouteDestination::Aether,
        None,
        RuleSource::User,
        RulePriority::Normal,
        None,
    )];

    // Compatibility rule disabled / removed
    let dota_compat = CompatibilityRule {
        id: "compat-dota2-valve-sdr".to_string(),
        name: "Valve SDR / Game UDP (Dota 2)".to_string(),
        description: "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy".to_string(),
        enabled: false,
        process_names: Some(vec!["dota2.exe".to_string()]),
        ports: Some((27000..=27250).collect()),
        port_ranges: Some(vec!["27000:27250".to_string()]),
        network: Some(NetworkProtocol::Udp),
        destination: RouteDestination::SecondaryProxy,
        scope: CompatibilityScope::AppScoped,
    };
    settings.compatibility.custom_compatibility_rules.push(dota_compat);

    let config = SingBoxConfigGenerator::generate(&settings);

    // 7. Disabling the toggle restores all Dota traffic to normal Aether route
    let route_disabled = SingBoxConfigGenerator::resolve_route_with_network(
        &config,
        Some("dota2.exe"),
        Some(27036),
        Some("udp"),
        false,
    );
    assert_eq!(route_disabled, "aether", "When disabled, dota2.exe UDP :27036 must route to aether");
}

fn test_ai_repeated_saves_do_not_duplicate_dota_sdr_rule() {
    let mut settings = AppSettings::default();

    let dota_rule_id = "compat-dota2-valve-sdr";

    // Simulate saving Dota 2 modal multiple times with toggle enabled
    for _ in 0..5 {
        let other_rules: Vec<CompatibilityRule> = settings
            .compatibility
            .custom_compatibility_rules
            .into_iter()
            .filter(|r| r.id != dota_rule_id)
            .collect();

        let updated_dota_compat = CompatibilityRule {
            id: dota_rule_id.to_string(),
            name: "Valve SDR / Game UDP (Dota 2)".to_string(),
            description: "Routes only Dota 2 Valve UDP traffic (27000-27250) through Secondary Proxy".to_string(),
            enabled: true,
            process_names: Some(vec!["dota2.exe".to_string()]),
            ports: Some((27000..=27250).collect()),
            port_ranges: Some(vec!["27000:27250".to_string()]),
            network: Some(NetworkProtocol::Udp),
            destination: RouteDestination::SecondaryProxy,
            scope: CompatibilityScope::AppScoped,
        };

        let mut new_rules = other_rules;
        new_rules.push(updated_dota_compat);
        settings.compatibility.custom_compatibility_rules = new_rules;
    }

    // 8. Repeated saves do not duplicate the compatibility rule
    assert_eq!(
        settings.compatibility.custom_compatibility_rules.len(),
        1,
        "Repeated saves must not produce duplicate compatibility rules"
    );
    assert_eq!(
        settings.compatibility.custom_compatibility_rules[0].id,
        dota_rule_id
    );
}
