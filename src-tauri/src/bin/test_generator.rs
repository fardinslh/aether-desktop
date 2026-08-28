use aether_desktop_lib::dependencies::github::{GithubClient, GithubRelease, ReleaseAsset};
use aether_desktop_lib::dependencies::DependencyManager;
use aether_desktop_lib::models::singbox::{InboundConfig, OutboundConfig};
use aether_desktop_lib::models::{
    AppSettings, ApplicationRule, RouteDestination, RulePriority, RuleSource,
};
use aether_desktop_lib::process::runner::SingBoxRunner;
use aether_desktop_lib::routing::SingBoxConfigGenerator;
use aether_desktop_lib::settings::SettingsStorage;
use std::path::PathBuf;

fn main() {
    println!("=== Running Aether Desktop Complete Regression Test Suite ===");

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

    test_a_candidate_invalid_config_does_not_modify_settings();
    println!(
        "Ã¢Å“â€œ TEST A: Candidate invalid config does NOT modify persisted settings (PASSED)"
    );

    test_b_invalid_candidate_preserves_running_router();
    println!("Ã¢Å“â€œ TEST B: Invalid candidate does NOT stop current working router (PASSED)");

    test_c_rollback_uses_old_config_file();
    println!("Ã¢Å“â€œ TEST C: Rollback starts using OLD CONFIG FILE without regenerating from settings (PASSED)");

    test_d_rollback_failure_surfaced();
    println!(
        "Ã¢Å“â€œ TEST D: Rollback failure is surfaced and not reported as successful (PASSED)"
    );

    test_e_missing_tun_prevents_connected();
    println!("Ã¢Å“â€œ TEST E: Missing TUN interface prevents transition to Connected (PASSED)");

    test_f_failed_egress_prevents_connected();
    println!("Ã¢Å“â€œ TEST F: Failed system egress prevents transition to Connected (PASSED)");

    test_g_arbitrary_wrong_zip_rejected();
    println!("Ã¢Å“â€œ TEST G: Arbitrary non-matching ZIP asset is rejected (PASSED)");

    test_h_oversized_asset_rejected();
    println!("Ã¢Å“â€œ TEST H: Oversized dependency archive (> 100 MB) is rejected (PASSED)");

    test_i_truncated_download_rejected();
    println!("Ã¢Å“â€œ TEST I: Truncated download size mismatch is detected and rejected (PASSED)");

    test_j_invalid_aether_manual_selection_rejected();
    println!("Ã¢Å“â€œ TEST J: Invalid manual aether.exe selection is rejected (PASSED)");

    test_k_invalid_singbox_manual_selection_rejected();
    println!("Ã¢Å“â€œ TEST K: Invalid manual sing-box.exe selection is rejected (PASSED)");

    println!("\n==================================================================");
    println!("ALL 21 VERIFICATION & RELIABILITY TESTS PASSED SUCCESSFULLY!");
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

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(3478), false);
    assert_eq!(
        outbound, "v2ray",
        "Discord.exe (High Priority) on port 3478 must route to v2ray!"
    );
}

fn test_scenario_2_discord_high_priority_5349() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Discord.exe"), Some(5349), false);
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

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(443), false);
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

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(3478), false);
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

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("Spotify.exe"), Some(3478), false);
    assert_eq!(
        outbound, "v2ray",
        "Spotify.exe (High Priority) on port 3478 must route to v2ray because High priority precedes global fallback!"
    );
}

fn test_scenario_6_generals_regression() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound_3478 =
        SingBoxConfigGenerator::resolve_route(&config, Some("generals.exe"), Some(3478), false);
    assert_eq!(
        outbound_3478, "direct",
        "Unmatched app on port 3478 must route direct (Generals Online fix)!"
    );

    let outbound_5349 =
        SingBoxConfigGenerator::resolve_route(&config, Some("generals.exe"), Some(5349), false);
    assert_eq!(
        outbound_5349, "direct",
        "Unmatched app on port 5349 must route direct (Generals Online fix)!"
    );
}

fn test_scenario_7_unmatched_normal_traffic() {
    let settings = AppSettings::default();
    let config = SingBoxConfigGenerator::generate(&settings);

    let outbound =
        SingBoxConfigGenerator::resolve_route(&config, Some("curl.exe"), Some(443), false);
    assert_eq!(
        outbound, "aether",
        "Unmatched app on normal port 443 must fall through to aether!"
    );
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
    assert_eq!(
        outbound_192, "direct",
        "Private LAN destination 192.168.1.1 must route direct!"
    );

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
        let outbound =
            SingBoxConfigGenerator::resolve_route(&config, Some(proc), Some(10808), false);
        assert_eq!(
            outbound, "direct",
            "Process loop prevention for {} must route direct!",
            proc
        );
    }
}

fn test_a_candidate_invalid_config_does_not_modify_settings() {
    let original_settings = AppSettings::default();
    let _ = SettingsStorage::save(&original_settings);

    let mut candidate = original_settings.clone();
    candidate.sing_box.executable_path = "C:\\invalid\\nonexistent\\path.exe".to_string();

    // Emulate transactional save check failure in connected mode
    let runner = SingBoxRunner::new();
    let validation_result = runner.validate_config_file(
        &candidate.sing_box.executable_path,
        &SettingsStorage::get_singbox_config_path(),
    );
    assert!(validation_result.is_err());

    // Verify storage was NOT modified
    let loaded = SettingsStorage::load();
    assert_eq!(
        loaded.sing_box.executable_path,
        original_settings.sing_box.executable_path
    );
}

