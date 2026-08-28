use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteDestination {
    Direct,
    SecondaryProxy,
    Aether,
}

impl Default for RouteDestination {
    fn default() -> Self {
        Self::Aether
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleSource {
    Preset,
    User,
}

impl Default for RuleSource {
    fn default() -> Self {
        Self::User
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RulePriority {
    Normal,
    High,
}

impl Default for RulePriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRule {
    pub id: String,
    #[serde(alias = "name")]
    pub display_name: String,
    pub executable_path: Option<String>,
    pub process_name: String,
    #[serde(alias = "route")]
    pub destination: RouteDestination,
    pub enabled: bool,
    #[serde(default)]
    pub source: RuleSource,
    #[serde(default)]
    pub priority: RulePriority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_base64: Option<String>,
}

impl ApplicationRule {
    pub fn new(
        display_name: impl Into<String>,
        process_name: impl Into<String>,
        destination: RouteDestination,
        executable_path: Option<String>,
        source: RuleSource,
        priority: RulePriority,
        icon_base64: Option<String>,
    ) -> Self {
        let process_name_str = process_name.into();
        let normalized_process = if process_name_str.to_lowercase().ends_with(".exe") {
            process_name_str
        } else {
            format!("{}.exe", process_name_str)
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: display_name.into(),
            executable_path,
            process_name: normalized_process,
            destination,
            enabled: true,
            source,
            priority,
            icon_base64,
        }
    }

    /// Normal-priority preset constructor (default for presets)
    pub fn preset(
        display_name: impl Into<String>,
        process_name: impl Into<String>,
        destination: RouteDestination,
    ) -> Self {
        Self::new(
            display_name,
            process_name,
            destination,
            None,
            RuleSource::Preset,
            RulePriority::Normal,
            None,
        )
    }

    /// High-priority preset constructor (e.g. for Discord.exe to override global STUN/TURN Direct fallback)
    pub fn preset_high(
        display_name: impl Into<String>,
        process_name: impl Into<String>,
        destination: RouteDestination,
    ) -> Self {
        Self::new(
            display_name,
            process_name,
            destination,
            None,
            RuleSource::Preset,
            RulePriority::High,
            None,
        )
    }

    /// Returns true if this rule matches a process name (case-insensitive on Windows)
    pub fn matches_process(&self, query_proc: &str) -> bool {
        self.process_name.eq_ignore_ascii_case(query_proc)
    }
}