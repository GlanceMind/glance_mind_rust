use crate::dto::novel_dto::NovelWorkerTaskEnvelope;
use redis::Client;

#[derive(Clone)]
pub struct NovelWorkerDispatcher {
    client: Client,
    queue_key: String,
}

impl NovelWorkerDispatcher {
    pub fn new(client: Client, queue_key: impl Into<String>) -> Self {
        Self {
            client,
            queue_key: queue_key.into(),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://host.docker.internal:6379".into());
        let client =
            redis::Client::open(redis_url.as_str()).map_err(|e| format!("Redis client create: {e}"))?;
        Ok(Self::new(client, NovelWorkerTaskEnvelope::QUEUE_KEY))
    }

    pub async fn enqueue(&self, envelope: &NovelWorkerTaskEnvelope) -> Result<(), String> {
        let client = self.client.clone();
        let queue_key = self.queue_key.clone();
        let payload =
            serde_json::to_string(envelope).map_err(|e| format!("serialize worker task: {e}"))?;

        tokio::task::spawn_blocking(move || {
            let mut conn = client
                .get_connection()
                .map_err(|e| format!("Redis connection error: {e}"))?;
            let _: i64 = redis::cmd("RPUSH")
                .arg(&queue_key)
                .arg(payload)
                .query(&mut conn)
                .map_err(|e| format!("Redis RPUSH error: {e}"))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("worker dispatch join error: {e}"))??;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::novel_dto::NovelWorkerTaskType;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn dispatcher_uses_expected_queue_key() {
        let client = Client::open("redis://127.0.0.1:6379").expect("redis client");
        let dispatcher = NovelWorkerDispatcher::new(client, NovelWorkerTaskEnvelope::QUEUE_KEY);
        assert_eq!(dispatcher.queue_key, "novel_worker_tasks");
    }

    #[tokio::test]
    async fn enqueue_round_trip_if_redis_available() {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        let client = match Client::open(redis_url.as_str()) {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut conn = match client.get_connection() {
            Ok(c) => c,
            Err(_) => return,
        };

        let _: Result<(), _> = redis::cmd("DEL")
            .arg(NovelWorkerTaskEnvelope::QUEUE_KEY)
            .query(&mut conn);

        let dispatcher =
            NovelWorkerDispatcher::new(client.clone(), NovelWorkerTaskEnvelope::QUEUE_KEY);

        let envelope = NovelWorkerTaskEnvelope {
            task_id: "550e8400-e29b-41d4-a716-446655440001".to_string(),
            project_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            job_id: 123,
            stage_code: "n01_architecture".to_string(),
            task_type: NovelWorkerTaskType::GenerateArchitecture,
            interaction_version: 1_i64,
            chapter_number: None,
            payload: json!({
                "config_snapshot_id": 10
            }),
            created_at: Utc::now(),
        };

        dispatcher.enqueue(&envelope).await.expect("enqueue worker task");

        let values: Vec<String> = redis::cmd("LRANGE")
            .arg(NovelWorkerTaskEnvelope::QUEUE_KEY)
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .expect("read queued values");
        assert_eq!(values.len(), 1);
        assert!(values[0].contains("\"task_type\":\"generate_architecture\""));
        assert!(values[0].contains("\"project_id\":\"550e8400-e29b-41d4-a716-446655440000\""));
    }
}
