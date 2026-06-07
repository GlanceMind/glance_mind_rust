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
    pub request_hash: String,
    pub pipeline: String,
    pub input_mode: Option<String>,
    pub status: String,
    pub snapshot_json: JsonValue,
    pub render_runtime: Option<String>,
    pub approval_policy: Option<String>,
    pub budget_limit_usd: Option<f64>,
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
    pub request_hash: String,
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

/// Asset record
#[derive(Debug, Clone)]
pub struct Asset {
    pub id: i32,
    pub asset_id: String,
    pub user_id: i32,
    pub kind: String,
    pub role: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
    pub duration_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// New asset data for creation
#[derive(Debug, Clone)]
pub struct NewAsset {
    pub asset_id: String,
    pub user_id: i32,
    pub kind: String,
    pub role: String,
    pub uri: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
    pub duration_ms: Option<i32>,
}

/// Result of create_job indicating whether a new job was created or an existing one was found
#[derive(Debug, Clone)]
pub struct CreateJobResult {
    pub job: Job,
    pub created: bool, // true if newly inserted, false if existing job returned
}

/// Derive the final job status from an event, applying downgrade rules.
///
/// If event.status is "completed" AND the event's artifacts contain NO primary_video,
/// returns "degraded". Otherwise returns event.status unchanged.
///
/// This is the single source of truth for the completed→degraded rule, applied
/// consistently in both InMemoryJobStore and PgJobStore.
fn derive_final_status(event: &NewJobEvent) -> Option<String> {
    let event_status = event.status.as_ref()?;

    // Only apply the rule if status is "completed"
    if event_status != "completed" {
        return Some(event_status.clone());
    }

    // Check for primary_video artifact in event_json
    if let Some(artifacts) = event.event_json.get("artifacts").and_then(|v| v.as_array()) {
        let has_primary_video = artifacts.iter().any(|a| {
            a.get("role")
                .and_then(|r| r.as_str())
                .map(|r| r == "primary_video")
                .unwrap_or(false)
        });

        if has_primary_video {
            return Some("completed".to_string());
        } else {
            // Completed without primary_video → degraded
            return Some("degraded".to_string());
        }
    }

    // No artifacts array → degraded
    Some("degraded".to_string())
}

/// Job store trait
pub trait OpenMontageJobStore: Send + Sync {
    fn create_job(&self, job: NewJob) -> Result<CreateJobResult, String>;
    fn get_job(&self, job_id: &str) -> Result<Option<Job>, String>;
    fn find_by_idempotency(&self, user_id: i32, key: &str) -> Result<Option<Job>, String>;
    fn append_event(&self, event: NewJobEvent) -> Result<AppendResult, String>;
    fn list_events(
        &self,
        job_id: &str,
        after_sequence: i64,
        limit: i64,
    ) -> Result<Vec<JobEvent>, String>;
    fn update_from_event(&self, event: &NewJobEvent) -> Result<(), String>;
    fn set_status(&self, job_id: &str, status: &str) -> Result<(), String>;
    fn set_cancel_requested(&self, job_id: &str) -> Result<(), String>;
    fn get_asset(&self, asset_id: &str) -> Result<Option<Asset>, String>;
    fn insert_asset(&self, asset: NewAsset) -> Result<Asset, String>;
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
    assets: Arc<Mutex<HashMap<String, Asset>>>,
    next_id: Arc<Mutex<i32>>,
}

impl InMemoryJobStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            assets: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    fn allocate_id(&self) -> i32 {
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        id
    }

    /// Test helper: list all jobs (for verifying enqueue count)
    pub fn list_jobs_for_test(&self) -> Vec<Job> {
        let data = self.data.lock().unwrap();
        data.values().map(|s| s.job.clone()).collect()
    }
}

