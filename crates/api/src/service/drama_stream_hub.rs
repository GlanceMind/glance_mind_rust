use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct DramaSseEvent {
    pub event_type: String,
    pub project_id: String,
    pub run_id: String,
    pub sequence: i64,
    pub interaction_version: i32,
    pub status: String,
    pub current_stage: Option<String>,
    pub payload: Value,
}

type Sender = mpsc::Sender<DramaSseEvent>;

#[derive(Clone)]
pub struct DramaStreamHub {
    subscribers: Arc<RwLock<HashMap<String, Vec<Sender>>>>,
}

impl DramaStreamHub {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn subscribe(&self, project_id: &str) -> mpsc::Receiver<DramaSseEvent> {
        let (tx, rx) = mpsc::channel(64);
        let mut subs = self.subscribers.write().await;
        subs.entry(project_id.to_string()).or_default().push(tx);
        rx
    }

    pub async fn publish(&self, event: DramaSseEvent) {
        let project_id = event.project_id.clone();
        let mut subs = self.subscribers.write().await;
        if let Some(senders) = subs.get_mut(&project_id) {
            senders.retain(|tx| !tx.is_closed());
            for tx in senders.iter() {
                let _ = tx.try_send(event.clone());
            }
            if senders.is_empty() {
                subs.remove(&project_id);
            }
        }
    }
}

impl Default for DramaStreamHub {
    fn default() -> Self {
        Self::new()
    }
}
