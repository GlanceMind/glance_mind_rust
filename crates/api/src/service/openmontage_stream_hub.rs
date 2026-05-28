//! OpenMontage SSE Stream Hub
//!
//! Per-job broadcast hub for Server-Sent Events.
//! Cloned from drama_stream_hub.rs

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct OpenMontageSseEvent {
    pub event_type: String,
    pub job_id: String,
    pub project_id: String,
    pub sequence: i64,
    pub status: String,
    pub stage: Option<String>,
    pub progress_pct: i32,
    pub payload: Value,
}

type Sender = mpsc::Sender<OpenMontageSseEvent>;

#[derive(Clone)]
pub struct OpenMontageStreamHub {
    subscribers: Arc<RwLock<HashMap<String, Vec<Sender>>>>,
}

impl OpenMontageStreamHub {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, job_id: &str) -> mpsc::Receiver<OpenMontageSseEvent> {
        let (tx, rx) = mpsc::channel(64);
        let mut subs = self.subscribers.write().await;
        subs.entry(job_id.to_string()).or_default().push(tx);
        rx
    }

    pub async fn publish(&self, event: OpenMontageSseEvent) {
        let job_id = event.job_id.clone();
        let mut subs = self.subscribers.write().await;
        if let Some(senders) = subs.get_mut(&job_id) {
            senders.retain(|tx| !tx.is_closed());
            for tx in senders.iter() {
                let _ = tx.try_send(event.clone());
            }
            if senders.is_empty() {
                subs.remove(&job_id);
            }
        }
    }
}

impl Default for OpenMontageStreamHub {
    fn default() -> Self {
        Self::new()
    }
}