impl Default for InMemoryJobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenMontageJobStore for InMemoryJobStore {
    fn create_job(&self, new_job: NewJob) -> Result<CreateJobResult, String> {
        let mut data = self.data.lock().unwrap();

        // Atomic find-or-insert: check for existing job with (user_id, idempotency_key)
        for stored in data.values() {
            if stored.job.user_id == new_job.user_id
                && stored.job.idempotency_key == new_job.idempotency_key
            {
                // Found existing job — return it with created=false
                return Ok(CreateJobResult {
                    job: stored.job.clone(),
                    created: false,
                });
            }
        }

        // No existing job — create new one
        let job = Job {
            id: self.allocate_id(),
            job_id: new_job.job_id.clone(),
            project_id: new_job.project_id,
            user_id: new_job.user_id,
            tenant_id: new_job.tenant_id,
            request_id: new_job.request_id,
            idempotency_key: new_job.idempotency_key,
            request_hash: new_job.request_hash,
            pipeline: new_job.pipeline,
            input_mode: new_job.input_mode,
            status: new_job.status,
            cancel_requested: false,
            current_stage: None,
            progress_pct: 0,
            render_runtime: new_job.render_runtime,
            approval_policy: new_job.approval_policy,
            budget_limit_usd: new_job.budget_limit_usd,
            last_event_sequence: 0,
            next_event_sequence: 1,
            sync_required: false,
            snapshot_json: new_job.snapshot_json,
            error_json: None,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        data.insert(
            new_job.job_id.clone(),
            InMemoryJob {
                job: job.clone(),
                events: Vec::new(),
            },
        );

        Ok(CreateJobResult { job, created: true })
    }

    fn get_job(&self, job_id: &str) -> Result<Option<Job>, String> {
        let data = self.data.lock().unwrap();
        Ok(data.get(job_id).map(|j| j.job.clone()))
    }

    fn find_by_idempotency(&self, user_id: i32, key: &str) -> Result<Option<Job>, String> {
        let data = self.data.lock().unwrap();
        for stored in data.values() {
            if stored.job.user_id == user_id && stored.job.idempotency_key == key {
                return Ok(Some(stored.job.clone()));
            }
        }
        Ok(None)
    }

    fn append_event(&self, event: NewJobEvent) -> Result<AppendResult, String> {
        let mut data = self.data.lock().unwrap();
        let stored = data
            .get_mut(&event.job_id)
            .ok_or_else(|| format!("Job not found: {}", event.job_id))?;

        // Check for duplicate (event_id + sequence)
        for existing in &stored.events {
            if existing.event_id == event.event_id && existing.sequence == event.sequence {
                return Ok(AppendResult {
                    inserted: false,
                    gap: false,
                });
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
        // Apply final status derivation (completed→degraded rule)
        if let Some(final_status) = derive_final_status(&event) {
            stored.job.status = final_status;
        }
        stored.job.updated_at = Some(chrono::Utc::now());

        Ok(AppendResult {
            inserted: true,
            gap,
        })
    }

    fn list_events(
        &self,
        job_id: &str,
        after_sequence: i64,
        limit: i64,
    ) -> Result<Vec<JobEvent>, String> {
        let data = self.data.lock().unwrap();
        let stored = data
            .get(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;

        let events: Vec<JobEvent> = stored
            .events
            .iter()
            .filter(|e| e.sequence > after_sequence)
            .take(limit as usize)
            .cloned()
            .collect();

        Ok(events)
    }

    fn update_from_event(&self, event: &NewJobEvent) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let stored = data
            .get_mut(&event.job_id)
            .ok_or_else(|| format!("Job not found: {}", event.job_id))?;

        // Apply final status derivation (completed→degraded rule)
        if let Some(final_status) = derive_final_status(event) {
            stored.job.status = final_status;
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
        let stored = data
            .get_mut(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;
        stored.job.status = status.to_string();
        stored.job.updated_at = Some(chrono::Utc::now());
        Ok(())
    }

    fn set_cancel_requested(&self, job_id: &str) -> Result<(), String> {
        let mut data = self.data.lock().unwrap();
        let stored = data
            .get_mut(job_id)
            .ok_or_else(|| format!("Job not found: {}", job_id))?;
        stored.job.cancel_requested = true;
        stored.job.updated_at = Some(chrono::Utc::now());
        Ok(())
    }

    fn get_asset(&self, asset_id: &str) -> Result<Option<Asset>, String> {
        let assets = self.assets.lock().unwrap();
        Ok(assets.get(asset_id).cloned())
    }

    fn insert_asset(&self, new_asset: NewAsset) -> Result<Asset, String> {
        let mut assets = self.assets.lock().unwrap();

        // Check if already exists
        if assets.contains_key(&new_asset.asset_id) {
            return Err(format!("Asset already exists: {}", new_asset.asset_id));
        }

        let asset = Asset {
            id: self.allocate_id(),
            asset_id: new_asset.asset_id.clone(),
            user_id: new_asset.user_id,
            kind: new_asset.kind,
            role: new_asset.role,
            uri: new_asset.uri,
            mime_type: new_asset.mime_type,
            bytes: new_asset.bytes,
            width_px: new_asset.width_px,
            height_px: new_asset.height_px,
            duration_ms: new_asset.duration_ms,
            created_at: chrono::Utc::now(),
        };

        assets.insert(new_asset.asset_id, asset.clone());
        Ok(asset)
    }
}

// ============================================================================
// Postgres Implementation
// ============================================================================

use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use glance_mind_db::entity::openmontage::{
    NewOpenmontageAsset, NewOpenmontageJob, NewOpenmontageJobEvent, OpenmontageAsset,
    OpenmontageJob, OpenmontageJobEvent, UpdateOpenmontageJob,
};

#[derive(Clone)]
pub struct PgJobStore {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl PgJobStore {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    fn get_conn(&self) -> Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>, String> {
        self.pool.get().map_err(|e| format!("Pool error: {}", e))
    }
}

impl OpenMontageJobStore for PgJobStore {
    fn create_job(&self, new_job: NewJob) -> Result<CreateJobResult, String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        let new_db_job = NewOpenmontageJob {
            job_id: new_job.job_id.clone(),
            project_id: new_job.project_id.clone(),
            user_id: new_job.user_id,
            tenant_id: new_job.tenant_id.clone(),
            request_id: new_job.request_id.clone(),
            idempotency_key: new_job.idempotency_key.clone(),
            request_hash: new_job.request_hash.clone(),
            pipeline: new_job.pipeline.clone(),
            input_mode: new_job.input_mode.clone(),
            status: new_job.status.clone(),
            snapshot_json: new_job.snapshot_json.clone(),
            render_runtime: new_job.render_runtime.clone(),
            approval_policy: new_job.approval_policy.clone(),
            budget_limit_usd: new_job
                .budget_limit_usd
                .and_then(|v| bigdecimal::BigDecimal::try_from(v).ok()),
        };

        // Atomic upsert with ON CONFLICT: try to insert, on conflict do nothing.
        // The idiomatic Diesel pattern: .optional() converts NotFound (no row on conflict) to Ok(None).
        let inserted: Option<OpenmontageJob> = diesel::insert_into(gm_openmontage_jobs)
            .values(&new_db_job)
            .on_conflict((user_id, idempotency_key))
            .do_nothing()
            .returning(OpenmontageJob::as_select())
            .get_result(&mut conn)
            .optional()
            .map_err(|e| format!("Insert error: {}", e))?;

        match inserted {
            Some(db_job) => {
                // Row was inserted — this is a new job
                Ok(CreateJobResult {
                    job: Job {
                        id: db_job.id,
                        job_id: db_job.job_id,
                        project_id: db_job.project_id,
                        user_id: db_job.user_id,
                        tenant_id: db_job.tenant_id,
                        request_id: db_job.request_id,
                        idempotency_key: db_job.idempotency_key,
                        request_hash: db_job.request_hash,
                        pipeline: db_job.pipeline,
                        input_mode: db_job.input_mode,
                        status: db_job.status,
                        cancel_requested: db_job.cancel_requested,
                        current_stage: db_job.current_stage,
                        progress_pct: db_job.progress_pct,
                        render_runtime: db_job.render_runtime,
                        approval_policy: db_job.approval_policy,
                        budget_limit_usd: db_job
                            .budget_limit_usd
                            .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0)),
                        last_event_sequence: db_job.last_event_sequence,
                        next_event_sequence: db_job.next_event_sequence,
                        sync_required: db_job.sync_required,
                        snapshot_json: db_job.snapshot_json,
                        error_json: db_job.error_json,
                        created_at: db_job.created_at,
                        updated_at: db_job.updated_at,
                    },
                    created: true,
                })
            }
            None => {
                // ON CONFLICT DO NOTHING returned no row — conflict occurred, fetch existing job
                let existing_job = self
                    .find_by_idempotency(new_job.user_id, &new_job.idempotency_key)?
                    .ok_or_else(|| {
                        "Conflict occurred but existing job not found (race condition)".to_string()
                    })?;

                Ok(CreateJobResult {
                    job: existing_job,
                    created: false,
                })
            }
        }
    }

    fn get_job(&self, job_id_param: &str) -> Result<Option<Job>, String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        let db_job: Option<OpenmontageJob> = gm_openmontage_jobs
            .filter(job_id.eq(job_id_param))
            .select(OpenmontageJob::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Query error: {}", e))?;

        Ok(db_job.map(|db_job| Job {
            id: db_job.id,
            job_id: db_job.job_id,
            project_id: db_job.project_id,
            user_id: db_job.user_id,
            tenant_id: db_job.tenant_id,
            request_id: db_job.request_id,
            idempotency_key: db_job.idempotency_key,
            request_hash: db_job.request_hash,
            pipeline: db_job.pipeline,
            input_mode: db_job.input_mode,
            status: db_job.status,
            cancel_requested: db_job.cancel_requested,
            current_stage: db_job.current_stage,
            progress_pct: db_job.progress_pct,
            render_runtime: db_job.render_runtime,
            approval_policy: db_job.approval_policy,
            budget_limit_usd: db_job
                .budget_limit_usd
                .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0)),
            last_event_sequence: db_job.last_event_sequence,
            next_event_sequence: db_job.next_event_sequence,
            sync_required: db_job.sync_required,
            snapshot_json: db_job.snapshot_json,
            error_json: db_job.error_json,
            created_at: db_job.created_at,
            updated_at: db_job.updated_at,
        }))
    }

    fn find_by_idempotency(&self, user_id_param: i32, key: &str) -> Result<Option<Job>, String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        let db_job: Option<OpenmontageJob> = gm_openmontage_jobs
            .filter(user_id.eq(user_id_param))
            .filter(idempotency_key.eq(key))
            .select(OpenmontageJob::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Query error: {}", e))?;

        Ok(db_job.map(|db_job| Job {
            id: db_job.id,
            job_id: db_job.job_id,
            project_id: db_job.project_id,
            user_id: db_job.user_id,
            tenant_id: db_job.tenant_id,
            request_id: db_job.request_id,
            idempotency_key: db_job.idempotency_key,
            request_hash: db_job.request_hash,
            pipeline: db_job.pipeline,
            input_mode: db_job.input_mode,
            status: db_job.status,
            cancel_requested: db_job.cancel_requested,
            current_stage: db_job.current_stage,
            progress_pct: db_job.progress_pct,
            render_runtime: db_job.render_runtime,
            approval_policy: db_job.approval_policy,
            budget_limit_usd: db_job
                .budget_limit_usd
                .map(|bd| bd.to_string().parse::<f64>().unwrap_or(0.0)),
            last_event_sequence: db_job.last_event_sequence,
            next_event_sequence: db_job.next_event_sequence,
            sync_required: db_job.sync_required,
            snapshot_json: db_job.snapshot_json,
            error_json: db_job.error_json,
            created_at: db_job.created_at,
            updated_at: db_job.updated_at,
        }))
    }

    fn append_event(&self, event: NewJobEvent) -> Result<AppendResult, String> {
        use glance_mind_db::schema::gm_openmontage_job_events::dsl::*;
        use glance_mind_db::schema::gm_openmontage_jobs;

        let mut conn = self.get_conn()?;

        // First, get the current job's next_event_sequence
        let current_job: OpenmontageJob = gm_openmontage_jobs::table
            .filter(gm_openmontage_jobs::job_id.eq(&event.job_id))
            .select(OpenmontageJob::as_select())
            .first(&mut conn)
            .map_err(|e| format!("Job not found: {}", e))?;

        let gap = event.sequence > current_job.next_event_sequence;

        // Try to insert event with ON CONFLICT DO NOTHING
        let new_db_event = NewOpenmontageJobEvent {
            job_id: event.job_id.clone(),
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            status: event.status.clone(),
            stage: None,
            progress_pct: None,
            event_json: event.event_json.clone(),
            emitted_at: Some(chrono::Utc::now()),
        };

        let insert_result = diesel::insert_into(gm_openmontage_job_events)
            .values(&new_db_event)
            .on_conflict((job_id, sequence))
            .do_nothing()
            .execute(&mut conn)
            .map_err(|e| format!("Insert event error: {}", e))?;

        let inserted = insert_result > 0;

        if !inserted {
            // Check if it was duplicate event_id at different sequence
            let existing_count: i64 = gm_openmontage_job_events
                .filter(event_id.eq(&event.event_id))
                .filter(job_id.eq(&event.job_id))
                .count()
                .get_result(&mut conn)
                .map_err(|e| format!("Check duplicate error: {}", e))?;

            if existing_count > 0 {
                // Duplicate event_id, return not inserted
                return Ok(AppendResult {
                    inserted: false,
                    gap: false,
                });
            }
        }

        // Update job metadata if inserted
        if inserted {
            let update = UpdateOpenmontageJob {
                last_event_sequence: Some(event.sequence),
                next_event_sequence: Some(event.sequence + 1),
                updated_at: Some(chrono::Utc::now()),
                sync_required: if gap { Some(true) } else { None },
                status: derive_final_status(&event),
                ..Default::default()
            };

            diesel::update(gm_openmontage_jobs::table)
                .filter(gm_openmontage_jobs::job_id.eq(&event.job_id))
                .set(&update)
                .execute(&mut conn)
                .map_err(|e| format!("Update job error: {}", e))?;
        }

        Ok(AppendResult { inserted, gap })
    }

    fn list_events(
        &self,
        job_id_param: &str,
        after_sequence: i64,
        limit: i64,
    ) -> Result<Vec<JobEvent>, String> {
        use glance_mind_db::schema::gm_openmontage_job_events::dsl::*;

        let mut conn = self.get_conn()?;

        let db_events: Vec<OpenmontageJobEvent> = gm_openmontage_job_events
            .filter(job_id.eq(job_id_param))
            .filter(sequence.gt(after_sequence))
            .order(sequence.asc())
            .limit(limit)
            .select(OpenmontageJobEvent::as_select())
            .load(&mut conn)
            .map_err(|e| format!("List events error: {}", e))?;

        Ok(db_events
            .into_iter()
            .map(|e| JobEvent {
                id: e.id,
                job_id: e.job_id,
                sequence: e.sequence,
                event_id: e.event_id,
                event_type: e.event_type,
                status: e.status,
                stage: e.stage,
                progress_pct: e.progress_pct,
                event_json: e.event_json,
                emitted_at: e.emitted_at,
                created_at: e.created_at,
            })
            .collect())
    }

    fn update_from_event(&self, event: &NewJobEvent) -> Result<(), String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        conn.transaction::<_, diesel::result::Error, _>(|conn| {
            // Read current job state
            let current_job: OpenmontageJob = gm_openmontage_jobs
                .filter(job_id.eq(&event.job_id))
                .select(OpenmontageJob::as_select())
                .first(conn)?;

            // Apply final status derivation (completed→degraded rule)
            let mut update = UpdateOpenmontageJob {
                status: derive_final_status(event),
                ..Default::default()
            };

            // Extract stage from event_json
            if let Some(stage_val) = event.event_json.get("stage").and_then(|v| v.as_str()) {
                update.current_stage = Some(stage_val.to_string());
            }

            // Extract progress_pct from event_json
            if let Some(progress_val) = event.event_json.get("progress").and_then(|v| v.as_i64()) {
                update.progress_pct = Some(progress_val as i32);
            }

            // Update snapshot_json with event data
            update.snapshot_json = Some(event.event_json.clone());

            // Detect gap and set sync_required
            let gap = event.sequence > current_job.next_event_sequence;
            if gap {
                update.sync_required = Some(true);
            }

            // Advance next_event_sequence only if contiguous
            if event.sequence == current_job.next_event_sequence {
                update.next_event_sequence = Some(event.sequence + 1);
            }

            // Always update last_event_sequence
            update.last_event_sequence = Some(event.sequence);
            update.updated_at = Some(chrono::Utc::now());

            diesel::update(gm_openmontage_jobs.filter(job_id.eq(&event.job_id)))
                .set(&update)
                .execute(conn)?;

            Ok(())
        })
        .map_err(|e| format!("Transaction error: {}", e))
    }

