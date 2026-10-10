pub mod detector;
pub mod icon;
pub mod job;
pub mod orchestrator;
pub mod picker;
pub mod runner;
pub mod adaptive;
#[cfg(target_os = "macos")]
pub mod macos_helper;

pub use detector::{ProcessDetector, RunningProcessInfo};
pub use icon::IconExtractor;
pub use orchestrator::ConnectionOrchestrator;
pub use picker::pick_windows_executable;
pub use runner::{AetherRunner, SingBoxRunner};
