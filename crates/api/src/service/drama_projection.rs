use crate::config::database::Database;
use crate::dto::drama_dto::DramaCallbackEvent;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Float4, Integer, Jsonb, Nullable, Text, Timestamptz};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct DramaProjectionService {
    db: Arc<Database>,
}

#[derive(Debug, QueryableByName)]
pub struct ProjectionRow {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub project_id: String,
    #[diesel(sql_type = Integer)]
    pub user_id: i32,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub description: String,
    #[diesel(sql_type = Text)]
    pub status: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub content_type: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub platform: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub current_stage: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub pending_stage: Option<String>,
    #[diesel(sql_type = Float4)]
    pub progress_percent: f32,
    #[diesel(sql_type = Nullable<Text>)]
    pub run_id: Option<String>,
    #[diesel(sql_type = Integer)]
    pub interaction_version: i32,
    #[diesel(sql_type = BigInt)]
    pub last_event_sequence: i64,
    #[diesel(sql_type = Nullable<Text>)]
    pub error_message: Option<String>,
    #[diesel(sql_type = BigInt)]
    pub cost_reserve_cents: i64,
    #[diesel(sql_type = BigInt)]
    pub cost_consumed_cents: i64,
    #[diesel(sql_type = Nullable<Jsonb>)]
    pub interaction_payload: Option<Value>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub completed_at: Option<NaiveDateTime>,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: NaiveDateTime,
    #[diesel(sql_type = Timestamptz)]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, QueryableByName)]
pub struct ArtifactRow {
    #[diesel(sql_type = BigInt)]
    pub id: i64,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Text)]
    pub public_url: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub format: Option<String>,
    #[diesel(sql_type = Nullable<BigInt>)]
    pub file_size: Option<i64>,
    #[diesel(sql_type = Nullable<Integer>)]
    pub duration_seconds: Option<i32>,
    #[diesel(sql_type = Nullable<Jsonb>)]
    pub metadata: Option<Value>,
}

#[derive(Debug, QueryableByName)]
pub struct CostLedgerRow {
    #[diesel(sql_type = Text)]
    pub cost_type: String,
    #[diesel(sql_type = BigInt)]
    pub amount_cents: i64,
    #[diesel(sql_type = Nullable<Text>)]
    pub provider: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub stage_code: Option<String>,
}

#[derive(Debug, QueryableByName)]
pub struct FallbackEventRow {
    #[diesel(sql_type = Nullable<Text>)]
    pub stage_code: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub reason_category: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub reason_detail: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub from_provider: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub to_provider: Option<String>,
    #[diesel(sql_type = Jsonb)]
    pub payload: Value,
    #[diesel(sql_type = Timestamptz)]
    pub occurred_at: NaiveDateTime,
}

