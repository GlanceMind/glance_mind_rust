//! OpenMontage Redis Client
//!
//! Queue client for dispatching work to the OpenMontage worker.
//! Mirrors the pattern from novel_worker_dispatcher.rs.

use crate::dto::openmontage_dto::{PipelinesDto, PreflightDto};
use redis::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

pub const OPENMONTAGE_QUEUE_KEY: &str = "openmontage_worker_tasks";

/// Worker envelope JSON shape (matches the Python worker's envelope.py)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerEnvelope {
    pub task_id: String,
    pub job_id: String,
    pub project_id: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub kind: String,            // "run" | "resume"
    pub request_json: JsonValue, // the OpenMontageProfessionalVideoRequest as JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_from_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_decision_json: Option<JsonValue>,
    pub start_sequence: i64,
    pub enqueued_at: String, // ISO 8601
}

/// OpenMontage client trait
pub trait OpenMontageClient: Send + Sync {
    fn enqueue_run(&self, envelope: WorkerEnvelope) -> Result<(), String>;
    fn enqueue_resume(&self, envelope: WorkerEnvelope) -> Result<(), String>;
    fn set_cancel_flag(&self, job_id: &str) -> Result<(), String>;
    fn read_preflight(&self) -> Result<Option<PreflightDto>, String>;
    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String>;
}

// ============================================================================
// Redis Implementation
// ============================================================================

#[derive(Clone)]
pub struct RedisOpenMontageClient {
    client: Client,
}

impl RedisOpenMontageClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self, String> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://host.docker.internal:6379".into());
        let client = redis::Client::open(redis_url.as_str())
            .map_err(|e| format!("Redis client create: {}", e))?;
        Ok(Self::new(client))
    }

    async fn enqueue_internal(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        let client = self.client.clone();
        let payload =
            serde_json::to_string(&envelope).map_err(|e| format!("serialize envelope: {}", e))?;

        tokio::task::spawn_blocking(move || {
            let mut conn = client
                .get_connection()
                .map_err(|e| format!("Redis connection error: {}", e))?;
            let _: i64 = redis::cmd("RPUSH")
                .arg(OPENMONTAGE_QUEUE_KEY)
                .arg(payload)
                .query(&mut conn)
                .map_err(|e| format!("Redis RPUSH error: {}", e))?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| format!("enqueue join error: {}", e))??;

        Ok(())
    }
}

impl OpenMontageClient for RedisOpenMontageClient {
    fn enqueue_run(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.enqueue_internal(envelope))
    }

    fn enqueue_resume(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {}", e))?;
        rt.block_on(self.enqueue_internal(envelope))
    }

    fn set_cancel_flag(&self, job_id: &str) -> Result<(), String> {
        let key = format!("openmontage:job:{}:cancel", job_id);
        let client = self.client.clone();
        let key_owned = key.clone();

        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {}", e))?;

        rt.block_on(tokio::task::spawn_blocking(move || {
            let mut conn = client
                .get_connection()
                .map_err(|e| format!("Redis connection error: {}", e))?;
            let _: () = redis::cmd("SET")
                .arg(&key_owned)
                .arg("1")
                .query(&mut conn)
                .map_err(|e| format!("Redis SET error: {}", e))?;
            Ok::<(), String>(())
        }))
        .map_err(|e| format!("set_cancel_flag join error: {}", e))??;

        Ok(())
    }

    fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
        let key = "openmontage:preflight";
        let client = self.client.clone();

        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {}", e))?;

        let value: Option<String> = rt
            .block_on(tokio::task::spawn_blocking(move || {
                let mut conn = client
                    .get_connection()
                    .map_err(|e| format!("Redis connection error: {}", e))?;
                redis::cmd("GET")
                    .arg(key)
                    .query(&mut conn)
                    .map_err(|e| format!("Redis GET error: {}", e))
            }))
            .map_err(|e| format!("read_preflight join error: {}", e))??;

        if let Some(json_str) = value {
            let dto = serde_json::from_str(&json_str)
                .map_err(|e| format!("parse preflight JSON: {}", e))?;
            Ok(Some(dto))
        } else {
            Ok(None)
        }
    }

    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
        let key = "openmontage:pipelines";
        let client = self.client.clone();

        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("tokio runtime: {}", e))?;

        let value: Option<String> = rt
            .block_on(tokio::task::spawn_blocking(move || {
                let mut conn = client
                    .get_connection()
                    .map_err(|e| format!("Redis connection error: {}", e))?;
                redis::cmd("GET")
                    .arg(key)
                    .query(&mut conn)
                    .map_err(|e| format!("Redis GET error: {}", e))
            }))
            .map_err(|e| format!("read_pipelines join error: {}", e))??;

        if let Some(json_str) = value {
            let dto = serde_json::from_str(&json_str)
                .map_err(|e| format!("parse pipelines JSON: {}", e))?;
            Ok(Some(dto))
        } else {
            Ok(None)
        }
    }
}

// ============================================================================
// Mock Implementation (for tests)
// ============================================================================

#[derive(Clone)]
pub struct MockOpenMontageClient {
    enqueued: Arc<Mutex<Vec<WorkerEnvelope>>>,
}

impl MockOpenMontageClient {
    pub fn new() -> Self {
        Self {
            enqueued: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_enqueued(&self) -> Vec<WorkerEnvelope> {
        self.enqueued.lock().unwrap().clone()
    }
}

impl Default for MockOpenMontageClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenMontageClient for MockOpenMontageClient {
    fn enqueue_run(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        self.enqueued.lock().unwrap().push(envelope);
        Ok(())
    }

    fn enqueue_resume(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        self.enqueued.lock().unwrap().push(envelope);
        Ok(())
    }

    fn set_cancel_flag(&self, _job_id: &str) -> Result<(), String> {
        Ok(())
    }

    fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
        Ok(None)
    }

    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
        Ok(None)
    }
}