    fn set_status(&self, job_id_param: &str, status_val: &str) -> Result<(), String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        let update = UpdateOpenmontageJob {
            status: Some(status_val.to_string()),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        };

        diesel::update(gm_openmontage_jobs.filter(job_id.eq(job_id_param)))
            .set(&update)
            .execute(&mut conn)
            .map_err(|e| format!("Set status error: {}", e))?;

        Ok(())
    }

    fn set_cancel_requested(&self, job_id_param: &str) -> Result<(), String> {
        use glance_mind_db::schema::gm_openmontage_jobs::dsl::*;

        let mut conn = self.get_conn()?;

        let update = UpdateOpenmontageJob {
            cancel_requested: Some(true),
            updated_at: Some(chrono::Utc::now()),
            ..Default::default()
        };

        diesel::update(gm_openmontage_jobs.filter(job_id.eq(job_id_param)))
            .set(&update)
            .execute(&mut conn)
            .map_err(|e| format!("Set cancel_requested error: {}", e))?;

        Ok(())
    }

    fn get_asset(&self, asset_id_param: &str) -> Result<Option<Asset>, String> {
        use glance_mind_db::schema::gm_openmontage_assets::dsl::*;

        let mut conn = self.get_conn()?;

        let db_asset: Option<OpenmontageAsset> = gm_openmontage_assets
            .filter(asset_id.eq(asset_id_param))
            .select(OpenmontageAsset::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Get asset error: {}", e))?;

        Ok(db_asset.map(|a| Asset {
            id: a.id,
            asset_id: a.asset_id,
            user_id: a.user_id,
            kind: a.kind,
            role: a.role,
            uri: a.uri,
            mime_type: a.mime_type,
            bytes: a.bytes,
            width_px: a.width_px,
            height_px: a.height_px,
            duration_ms: a.duration_ms,
            created_at: a.created_at,
        }))
    }

    fn insert_asset(&self, new_asset: NewAsset) -> Result<Asset, String> {
        use glance_mind_db::schema::gm_openmontage_assets::dsl::*;

        let mut conn = self.get_conn()?;

        let new_db_asset = NewOpenmontageAsset {
            asset_id: new_asset.asset_id.clone(),
            user_id: new_asset.user_id,
            kind: new_asset.kind.clone(),
            role: new_asset.role.clone(),
            uri: new_asset.uri.clone(),
            mime_type: new_asset.mime_type.clone(),
            bytes: new_asset.bytes,
            width_px: new_asset.width_px,
            height_px: new_asset.height_px,
            duration_ms: new_asset.duration_ms,
        };

        let db_asset: OpenmontageAsset = diesel::insert_into(gm_openmontage_assets)
            .values(&new_db_asset)
            .returning(OpenmontageAsset::as_select())
            .get_result(&mut conn)
            .map_err(|e| format!("Insert asset error: {}", e))?;

        Ok(Asset {
            id: db_asset.id,
            asset_id: db_asset.asset_id,
            user_id: db_asset.user_id,
            kind: db_asset.kind,
            role: db_asset.role,
            uri: db_asset.uri,
            mime_type: db_asset.mime_type,
            bytes: db_asset.bytes,
            width_px: db_asset.width_px,
            height_px: db_asset.height_px,
            duration_ms: db_asset.duration_ms,
            created_at: db_asset.created_at,
        })
    }
}