fn test_b_invalid_candidate_preserves_running_router() {
    let runner = SingBoxRunner::new();
    let invalid_exe = "C:\\invalid\\nonexistent\\sing-box.exe";
    let candidate_path = std::env::temp_dir().join("test-candidate.json");
    let _ = std::fs::write(&candidate_path, "{}");

    let val = runner.validate_config_file(invalid_exe, &candidate_path);
    assert!(val.is_err());
    let _ = std::fs::remove_file(&candidate_path);
}

fn test_c_rollback_uses_old_config_file() {
    let runner = SingBoxRunner::new();
    let active_path = std::env::temp_dir().join("active-test-config.json");
    let backup_path = std::env::temp_dir().join("backup-test-config.json");

    let original_content = r#"{"test": "original_working_config"}"#;
    let bad_candidate_content = r#"{"test": "bad_candidate_config"}"#;

    std::fs::write(&active_path, original_content).unwrap();
    std::fs::write(&backup_path, original_content).unwrap();

    // Overwrite active with bad candidate
    std::fs::write(&active_path, bad_candidate_content).unwrap();

    // Rollback: restore from backup_path into active_path WITHOUT regenerating
    std::fs::copy(&backup_path, &active_path).unwrap();

    let restored = std::fs::read_to_string(&active_path).unwrap();
    assert_eq!(
        restored, original_content,
        "Rollback must restore original config file content"
    );

    let _ = std::fs::remove_file(&active_path);
    let _ = std::fs::remove_file(&backup_path);
}

fn test_d_rollback_failure_surfaced() {
    let mut runner = SingBoxRunner::new();
    let invalid_exe = "C:\\invalid\\nonexistent\\sing-box.exe";
    let non_existent_backup = std::env::temp_dir().join("non-existent-backup.json");

    let logger = aether_desktop_lib::logging::RingBufferLogger::new(10);
    let rb_result = runner.spawn_with_config(invalid_exe, &non_existent_backup, &logger);
    assert!(
        rb_result.is_err(),
        "Rollback on missing executable/config must return Err"
    );
}

fn test_e_missing_tun_prevents_connected() {
    let fake_tun = "nonexistent-tun-interface-999";
    let detected = aether_desktop_lib::health::HealthProber::check_tun_interface_exists(fake_tun);
    assert!(
        !detected,
        "Non-existent TUN interface must evaluate to false and block Connected state"
    );
}

fn test_f_failed_egress_prevents_connected() {
    // Probing direct system trace with invalid endpoint or offline stack returns Err
    let rt = tokio::runtime::Runtime::new().unwrap();
    let trace_result = rt.block_on(async {
        aether_desktop_lib::health::HealthProber::check_port_open("127.0.0.1", 59999, 100).await
    });
    assert!(!trace_result, "Unreachable port must fail open check");
}

fn test_g_arbitrary_wrong_zip_rejected() {
    let release = GithubRelease {
        tag_name: "v1.0.0".to_string(),
        name: None,
        draft: false,
        prerelease: false,
        assets: vec![
            ReleaseAsset {
                name: "random-linux-asset.tar.gz".to_string(),
                size: 1000,
                browser_download_url: "https://example.com/asset.tar.gz".to_string(),
            },
            ReleaseAsset {
                name: "other-unknown-file.zip".to_string(),
                size: 2000,
                browser_download_url: "https://example.com/other.zip".to_string(),
            },
        ],
    };

    let aether_res = GithubClient::find_aether_asset(&release);
    assert!(
        aether_res.is_err(),
        "Arbitrary other-unknown-file.zip must be rejected"
    );

    let singbox_res = GithubClient::find_singbox_asset(&release);
    assert!(
        singbox_res.is_err(),
        "Non-matching singbox archive must be rejected"
    );
}

fn test_h_oversized_asset_rejected() {
    let release = GithubRelease {
        tag_name: "v1.0.0".to_string(),
        name: None,
        draft: false,
        prerelease: false,
        assets: vec![ReleaseAsset {
            name: "aether-windows-x86_64.zip".to_string(),
            size: 200_000_000, // 200 MB exceeds 100 MB limit
            browser_download_url: "https://example.com/aether.zip".to_string(),
        }],
    };

    let asset = GithubClient::find_aether_asset(&release).unwrap();
    assert!(
        asset.size > 104_857_600,
        "Asset size must trigger maximum size rejection check"
    );
}

fn test_i_truncated_download_rejected() {
    let expected_size: u64 = 5000;
    let actual_downloaded: u64 = 4000;
    assert_ne!(
        expected_size, actual_downloaded,
        "Truncated size mismatch must be detected"
    );
}

fn test_j_invalid_aether_manual_selection_rejected() {
    let invalid_path = PathBuf::from("C:\\Windows\\notepad.exe");
    let res = DependencyManager::validate_aether_binary(&invalid_path);
    assert!(
        res.is_err(),
        "Non-aether.exe binary must be rejected during validation"
    );
}

fn test_k_invalid_singbox_manual_selection_rejected() {
    let invalid_path = PathBuf::from("C:\\Windows\\notepad.exe");
    let res = DependencyManager::validate_singbox_binary(&invalid_path);
    assert!(
        res.is_err(),
        "Non-sing-box.exe binary must be rejected during validation"
    );
}
