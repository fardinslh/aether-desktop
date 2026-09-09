//! Integration registration of the shared Aether Desktop verification suite.
//!
//! Every case delegates to `tests/common/test_suite.rs`, the single source of
//! truth also used by `src/bin/test_generator.rs`, so expectations cannot
//! drift between the cargo test harness and the documented standalone runner.

// The harness registers a subset of the shared suite; the remaining
// functions are exercised by the standalone `test_generator` bin, which
// compiles the same module, so dead-code analysis here is expected noise.
#[path = "common/test_suite.rs"]
#[allow(dead_code)]
mod test_suite;

use std::sync::Once;

static CONFIG_ISOLATION: Once = Once::new();

fn ensure_isolated_config_dir() {
    // SettingsStorage resolves its directory from the process environment,
    // so the isolated config dir must be installed before any storage-backed
    // case runs. Once::call_once blocks every wrapper until the variable is
    // set, which keeps parallel harness threads from touching the real user
    // settings and removes the old dependency on a sequential legacy main.
    CONFIG_ISOLATION.call_once(|| {
        let isolated_config = std::env::temp_dir().join(format!(
            "aether_desktop_integration_tests_{}",
            uuid::Uuid::new_v4()
        ));
        std::env::set_var("AETHER_DESKTOP_CONFIG_DIR", &isolated_config);
        println!(
            "Isolated AETHER_DESKTOP_CONFIG_DIR: {}",
            isolated_config.display()
        );
    });
}

macro_rules! integration_case {
    ($name:ident) => {
        #[test]
        fn $name() {
            ensure_isolated_config_dir();
            test_suite::$name();
        }
    };
}

integration_case!(test_reference_config_match);
integration_case!(test_scenario_1_discord_high_priority_3478);
integration_case!(test_scenario_2_discord_high_priority_5349);
integration_case!(test_scenario_3_normal_secondary_proxy);
integration_case!(test_scenario_4_global_compatibility_fallback_against_normal_rule);
integration_case!(test_scenario_5_high_custom_override);
integration_case!(test_scenario_6_generals_regression);
integration_case!(test_scenario_7_unmatched_normal_traffic);
integration_case!(test_scenario_8_private_lan);
integration_case!(test_scenario_9_proxy_loop_prevention);
integration_case!(test_a_candidate_validation_failure_preserves_old_state);
integration_case!(test_b_missing_tun_triggers_verified_rollback);
integration_case!(test_c_failed_egress_ip_mismatch_triggers_rollback);
integration_case!(test_d_rollback_failure_surfaced_as_critical);
integration_case!(test_e_persistence_failure_runtime_rollback);
integration_case!(test_f_existing_aether_reuse_decision_path);
integration_case!(test_g_occupied_wrong_port_owner_rejection);
integration_case!(test_h_github_digest_integrity_verification);
integration_case!(test_i_safe_staging_promotion_restore_and_verification);
integration_case!(test_j_download_size_and_truncation_guards);
integration_case!(test_k_aether_noninteractive_launch_arguments);
integration_case!(test_l_aether_scan_mode_startup_deadlines);
integration_case!(test_m_native_windows_tun_detection_by_ip_and_name);
integration_case!(test_n_process_elevation_token_check);
integration_case!(test_o_dns_hijack_infrastructure_overrides_private_lan_rule);
integration_case!(test_p_staged_egress_decision_path_mocked_suite);
integration_case!(test_q_concurrent_connect_atomic_single_attempt);
integration_case!(test_r_op_lock_held_preserves_disconnected_state);