impl DramaProjectionService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self { db: db.clone() }
    }

    pub fn ingest_event(&self, event: &DramaCallbackEvent) -> Result<bool, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let job_id = extract_job_id(event);

        let existing: Option<i64> = diesel::sql_query(
            "SELECT id FROM gm_drama_callback_events WHERE event_id = $1",
        )
        .bind::<Text, _>(&event.event_id)
        .get_result::<IdOnly>(conn)
        .optional()
        .map_err(|e| format!("dedup check: {}", e))?
        .map(|r| r.id);

        if existing.is_some() {
            return Ok(false);
        }

        diesel::sql_query(
            "INSERT INTO gm_drama_callback_events \
             (event_id, project_id, run_id, sequence, stage_code, event_type, payload, occurred_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind::<Text, _>(&event.event_id)
        .bind::<Text, _>(&event.project_id)
        .bind::<Text, _>(&event.run_id)
        .bind::<BigInt, _>(event.sequence)
        .bind::<Nullable<Text>, _>(&event.stage_code)
        .bind::<Text, _>(&event.event_type)
        .bind::<Jsonb, _>(&event.payload)
        .bind::<Timestamptz, _>(event.occurred_at.naive_utc())
        .execute(conn)
        .map_err(|e| format!("insert event: {}", e))?;

        diesel::sql_query(
            "INSERT INTO gm_drama.events \
             (project_id, job_id, run_id, sequence, stage_code, event_type, status, payload, occurred_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (project_id, sequence) DO NOTHING",
        )
        .bind::<Text, _>(&event.project_id)
        .bind::<Nullable<BigInt>, _>(job_id)
        .bind::<Nullable<Text>, _>(Some(&event.run_id))
        .bind::<BigInt, _>(event.sequence)
        .bind::<Nullable<Text>, _>(&event.stage_code)
        .bind::<Text, _>(&event.event_type)
        .bind::<Nullable<Text>, _>(
            event.payload
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        )
        .bind::<Jsonb, _>(&event.payload)
        .bind::<Timestamptz, _>(event.occurred_at.naive_utc())
        .execute(conn)
        .map_err(|e| format!("insert canonical event: {}", e))?;

        self.apply_event_to_projection(conn, event)?;

        Ok(true)
    }

    fn apply_event_to_projection(
        &self,
        conn: &mut diesel::PgConnection,
        event: &DramaCallbackEvent,
    ) -> Result<(), String> {
        if event.event_type != "run_cancelled" && project_is_cancelled(conn, &event.project_id)? {
            return Ok(());
        }
        match event.event_type.as_str() {
            "run_started" => {
                sync_job_status(conn, event, "processing")?;
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'running', run_id = $2, last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(&event.run_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply run_started: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'running', updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical run_started: {}", e))?;
            }
            "stage_entered" => {
                let stage = event
                    .stage_code
                    .as_deref()
                    .unwrap_or("unknown");
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET current_stage = $2, last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(stage)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply stage_entered: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET current_stage = $2, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(stage)
                .execute(conn)
                .map_err(|e| format!("apply canonical stage_entered: {}", e))?;
            }
            "clarification_required" => {
                sync_job_status(conn, event, "completed")?;
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'needs_clarification', pending_stage = $2, \
                         interaction_payload = $4, \
                         interaction_version = interaction_version + 1, \
                         last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Nullable<Text>, _>(&event.stage_code)
                .bind::<BigInt, _>(event.sequence)
                .bind::<Jsonb, _>(&event.payload)
                .execute(conn)
                .map_err(|e| format!("apply clarification_required: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'needs_clarification', pending_stage = $2, \
                         metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{interaction_payload}', $3, true), \
                         interaction_version = interaction_version + 1, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Nullable<Text>, _>(&event.stage_code)
                .bind::<Jsonb, _>(&event.payload)
                .execute(conn)
                .map_err(|e| format!("apply canonical clarification_required: {}", e))?;
            }
            "clarification_resolved" => {
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'running', pending_stage = NULL, interaction_payload = NULL, \
                         last_event_sequence = $2, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $2",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply clarification_resolved: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'running', pending_stage = NULL, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical clarification_resolved: {}", e))?;
            }
            "strategy_package_ready" | "script_package_ready" => {
                sync_job_status(conn, event, "completed")?;
                let pending = if event.event_type == "strategy_package_ready" {
                    "strategy"
                } else {
                    "script"
                };
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'needs_approval', pending_stage = $2, \
                         interaction_payload = $4, \
                         interaction_version = interaction_version + 1, \
                         last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(pending)
                .bind::<BigInt, _>(event.sequence)
                .bind::<Jsonb, _>(&event.payload)
                .execute(conn)
                .map_err(|e| format!("apply package_ready: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'needs_approval', pending_stage = $2, \
                         metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{interaction_payload}', $3, true), \
                         interaction_version = interaction_version + 1, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(pending)
                .bind::<Jsonb, _>(&event.payload)
                .execute(conn)
                .map_err(|e| format!("apply canonical package_ready: {}", e))?;
            }
            "render_progress_recorded" => {
                let pct = event
                    .payload
                    .get("progress_percent")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET progress_percent = $2, last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Float4, _>(pct)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply render_progress: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET progress_percent = $2, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Float4, _>(pct)
                .execute(conn)
                .map_err(|e| format!("apply canonical render_progress: {}", e))?;
            }
            "cost_recorded" => {
                let cents = event
                    .payload
                    .get("cents")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cost_type = event
                    .payload
                    .get("line_item_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                diesel::sql_query(
                    "INSERT INTO gm_drama_cost_events \
                     (project_id, run_id, cost_type, amount_cents, stage_code, event_id) \
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Nullable<Text>, _>(Some(&event.run_id))
                .bind::<Text, _>(cost_type)
                .bind::<BigInt, _>(cents)
                .bind::<Nullable<Text>, _>(&event.stage_code)
                .bind::<Nullable<Text>, _>(Some(&event.event_id))
                .execute(conn)
                .map_err(|e| format!("insert cost event: {}", e))?;

                let provider = event
                    .payload
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                diesel::sql_query(
                    "INSERT INTO gm_drama.cost_ledger \
                     (project_id, stage_code, cost_type, provider, amount_cents, payload) \
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Nullable<Text>, _>(&event.stage_code)
                .bind::<Text, _>(cost_type)
                .bind::<Nullable<Text>, _>(provider)
                .bind::<BigInt, _>(cents)
                .bind::<Jsonb, _>(&event.payload)
                .execute(conn)
                .map_err(|e| format!("insert canonical cost ledger: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET cost_consumed_cents = cost_consumed_cents + $2, \
                         last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(cents)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply cost_recorded: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET cost_consumed_cents = cost_consumed_cents + $2, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(cents)
                .execute(conn)
                .map_err(|e| format!("apply canonical cost_recorded: {}", e))?;
            }
            "fallback_recorded" => {
                let reason_category = event
                    .payload
                    .get("reason_category")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let reason_detail = event
                    .payload
                    .get("reason_detail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let from_provider = event
                    .payload
                    .get("from_provider")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let to_provider = event
                    .payload
                    .get("to_provider")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                diesel::sql_query(
                    "INSERT INTO gm_drama.fallback_events \
                     (project_id, stage_code, reason_category, reason_detail, from_provider, to_provider, payload, occurred_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Nullable<Text>, _>(&event.stage_code)
                .bind::<Nullable<Text>, _>(reason_category)
                .bind::<Nullable<Text>, _>(reason_detail)
                .bind::<Nullable<Text>, _>(from_provider)
                .bind::<Nullable<Text>, _>(to_provider)
                .bind::<Jsonb, _>(&event.payload)
                .bind::<Timestamptz, _>(event.occurred_at.naive_utc())
                .execute(conn)
                .map_err(|e| format!("insert canonical fallback event: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET last_event_sequence = $2, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $2",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply fallback_recorded: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical fallback_recorded: {}", e))?;
            }
            "artifact_uploaded" => {
                sync_job_status(conn, event, "completed")?;

                let artifact_name = event
                    .payload
                    .get("artifact_name")
                    .or_else(|| event.payload.get("artifact_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("artifact");
                let artifact_type = event
                    .payload
                    .get("artifact_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("video");
                let public_url = event
                    .payload
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let bucket = event
                    .payload
                    .get("bucket")
                    .and_then(|v| v.as_str())
                    .unwrap_or("huobao-drama");
                let object_key = event
                    .payload
                    .get("object_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let format = event
                    .payload
                    .get("format")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let file_size = event.payload.get("size_bytes").and_then(|v| v.as_i64());
                let duration_seconds = event
                    .payload
                    .get("duration_seconds")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);

                diesel::sql_query(
                    "INSERT INTO gm_drama.assets \
                     (project_id, name, type, bucket, object_key, public_url, format, file_size, duration_seconds, metadata, source_job_id, created_at, updated_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW()) \
                     ON CONFLICT (bucket, object_key) DO UPDATE SET \
                       public_url = EXCLUDED.public_url, \
                       format = EXCLUDED.format, \
                       file_size = EXCLUDED.file_size, \
                       duration_seconds = EXCLUDED.duration_seconds, \
                       metadata = EXCLUDED.metadata, \
                       updated_at = NOW()",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(artifact_name)
                .bind::<Text, _>(artifact_type)
                .bind::<Text, _>(bucket)
                .bind::<Text, _>(object_key)
                .bind::<Text, _>(public_url)
                .bind::<Nullable<Text>, _>(format)
                .bind::<Nullable<BigInt>, _>(file_size)
                .bind::<Nullable<Integer>, _>(duration_seconds)
                .bind::<Jsonb, _>(&event.payload)
                .bind::<Nullable<BigInt>, _>(extract_job_id(event))
                .execute(conn)
                .map_err(|e| format!("insert canonical artifact: {}", e))?;
            }
            "run_completed" => {
                sync_job_status(conn, event, "completed")?;
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'completed', progress_percent = 100, pending_stage = NULL, \
                         last_event_sequence = $2, completed_at = NOW(), updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $2",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply run_completed: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'completed', progress_percent = 100, pending_stage = NULL, \
                         completed_at = NOW(), updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical run_completed: {}", e))?;
            }
            "run_failed" => {
                sync_job_status(conn, event, "failed")?;
                let msg = event
                    .payload
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'failed', error_message = $2, pending_stage = NULL, \
                         last_event_sequence = $3, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $3",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(msg)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply run_failed: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'failed', error_message = $2, pending_stage = NULL, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<Text, _>(msg)
                .execute(conn)
                .map_err(|e| format!("apply canonical run_failed: {}", e))?;
            }
            "run_cancelled" => {
                sync_job_status(conn, event, "cancelled")?;
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET status = 'cancelled', pending_stage = NULL, \
                         last_event_sequence = $2, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $2",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply run_cancelled: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET status = 'cancelled', pending_stage = NULL, updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical run_cancelled: {}", e))?;
            }
            _ => {
                diesel::sql_query(
                    "UPDATE gm_drama_project_projections \
                     SET last_event_sequence = $2, updated_at = NOW() \
                     WHERE project_id = $1 AND last_event_sequence < $2",
                )
                .bind::<Text, _>(&event.project_id)
                .bind::<BigInt, _>(event.sequence)
                .execute(conn)
                .map_err(|e| format!("apply default: {}", e))?;

                diesel::sql_query(
                    "UPDATE gm_drama.projects \
                     SET updated_at = NOW() \
                     WHERE project_id = $1",
                )
                .bind::<Text, _>(&event.project_id)
                .execute(conn)
                .map_err(|e| format!("apply canonical default: {}", e))?;
            }
        }
        Ok(())
    }

    pub fn get_projection(&self, project_id: &str) -> Result<Option<ProjectionRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT id, project_id, user_id, title, description, status, content_type, platform, current_stage, pending_stage, \
             progress_percent, run_id, interaction_version, last_event_sequence, \
             error_message, cost_reserve_cents, cost_consumed_cents, interaction_payload, completed_at, created_at, updated_at \
             FROM gm_drama_project_projections WHERE project_id = $1",
        )
        .bind::<Text, _>(project_id)
        .get_result::<ProjectionRow>(conn)
        .optional()
        .map_err(|e| format!("get projection: {}", e))
    }

    pub fn list_projections(
        &self,
        user_id: i32,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ProjectionRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        let limit = if limit > 0 { limit } else { 20 };
        let status = status.map(|s| s.to_string());

        diesel::sql_query(
            "SELECT id, project_id, user_id, title, description, status, content_type, platform, current_stage, pending_stage, \
             progress_percent, run_id, interaction_version, last_event_sequence, \
             error_message, cost_reserve_cents, cost_consumed_cents, interaction_payload, completed_at, created_at, updated_at \
             FROM gm_drama_project_projections \
             WHERE user_id = $1 AND ($2::text IS NULL OR status = $2) \
             ORDER BY updated_at DESC \
             LIMIT $3",
        )
        .bind::<Integer, _>(user_id)
        .bind::<Nullable<Text>, _>(status)
        .bind::<BigInt, _>(limit)
        .load::<ProjectionRow>(conn)
        .map_err(|e| format!("list projections: {}", e))
    }

    pub fn list_artifacts(&self, project_id: &str) -> Result<Vec<ArtifactRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT id, name, public_url, format, file_size, duration_seconds, metadata \
             FROM gm_drama.assets \
             WHERE project_id = $1 AND deleted_at IS NULL \
             ORDER BY created_at ASC",
        )
        .bind::<Text, _>(project_id)
        .load::<ArtifactRow>(conn)
        .map_err(|e| format!("list artifacts: {}", e))
    }

    pub fn list_cost_ledger(&self, project_id: &str) -> Result<Vec<CostLedgerRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT cost_type, amount_cents, provider, stage_code \
             FROM gm_drama.cost_ledger \
             WHERE project_id = $1 \
             ORDER BY created_at ASC",
        )
        .bind::<Text, _>(project_id)
        .load::<CostLedgerRow>(conn)
        .map_err(|e| format!("list cost ledger: {}", e))
    }

    pub fn list_fallback_events(&self, project_id: &str) -> Result<Vec<FallbackEventRow>, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "SELECT stage_code, reason_category, reason_detail, from_provider, to_provider, payload, occurred_at \
             FROM gm_drama.fallback_events \
             WHERE project_id = $1 \
             ORDER BY occurred_at ASC",
        )
        .bind::<Text, _>(project_id)
        .load::<FallbackEventRow>(conn)
        .map_err(|e| format!("list fallback events: {}", e))
    }

    pub fn create_projection(
        &self,
        project_id: &str,
        user_id: i32,
        title: &str,
        description: &str,
        content_type: Option<&str>,
        platform: Option<&str>,
        reserve_cents: i64,
    ) -> Result<(), String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "INSERT INTO gm_drama_project_projections \
             (project_id, user_id, title, description, status, content_type, platform, cost_reserve_cents) \
             VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7) \
             ON CONFLICT (project_id) DO NOTHING",
        )
        .bind::<Text, _>(project_id)
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(title)
        .bind::<Text, _>(description)
        .bind::<Nullable<Text>, _>(content_type)
        .bind::<Nullable<Text>, _>(platform)
        .bind::<BigInt, _>(reserve_cents)
        .execute(conn)
        .map_err(|e| format!("create projection: {}", e))?;

        diesel::sql_query(
            "INSERT INTO gm_drama.projects \
             (project_id, user_id, title, description, status, content_type, metadata, cost_reserve_cents) \
             VALUES ($1, $2, $3, $4, 'pending', $5, $6, $7) \
             ON CONFLICT (project_id) DO NOTHING",
        )
        .bind::<Text, _>(project_id)
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(title)
        .bind::<Text, _>(description)
        .bind::<Nullable<Text>, _>(content_type)
        .bind::<Jsonb, _>(&serde_json::json!({
            "platform": platform,
        }))
        .bind::<BigInt, _>(reserve_cents)
        .execute(conn)
        .map_err(|e| format!("create canonical project: {}", e))?;
        Ok(())
    }

    pub fn create_job(
        &self,
        project_id: &str,
        stage_code: Option<&str>,
        job_type: &str,
        idempotency_key: Option<&str>,
        request_payload: &serde_json::Value,
    ) -> Result<i64, String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "INSERT INTO gm_drama.jobs \
             (project_id, stage_code, job_type, status, worker_name, idempotency_key, request_payload) \
             VALUES ($1, $2, $3, 'pending', 'huobao-drama', $4, $5) \
             RETURNING id",
        )
        .bind::<Text, _>(project_id)
        .bind::<Nullable<Text>, _>(stage_code)
        .bind::<Text, _>(job_type)
        .bind::<Nullable<Text>, _>(idempotency_key)
        .bind::<Jsonb, _>(request_payload)
        .get_result::<IdOnly>(conn)
        .map(|row| row.id)
        .map_err(|e| format!("create job: {}", e))
    }

    pub fn mark_running(
        &self,
        project_id: &str,
        current_stage: Option<&str>,
    ) -> Result<(), String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "UPDATE gm_drama_project_projections \
             SET status = 'running', pending_stage = NULL, interaction_payload = NULL, current_stage = COALESCE($2, current_stage), updated_at = NOW() \
             WHERE project_id = $1",
        )
        .bind::<Text, _>(project_id)
        .bind::<Nullable<Text>, _>(current_stage)
        .execute(conn)
        .map_err(|e| format!("mark projection running: {}", e))?;

        diesel::sql_query(
            "UPDATE gm_drama.projects \
             SET status = 'running', pending_stage = NULL, current_stage = COALESCE($2, current_stage), metadata = metadata - 'interaction_payload', updated_at = NOW() \
             WHERE project_id = $1",
        )
        .bind::<Text, _>(project_id)
        .bind::<Nullable<Text>, _>(current_stage)
        .execute(conn)
        .map_err(|e| format!("mark canonical project running: {}", e))?;
        Ok(())
    }

    pub fn cancel_project(&self, project_id: &str) -> Result<(), String> {
        let conn = &mut self.db.pool.get().map_err(|e| format!("db pool: {}", e))?;
        diesel::sql_query(
            "UPDATE gm_drama_project_projections \
             SET status = 'cancelled', pending_stage = NULL, interaction_payload = NULL, updated_at = NOW() \
             WHERE project_id = $1",
        )
        .bind::<Text, _>(project_id)
        .execute(conn)
        .map_err(|e| format!("cancel projection: {}", e))?;

        diesel::sql_query(
            "UPDATE gm_drama.projects \
             SET status = 'cancelled', pending_stage = NULL, updated_at = NOW() \
             WHERE project_id = $1",
        )
        .bind::<Text, _>(project_id)
        .execute(conn)
        .map_err(|e| format!("cancel canonical project: {}", e))?;
        Ok(())
    }
}

