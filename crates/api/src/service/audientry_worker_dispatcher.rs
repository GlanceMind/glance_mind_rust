use crate::dto::audientry_dto::AudientryWorkerTaskEnvelope;
use redis::Client;

/// Dispatches audientry worker tasks onto a Redis list via RPUSH.
///
/// Mirrors `DramaWorkerDispatcher`: the synchronous `get_connection()` + RPUSH
/// runs inside `tokio::task::spawn_blocking` so the async caller is never blocked.
/// Injected at the ai_chat handler via axum `Extension` (the Redis-less
/// `AiChatService` never holds it).
#[derive(Clone)]
pub struct AudientryWorkerDispatcher {
    client: Client,
    queue_key: String,
}

impl AudientryWorkerDispatcher {
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
        Ok(Self::new(client, AudientryWorkerTaskEnvelope::QUEUE_KEY))
    }

    /// Returns the Redis client so callers (the handler) can reuse it for the
    /// pub/sub relay without opening a second connection pool.
    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub async fn enqueue(&self, envelope: &AudientryWorkerTaskEnvelope) -> Result<(), String> {
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
    use crate::dto::audientry_dto::AudientryWorkerTaskEnvelope;
    use redis::Client;

    #[test]
    fn dispatcher_uses_expected_queue_key() {
        let client = Client::open("redis://127.0.0.1:6379").expect("redis client");
        let d = AudientryWorkerDispatcher::new(client, AudientryWorkerTaskEnvelope::QUEUE_KEY);
        assert_eq!(d.queue_key, "audientry_worker_tasks");
    }

    #[tokio::test]
    #[ignore] // opt-in: RUN_REDIS_INTEGRATION_TESTS=1 cargo test -p glance_mind_api ... -- --include-ignored
    async fn enqueue_round_trip_if_redis_available() {
        if std::env::var("RUN_REDIS_INTEGRATION_TESTS").as_deref() != Ok("1") {
            return;
        }
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
            .arg(AudientryWorkerTaskEnvelope::QUEUE_KEY)
            .query(&mut conn);

        let d =
            AudientryWorkerDispatcher::new(client.clone(), AudientryWorkerTaskEnvelope::QUEUE_KEY);
        let env = AudientryWorkerTaskEnvelope {
            contract_version: "2026-04-29".into(),
            job_id: "aud_rt".into(),
            conversation_id: 1,
            user_id: 2,
            product: crate::dto::audientry_dto::ProductBrief {
                contract_version: "2026-04-29".into(),
                name: "Tea".into(),
                description: "sleep tea".into(),
                landing_page_url: None,
                locale: "en-US".into(),
            },
            questionnaire_answers: None,
        };
        d.enqueue(&env).await.unwrap();

        let popped: Option<(String, String)> = redis::cmd("BLPOP")
            .arg(AudientryWorkerTaskEnvelope::QUEUE_KEY)
            .arg(1)
            .query(&mut conn)
            .unwrap();
        assert!(popped.is_some());
    }
}
