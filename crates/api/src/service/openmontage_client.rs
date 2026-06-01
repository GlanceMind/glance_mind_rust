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

    /// Synchronous RPUSH of an envelope onto the worker queue.
    ///
    /// IMPORTANT: this MUST stay synchronous (no nested `Runtime::new()` +
    /// `block_on`). These trait methods are invoked from inside async axum
    /// handlers, i.e. on a tokio worker thread; spinning up a new multi-thread
    /// runtime and calling `block_on` there panics at runtime with "Cannot
    /// start a runtime from within a runtime". The redis crate's blocking
    /// client works fine on a runtime worker thread (same pattern used by
    /// `lib.rs::init_redis`).
    fn rpush_envelope(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        let job_id = envelope.job_id.clone();
        let task_id = envelope.task_id.clone();
        let payload =
            serde_json::to_string(&envelope).map_err(|e| format!("serialize envelope: {}", e))?;
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {}", e))?;
        let queue_len: i64 = redis::cmd("RPUSH")
            .arg(OPENMONTAGE_QUEUE_KEY)
            .arg(payload)
            .query(&mut conn)
            .map_err(|e| format!("Redis RPUSH error: {}", e))?;
        tracing::info!(
            %job_id,
            %task_id,
            queue = OPENMONTAGE_QUEUE_KEY,
            queue_len,
            "openmontage enqueue: RPUSH ok"
        );
        Ok(())
    }

    /// Synchronous GET of a JSON string value (used by preflight/pipelines).
    fn get_string(&self, key: &str) -> Result<Option<String>, String> {
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {}", e))?;
        redis::cmd("GET")
            .arg(key)
            .query(&mut conn)
            .map_err(|e| format!("Redis GET error: {}", e))
    }
}

impl OpenMontageClient for RedisOpenMontageClient {
    fn enqueue_run(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        self.rpush_envelope(envelope)
    }

    fn enqueue_resume(&self, envelope: WorkerEnvelope) -> Result<(), String> {
        self.rpush_envelope(envelope)
    }

    fn set_cancel_flag(&self, job_id: &str) -> Result<(), String> {
        let key = format!("openmontage:job:{}:cancel", job_id);
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| format!("Redis connection error: {}", e))?;
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg("1")
            .query(&mut conn)
            .map_err(|e| format!("Redis SET error: {}", e))?;
        Ok(())
    }

    fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
        match self.get_string("openmontage:preflight")? {
            Some(json_str) => {
                let dto = serde_json::from_str(&json_str)
                    .map_err(|e| format!("parse preflight JSON: {}", e))?;
                Ok(Some(dto))
            }
            None => Ok(None),
        }
    }

    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
        match self.get_string("openmontage:pipelines")? {
            Some(json_str) => {
                let dto = serde_json::from_str(&json_str)
                    .map_err(|e| format!("parse pipelines JSON: {}", e))?;
                Ok(Some(dto))
            }
            None => Ok(None),
        }
    }
}

// ============================================================================
// Mock Implementation (for tests)
// ============================================================================

#[derive(Clone)]
pub struct MockOpenMontageClient {
    enqueued: Arc<Mutex<Vec<WorkerEnvelope>>>,
    cancel_flags: Arc<Mutex<Vec<String>>>,
}

impl MockOpenMontageClient {
    pub fn new() -> Self {
        Self {
            enqueued: Arc::new(Mutex::new(Vec::new())),
            cancel_flags: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_enqueued(&self) -> Vec<WorkerEnvelope> {
        self.enqueued.lock().unwrap().clone()
    }

    pub fn was_cancel_flag_set(&self, job_id: &str) -> bool {
        self.cancel_flags
            .lock()
            .unwrap()
            .contains(&job_id.to_string())
    }

    pub fn last_enqueued_run(&self) -> Option<WorkerEnvelope> {
        self.enqueued.lock().unwrap().last().cloned()
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

    fn set_cancel_flag(&self, job_id: &str) -> Result<(), String> {
        self.cancel_flags.lock().unwrap().push(job_id.to_string());
        Ok(())
    }

    fn read_preflight(&self) -> Result<Option<PreflightDto>, String> {
        Ok(None)
    }

    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
        Ok(None)
    }
}
