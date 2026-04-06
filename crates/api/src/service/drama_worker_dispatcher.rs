use crate::dto::drama_dto::DramaWorkerTaskEnvelope;
use redis::Client;

#[derive(Clone)]
pub struct DramaWorkerDispatcher {
    client: Client,
    queue_key: String,
}

impl DramaWorkerDispatcher {
    pub fn new(client: Client, queue_key: impl Into<String>) -> Self {
        Self {
            client,
            queue_key: queue_key.into(),
        }
    }

    pub fn from_env() -> Result<Self, String> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://host.docker.internal:6379".into());
        let client = redis::Client::open(redis_url.as_str())
            .map_err(|e| format!("Redis client create: {e}"))?;
        Ok(Self::new(client, DramaWorkerTaskEnvelope::QUEUE_KEY))
    }

    pub async fn enqueue(&self, envelope: &DramaWorkerTaskEnvelope) -> Result<(), String> {
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
    use crate::dto::drama_dto::{DramaWorkerTaskTarget, DramaWorkerTaskType};
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn dispatcher_uses_expected_queue_key() {
        let client = Client::open("redis://127.0.0.1:6379").expect("redis client");
        let dispatcher = DramaWorkerDispatcher::new(client, DramaWorkerTaskEnvelope::QUEUE_KEY);
        assert_eq!(dispatcher.queue_key, "drama_worker_tasks");
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
            .arg(DramaWorkerTaskEnvelope::QUEUE_KEY)
            .query(&mut conn);

        let dispatcher =
            DramaWorkerDispatcher::new(client.clone(), DramaWorkerTaskEnvelope::QUEUE_KEY);

        let envelope = DramaWorkerTaskEnvelope {
            task_id: "task_001".to_string(),
            project_id: "proj_001".to_string(),
            task_type: DramaWorkerTaskType::GenerateSceneImages,
            stage_code: Some("s04_execution".to_string()),
            job_id: Some(7),
            interaction_version: Some(3),
            target: DramaWorkerTaskTarget {
                episode_id: Some(11),
                scene_id: Some(13),
                storyboard_id: None,
                character_id: None,
                prop_id: None,
            },
            payload: json!({
                "provider_profile_ref": "jimeng-default"
            }),
            created_at: Utc::now(),
        };

        dispatcher
            .enqueue(&envelope)
            .await
            .expect("enqueue worker task");

        let values: Vec<String> = redis::cmd("LRANGE")
            .arg(DramaWorkerTaskEnvelope::QUEUE_KEY)
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .expect("read queued values");
        assert_eq!(values.len(), 1);
        assert!(values[0].contains("\"task_type\":\"generate_scene_images\""));
        assert!(values[0].contains("\"project_id\":\"proj_001\""));
    }
}
