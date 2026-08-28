use crate::models::{ApplicationRule, RouteDestination};

/// System processes that MUST NEVER be routed into TUN to prevent infinite forwarding loops.
pub const LOOP_PREVENTION_PROCESSES: &[&str] = &[
    "xray.exe",
    "v2ray.exe",
    "v2rayN.exe",
    "aether.exe",
];

/// STUN/TURN UDP/TCP ports 3478, 5349 used for Generals Online compatibility fallback
pub const GENERALS_STUN_TURN_PORTS: &[u16] = &[3478, 5349];

/// Initial preset rules populated into user settings on first launch.
/// Note: These are ordinary user-editable and removable seed records, not hardcoded constraints!
pub fn get_default_rules() -> Vec<ApplicationRule> {
    vec![
        // DIRECT PRESETS (Normal Priority)
        ApplicationRule::preset("Dota 2", "dota2.exe", RouteDestination::Direct),
        ApplicationRule::preset("Rust Client", "RustClient.exe", RouteDestination::Direct),
        ApplicationRule::preset("Rust", "Rust.exe", RouteDestination::Direct),

        // HIGH PRIORITY SECONDARY PROXY PRESET (Discord Voice STUN/TURN Override)
        ApplicationRule::preset_high("Discord", "Discord.exe", RouteDestination::SecondaryProxy),

        // NORMAL PRIORITY SECONDARY PROXY PRESETS (V2Ray / Xray)
        ApplicationRule::preset("Google Chrome", "chrome.exe", RouteDestination::SecondaryProxy),
        ApplicationRule::preset("Visual Studio Code", "Code.exe", RouteDestination::SecondaryProxy),
        ApplicationRule::preset("Codex", "codex.exe", RouteDestination::SecondaryProxy),
        ApplicationRule::preset("Antigravity App", "Antigravity.exe", RouteDestination::SecondaryProxy),
        ApplicationRule::preset("Antigravity Backend (agy)", "agy.exe", RouteDestination::SecondaryProxy),
        ApplicationRule::preset("Antigravity Language Server", "language_server.exe", RouteDestination::SecondaryProxy),
    ]
}