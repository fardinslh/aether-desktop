//! Documented standalone Aether Desktop verification suite.
//! See `.project-memory/TESTING.md` for the full protocol.
//! Run with AETHER_TEST_BUILD=1 and the LLVM-MinGW toolchain on PATH:
//! `cargo run --manifest-path src-tauri/Cargo.toml --bin test_generator`

#[path = "../../tests/common/test_suite.rs"]
mod test_suite;

use test_suite::*;
use uuid::Uuid;

fn main() {
    let isolated_config =
        std::env::temp_dir().join(format!("aether_desktop_generator_tests_{}", Uuid::new_v4()));
    std::env::set_var("AETHER_DESKTOP_CONFIG_DIR", &isolated_config);
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

    test_aj_dota2_mode_direct_routes_to_direct();
    println!("✓ TEST AJ [UNIT / ROUTING]: Dota 2 Valve SDR UDP mode Direct routes traffic to direct (PASSED)");

    test_ak_user_compatibility_rules_preserved_on_dota_save();
    println!("✓ TEST AK [UNIT / SETTINGS]: User-defined compatibility rules are preserved when saving or clearing Dota SDR settings (PASSED)");

    test_al_gateway_optimization_evaluation_decision_logic();
    println!("✓ TEST AL [UNIT / DECISION]: Gateway optimization evaluation enforces anti-noise threshold and correct keep/rollback decisions (PASSED)");

    test_am_latency_profile_median_mad_and_insufficient_samples();
    println!("✓ TEST AM [UNIT / STATS]: Multi-sample latency calculation accurately evaluates median, MAD jitter, and rejects insufficient sample sets (PASSED)");

    test_an_ring_buffer_logger_10000_capacity_and_eviction();
    println!("✓ TEST AN [UNIT / LOGGING]: RingBufferLogger enforces 10,000 entry capacity with FIFO oldest-entry eviction (PASSED)");

    test_ao_preexisting_tun_conflict_guard();
    println!("✓ TEST AO [UNIT / CONFLICT]: Pre-existing TUN adapter rejected before spawning new sing-box (PASSED)");

    test_ap_singbox_liveness_failure_during_tun_detection();
    println!("✓ TEST AP [UNIT / LIVENESS]: Inactive sing-box child fails verification immediately upon TUN detection (PASSED)");

    test_aq_singbox_liveness_failure_during_staged_egress();
    println!("✓ TEST AQ [UNIT / LIVENESS]: Child exit during staged egress verification fails connection authoritatively (PASSED)");

    test_ar_external_aether_guards_abort_optimization_before_disruption();
    println!("✓ TEST AR [UNIT / OPTIMIZATION]: External Aether connection aborts optimization before disrupting active router (PASSED)");

    test_as_tun_stabilization_window_guarantees_adapter_persistence();
    println!("✓ TEST AS [UNIT / STABILIZATION]: TUN stabilization window enforces continuous adapter and child liveness (PASSED)");

    test_at_quick_reconnect_cli_tri_state_and_forced_fresh_scan();
    println!("✓ TEST AT [UNIT / CLI]: Tri-state Quick Reconnect CLI generation (--quick-reconnect vs --no-quick-reconnect) (PASSED)");

    test_au_cached_endpoint_reuse_rejected_during_forced_fresh_scan();
    println!("✓ TEST AU [UNIT / SCAN]: Output stream parser detects and rejects cached endpoint reuse during forced fresh scan (PASSED)");

    test_av_steam_suite_routes_to_aether();
    println!("✓ TEST AV [UNIT / ROUTING]: Steam suite (steam.exe, steamwebhelper.exe, steamservice.exe) routes to Aether (PASSED)");

    test_aw_steam_companion_propagation_when_steam_customized();
    println!("✓ TEST AW [UNIT / ROUTING]: Steam companion processes automatically propagate with steam.exe custom destination (PASSED)");

    println!("\n==================================================================");
    println!("ALL 58 VERIFICATION & RELIABILITY TESTS PASSED!");
    println!("==================================================================");
}
