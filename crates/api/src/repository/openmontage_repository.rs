//! OpenMontage Job Store
//!
//! Persistent storage for OpenMontage jobs and events.
//! Provides both in-memory (for tests) and Postgres implementations.

use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// New job data for creation
#[derive(Debug, Clone)]
pub struct NewJob {
    pub job_id: String,
    pub project_id: String,
    pub user_id: i32,
    pub tenant_id: String,
    pub request_id: String,
    pub idempotency_key: String,
    pub pipeline: String,
    pub input_mode: Option<String>,
    pub status: String,
    pub snapshot_json: JsonValue,
}

/// New job event data
#[derive(Debug, Clone)]
pub struct NewJobEvent {
    pub job_id: String,
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub status: Option<String>,
    pub event_json: JsonValue,
}

/// Job record (returned from store)
#[derive(Debug, Clone)]
pub struct Job {
    pub id: i32,
    pub job_id: String,
    pub project_id: String,
    pub user_id: i32,
    pub tenant_id: String,
    pub request_id: String,
    pub idempotency_key: String,
    pub pipeline: String,
    pub input_mode: Option<String>,
    pub status: String,
    pub cancel_requested: bool,
    pub current_stage: Option<String>,
    pub progress_pct: i32,
    pub render_runtime: Option<String>,
    pub approval_policy: Option<String>,
    pub budget_limit_usd: Option<f64>,
    pub last_event_sequence: i64,
    pub next_event_sequence: i64,
    pub sync_required: bool,
    pub snapshot_json: JsonValue,
    pub error_json: Option<JsonValue>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Job event record
#[derive(Debug, Clone)]
pub struct JobEvent {
    pub id: i32,
    pub job_id: String,
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub status: Option<String>,
    pub stage: Option<String>,
    pub progress_pct: Option<i32>,
    pub event_json: JsonValue,
    pub emitted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of appending an event
#[derive(Debug, Clone)]
pub struct AppendResult {
    pub inserted: bool,
    pub gap: bool,
}

/// Job store trait
pub trait OpenMontageJobStore: Send + Sync {
    fn create_job(&self, job: NewJob) -> Result<Job, String>;
    fn get_job(&self, job_id: &str) -> Result<Option<Job>, String>;
    fn find_by_idempotency(&self, key: &str) -> Result<Option<Job>, String>;
    fn append_event(&self, event: NewJobEvent) -> Result<AppendResult, String>;
    fn list_events(&self, job_id: &str, after_sequence: i64, limit: i64) -> Result<Vec<JobEvent>, String>;
    fn update_from_event(&self, event: &NewJobEvent) -> Result<(), String>;
    fn set_status(&self, job_id: &str, status: &str) -> Result<(), String>;
    fn set_cancel_requested(&self, job_id: &str) -> Result<(), String>;
}

// ============================================================================
// In-Memory Implementation (for tests)
// ============================================================================

#[derive(Debug, Clone)]
struct InMemoryJob {
    job: Job,
    events: Vec<JobEvent>,
}

#[derive(Clone)]
pub struct InMemoryJobStore {
    data: Arc<Mutex<HashMap<String, InMemoryJob>>>,
    next_id: Arc<Mutex<i32>>,
}

impl InMemoryJobStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    fn allocate_id(&self) -> i32 {
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        id
    }
}

impl Default for InMemoryJobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenMontageJobStore for InMemoryJobStore {
    fn create_job(&self, new_job: NewJob) -> Result<Job, String> {
        let mut data = self.data.lock().unwrap();

        // Check idempotency
        for stored in data.values() {
            if stored.job.idempotency_key == new_job.idempotency_key {
                return Err("Duplicate idempotency_key".to_string());
            }
        }

        let job = Job {
            id: self.allocate_id(),
            job_id: new_job.job_id.clone(),
            project_id: new_job.project_id,
            user_id: new_job.user_id,
            tenant_id: new_job.tenant_id,
            request_id: new_job.request_id,
            idempotency_key: new_job.idempotency_key,
            pipeline: new_job.pipeline,
            input_mode: new_job.input_mode,
            status: new_job.status,
            cancel_requested: false,
            current_stage: None,
            progress_pct: 0,
            render_runtime: None,
            approval_policy: None,
            budget_limit_usd: None,
            last_event_sequence: 0,
            next_event_sequence: 1,
            sync_required: false,
            snapshot_json: new_job.snapshot_json,
            error_json: None,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        data.insert(new_job.job_id.clone(), InMemoryJob {
            job: job.clone(),
            events: Vec::new(),
        });

        Ok(job)
    }

    fn get_job(&self, job_id: &str) -> Result<Option<Job>, String> {
        let data = self.data.lock().unwrap();
        Ok(data.get(job_id).map(|j| j.job.clone()))
    }

    fn find_by_idempotency(&self, key: &str) -> Result<Option<Job>, String> {
        let data = self.data.lock().unwrap();
        for stored in data.values() {
            if stored.job.idempotency_key == key {
                return Ok(Some(stored.job.clone()));
            }
        }
        Ok(None)
    }

    fn append_event(&self, event: NewJobEvent) -> Result<AppendResult, String> {
        let mut data = self.data.lock().unwrap();
        let stored = data.get_mut(&event.job_id)
            .ok_or_else(|| format!("Job not found: {}", event.job_id))?;

        // Check for duplicate (event_id + sequence)
        for existing in &stored.events {
            if existing.event_id == event.event_id && existing.sequence == event.sequence {
                return Ok(AppendResult { inserted: false, gap: false });
            }
        }

        // Detect gap
        let expected_seq = stored.job.next_event_sequence;
        let gap = event.sequence > expected_seq;

        let job_event = JobEvent {
            id: self.allocate_id(),
            job_id: event.job_id.clone(),
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            status: event.status.clone(),
            stage: None,
            progress_pct: None,
            event_json: event.event_json.clone(),
            emitted_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
        };

        stored.events.push(job_event);
        stored.events.sort_by_key(|e| e.sequence);

        // Update job metadata
        stored.job.last_event_sequence = event.sequence;
        stored.job.next_event_sequence = event.sequence + 1;
        if gap {
            stored.job.sync_required = true;
        }
        if let Some(ref status) = event.status {
            stored.job.status = status.clone();
        }
        stored.job.updated_at = Some(chrono::Utc::now());

        Ok(AppendResult { inserted: true, gap })
    }

    fn list_events(&self, job_id: &str, after_sequence: i64, limit: i64) -> Result<Vec<JobEvent>, String> {
        let data = self.data.lock().unwrap();
        let stored = data.get(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;

        let events: Vec<JobEvent> = stored.events.iter()
            .filter(|e| e.sequence > after_sequence)
            .take(limit as usize)
            .cloned()
            .collect();

        Ok(events)
    }

    fn update_from_event(&self, event: &NewJobEvent) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let stored = data.get_mut(&event.job_id)
            .ok_or_else(|| format!("Job not found: {}", event.job_id))?;

        if let Some(ref status) = event.status {
            stored.job.status = status.clone();
        }

        // Extract stage/progress from event_json if present
        if let Some(stage) = event.event_json.get("stage").and_then(|v| v.as_str()) {
            stored.job.current_stage = Some(stage.to_string());
        }
        if let Some(progress) = event.event_json.get("progress").and_then(|v| v.as_i64()) {
            stored.job.progress_pct = progress as i32;
        }

        stored.job.updated_at = Some(chrono::Utc::now());
        Ok(())
    }

    fn set_status(&self, job_id: &str, status: &str) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let stored = data.get_mut(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;
        stored.job.status = status.to_string();
        stored.job.updated_at = Some(chrono::Utc::now());
        Ok(())
    }

    fn set_cancel_requested(&self, job_id: &str) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let stored = data.get_mut(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;
        stored.job.cancel_requested = true;
        stored.job.updated_at = Some(chrono::Utc::now());
        Ok(())
    }
}

// ============================================================================
// Postgres Implementation (stub for production — DB schema not yet migrated)
// ============================================================================

#[derive(Clone)]
pub struct PgJobStore {}

impl PgJobStore {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {}
    }
}

impl OpenMontageJobStore for PgJobStore {
    fn create_job(&self, _new_job: NewJob) -> Result<Job, String> {
        Err("PgJobStore not implemented - use InMemoryJobStore for tests".to_string())
    }

    fn get_job(&self, _job_id: &str) -> Result<Option<Job>, String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn find_by_idempotency(&self, _key: &str) -> Result<Option<Job>, String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn append_event(&self, _event: NewJobEvent) -> Result<AppendResult, String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn list_events(&self, _job_id: &str, _after_sequence: i64, _limit: i64) -> Result<Vec<JobEvent>, String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn update_from_event(&self, _event: &NewJobEvent) -> Result<(), String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn set_status(&self, _job_id: &str, _status: &str) -> Result<(), String> {
        Err("PgJobStore not implemented".to_string())
    }

    fn set_cancel_requested(&self, _job_id: &str) -> Result<(), String> {
        Err("PgJobStore not implemented".to_string())
    }
}
