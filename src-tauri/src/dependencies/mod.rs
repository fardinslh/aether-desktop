pub mod github;
pub mod manager;
pub mod bundled;

pub use manager::{DependencyManager, DependencyStatus, DependencyUpdateInfo, DownloadProgress};
