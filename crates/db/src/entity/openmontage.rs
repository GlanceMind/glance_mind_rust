//! OpenMontage Module Entities
//! Entity definitions for OpenMontage professional video production.

use crate::schema::{gm_openmontage_assets, gm_openmontage_job_events, gm_openmontage_jobs};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// =============================================================================
// OpenMontage Job Entity
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_openmontage_jobs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenmontageJob {
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
    pub budget_limit_usd: Option<BigDecimal>,
    pub last_event_sequence: i64,
    pub next_event_sequence: i64,
    pub sync_required: bool,
    pub snapshot_json: JsonValue,
    pub error_json: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_openmontage_jobs)]
pub struct NewOpenmontageJob {
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

#[derive(Debug, Clone, Default, AsChangeset)]
#[diesel(table_name = gm_openmontage_jobs)]
pub struct UpdateOpenmontageJob {
    pub status: Option<String>,
    pub cancel_requested: Option<bool>,
    pub current_stage: Option<String>,
    pub progress_pct: Option<i32>,
    pub last_event_sequence: Option<i64>,
    pub next_event_sequence: Option<i64>,
    pub sync_required: Option<bool>,
    pub snapshot_json: Option<JsonValue>,
    pub error_json: Option<JsonValue>,
    pub updated_at: Option<DateTime<Utc>>,
}

// =============================================================================
// OpenMontage Job Event Entity
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_openmontage_job_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenmontageJobEvent {
    pub id: i32,
    pub job_id: String,
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub status: Option<String>,
    pub stage: Option<String>,
    pub progress_pct: Option<i32>,
    pub event_json: JsonValue,
    pub emitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_openmontage_job_events)]
pub struct NewOpenmontageJobEvent {
    pub job_id: String,
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    pub status: Option<String>,
    pub stage: Option<String>,
    pub progress_pct: Option<i32>,
    pub event_json: JsonValue,
    pub emitted_at: Option<DateTime<Utc>>,
}

// =============================================================================
// OpenMontage Asset Entity
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_openmontage_assets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OpenmontageAsset {
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_openmontage_assets)]
pub struct NewOpenmontageAsset {
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
