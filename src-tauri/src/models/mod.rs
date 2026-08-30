pub mod app_rule;
pub mod health;
pub mod settings;
pub mod singbox;
pub mod state;

pub use app_rule::{ApplicationRule, RouteDestination, RulePriority, RuleSource};
pub use health::{CloudflareTrace, HealthStatus, ServiceHealth};
pub use settings::{AetherLaunchOptions, AppSettings, QuickReconnectOption};
pub use singbox::SingBoxConfig;
pub use state::ConnectionState;
