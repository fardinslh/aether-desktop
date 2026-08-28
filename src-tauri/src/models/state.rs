use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConnectionState {
    Disconnected,
    StartingAether,
    ScanningAether,
    WaitingForAether,
    TestingAether,
    StartingRouter,
    TestingRouting,
    Connected,
    Reconnecting,
    Disconnecting,
    Error,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}
