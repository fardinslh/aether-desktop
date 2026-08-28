use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

#[derive(Clone)]
pub struct RingBufferLogger {
    max_entries: usize,
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
}

impl RingBufferLogger {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
        }
    }

    pub fn log(
        &self,
        level: impl Into<String>,
        source: impl Into<String>,
        message: impl Into<String>,
    ) {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            level: level.into().to_uppercase(),
            source: source.into(),
            message: message.into(),
        };

        let mut lock = self.entries.write();
        if lock.len() >= self.max_entries {
            lock.pop_front();
        }
        lock.push_back(entry);
    }

    pub fn get_entries(&self) -> Vec<LogEntry> {
        self.entries.read().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }

    pub fn export_as_string(&self) -> String {
        let entries = self.get_entries();
        let mut out = String::new();
        for e in entries {
            out.push_str(&format!(
                "[{}] [{}] [{}] {}\n",
                e.timestamp, e.level, e.source, e.message
            ));
        }
        out
    }
}
