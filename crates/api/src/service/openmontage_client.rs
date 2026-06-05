//! OpenMontage Redis Client
//!
//! Queue client for dispatching work to the OpenMontage worker.
//! Mirrors the pattern from novel_worker_dispatcher.rs.

use crate::dto::openmontage_dto::{
    CompositionRuntimes, PipelineInfoDto, PipelinesDto, PreflightDto, SetupOffer,
};
use redis::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const OPENMONTAGE_QUEUE_KEY: &str = "openmontage_worker_tasks";

// ============================================================================
// Worker snapshot -> frontend DTO transforms
//
// The OpenMontage Python worker publishes its preflight/pipelines snapshots to
// redis in a protobuf-JSON shape (see `lib/protocol_export.py` +
// `tools/tool_registry.py::provider_menu_summary`) that does NOT match the
// frontend contract (`packages/shared/src/api/openmontageTypes.ts`). The rust
// facade used to `serde_json::from_str::<DTO>` the worker JSON directly, which
// failed to parse and returned HTTP 500.
//
// These transforms parse the worker JSON as an untyped `serde_json::Value` and
// project it into the frontend DTOs. EVERY extraction is tolerant: a missing or
// wrong-typed field degrades to a sensible default, so the endpoints never 500
// again even if the worker shape drifts.
// ============================================================================

/// Read a boolean by JSON path, defaulting to `false` on any miss/mismatch.
fn json_bool(v: &JsonValue, default: bool) -> bool {
    v.as_bool().unwrap_or(default)
}

/// Read a string field, defaulting to `""` on any miss/mismatch.
fn json_str(v: &JsonValue) -> String {
    v.as_str().unwrap_or_default().to_string()
}

/// Read a `Vec<String>` from a JSON array, skipping non-string items.
fn json_str_vec(v: &JsonValue) -> Vec<String> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Transform the worker `openmontage:preflight` snapshot into the frontend
/// `PreflightDto`. Tolerant of missing/drifted fields.
pub fn transform_preflight(worker: &JsonValue) -> PreflightDto {
    // composition_runtimes: worker ARRAY [{name, available}] -> object booleans.
    let mut composition_runtimes = CompositionRuntimes::default();
    if let Some(arr) = worker
        .get("composition_runtimes")
        .and_then(|v| v.as_array())
    {
        for entry in arr {
            let name = entry.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let available = json_bool(entry.get("available").unwrap_or(&JsonValue::Null), false);
            match name {
                "ffmpeg" => composition_runtimes.ffmpeg = available,
                "remotion" => composition_runtimes.remotion = available,
                "hyperframes" => composition_runtimes.hyperframes = available,
                _ => {}
            }
        }
    }

    // available_pipelines: names from the worker preflight `pipelines` field
    // (each item is a pipeline manifest with a `name`), else [].
    let available_pipelines = worker
        .get("pipelines")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    p.get("name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    // tool_availability: best-effort map of tool name -> (status == "available")
    // from the worker `tools` array. Desktop doesn't read this, so an empty map
    // is acceptable; we populate it when the data is trivially mappable.
    let mut tool_availability: HashMap<String, bool> = HashMap::new();
    if let Some(tools) = worker.get("tools").and_then(|v| v.as_array()) {
        for tool in tools {
            if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                if name.is_empty() {
                    continue;
                }
                let available = tool
                    .get("status")
                    .and_then(|s| s.as_str())
                    .map(|s| s == "available")
                    .unwrap_or(false);
                tool_availability.insert(name.to_string(), available);
            }
        }
    }

    // setup_offers: worker {provider, install_instructions, tool, capability}
    // -> frontend {provider, instructions}. Fall back to tool/capability for the
    // provider label when `provider` is absent.
    let setup_offers = worker
        .get("setup_offers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|offer| {
                    let provider = offer
                        .get("provider")
                        .and_then(|p| p.as_str())
                        .filter(|s| !s.is_empty())
                        .or_else(|| offer.get("tool").and_then(|t| t.as_str()))
                        .or_else(|| offer.get("capability").and_then(|c| c.as_str()))
                        .unwrap_or_default()
                        .to_string();
                    let instructions = offer
                        .get("install_instructions")
                        .and_then(|i| i.as_str())
                        .or_else(|| offer.get("instructions").and_then(|i| i.as_str()))
                        .unwrap_or_default()
                        .to_string();
                    SetupOffer {
                        provider,
                        instructions,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    PreflightDto {
        composition_runtimes,
        available_pipelines,
        tool_availability,
        setup_offers,
    }
}

/// Transform the worker `openmontage:pipelines` snapshot (a BARE ARRAY of
/// pipeline manifests) into the frontend `PipelinesDto`. Tolerant of a
/// non-array payload (degrades to an empty list).
pub fn transform_pipelines(worker: &JsonValue) -> PipelinesDto {
    let pipelines = worker
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    // Skip entries with no usable name (id == name is the
                    // frontend key; a nameless option is useless).
                    let name = item.get("name").and_then(|n| n.as_str())?;
                    if name.is_empty() {
                        return None;
                    }
                    Some(PipelineInfoDto {
                        id: name.to_string(),
                        name: name.to_string(),
                        description: json_str(item.get("description").unwrap_or(&JsonValue::Null)),
                        // best_for: the worker manifest has no `best_for`; use
                        // `category` as the closest human-facing grouping.
                        best_for: json_str(item.get("category").unwrap_or(&JsonValue::Null)),
                        stability: json_str(item.get("stability").unwrap_or(&JsonValue::Null)),
                        // required_tools lives per-stage in the manifest, not at
                        // the pipeline level; surface it only if a top-level
                        // field happens to exist, else [].
                        required_tools: json_str_vec(
                            item.get("required_tools").unwrap_or(&JsonValue::Null),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    PipelinesDto { pipelines }
}

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
                // Parse as untyped Value, then TRANSFORM into the frontend DTO.
                // The worker's snapshot shape does NOT match PreflightDto, so a
                // direct `from_str::<PreflightDto>` would fail and 500. A bad
                // payload degrades to an all-default preflight rather than
                // erroring, so the endpoint never 500s on worker drift.
                let value: JsonValue = serde_json::from_str(&json_str).unwrap_or(JsonValue::Null);
                Ok(Some(transform_preflight(&value)))
            }
            None => Ok(None),
        }
    }

    fn read_pipelines(&self) -> Result<Option<PipelinesDto>, String> {
        match self.get_string("openmontage:pipelines")? {
            Some(json_str) => {
                // The worker publishes a BARE ARRAY here; parse as untyped Value
                // and transform. A non-array / unparseable payload degrades to an
                // empty pipeline list rather than erroring.
                let value: JsonValue = serde_json::from_str(&json_str).unwrap_or(JsonValue::Null);
                Ok(Some(transform_pipelines(&value)))
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
