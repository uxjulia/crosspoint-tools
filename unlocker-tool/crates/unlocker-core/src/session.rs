use crate::types::LogEntry;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub struct SessionLog {
    buffer: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
    tx: broadcast::Sender<LogEntry>,
}

impl SessionLog {
    pub fn new(capacity: usize) -> Arc<Self> {
        let (tx, _rx) = broadcast::channel(256);
        Arc::new(Self {
            buffer: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
            tx,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.tx.subscribe()
    }

    pub async fn push(
        &self,
        level: &str,
        message: impl Into<String>,
        data: Option<serde_json::Value>,
    ) {
        let entry = LogEntry {
            ts: chrono::Utc::now().to_rfc3339(),
            level: level.to_string(),
            message: message.into(),
            data,
        };
        let mut buf = self.buffer.lock().await;
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(entry.clone());
        let _ = self.tx.send(entry);
    }

    pub async fn snapshot(&self) -> Vec<LogEntry> {
        self.buffer.lock().await.iter().cloned().collect()
    }
}