fn extract_job_id(event: &DramaCallbackEvent) -> Option<i64> {
    event.payload.get("job_id").and_then(|v| {
        v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

fn sync_job_status(
    conn: &mut diesel::PgConnection,
    event: &DramaCallbackEvent,
    status: &str,
) -> Result<(), String> {
    let job_id = match extract_job_id(event) {
        Some(job_id) => job_id,
        None => return Ok(()),
    };

    let result_payload = if status == "completed" {
        Some(event.payload.clone())
    } else {
        None
    };
    let error_payload = if status == "failed" {
        Some(event.payload.clone())
    } else {
        None
    };

    diesel::sql_query(
        "UPDATE gm_drama.jobs \
         SET status = $2,
             started_at = CASE WHEN $2 = 'processing' AND started_at IS NULL THEN NOW() ELSE started_at END,
             completed_at = CASE WHEN $2 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END,
             result_payload = COALESCE($3, result_payload),
             error_payload = COALESCE($4, error_payload),
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind::<BigInt, _>(job_id)
    .bind::<Text, _>(status)
    .bind::<Nullable<Jsonb>, _>(result_payload)
    .bind::<Nullable<Jsonb>, _>(error_payload)
    .execute(conn)
    .map_err(|e| format!("sync job status: {}", e))?;

    Ok(())
}

fn project_is_cancelled(
    conn: &mut diesel::PgConnection,
    project_id: &str,
) -> Result<bool, String> {
    #[derive(QueryableByName)]
    struct StatusOnly {
        #[diesel(sql_type = Text)]
        status: String,
    }

    let row = diesel::sql_query(
        "SELECT status FROM gm_drama.projects WHERE project_id = $1",
    )
    .bind::<Text, _>(project_id)
    .get_result::<StatusOnly>(conn)
    .optional()
    .map_err(|e| format!("read project cancel status: {}", e))?;

    Ok(matches!(row, Some(StatusOnly { status }) if status == "cancelled"))
}

#[derive(QueryableByName)]
struct IdOnly {
    #[diesel(sql_type = BigInt)]
    id: i64,
}
