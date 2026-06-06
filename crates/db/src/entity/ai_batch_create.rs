//! Module C: `gm_ai_batch_creates` entity.
//!
//! Write-ahead idempotency ledger backing the batch-create API. A
//! `NewBatchCreate` row is inserted with status `in_progress` before any
//! sub-task work happens; once the batch settles, the row is updated to
//! `completed` with the serialized result. The `UNIQUE(user_id,
//! idempotency_key)` constraint makes a replayed request collide, at which
//! point the service reads the existing row instead of doing the work again.

use crate::schema::gm_ai_batch_creates;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// A persisted batch-create record.
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = gm_ai_batch_creates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BatchCreate {
    pub id: Uuid,
    pub user_id: i32,
    pub idempotency_key: String,
    pub task_kind: String,
    /// "in_progress" | "completed"
    pub status: String,
    /// Serialized `BatchCreateResultDto`; populated when status == "completed".
    pub result: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insertable write-ahead row. `status` defaults to 'in_progress' at the DB
/// level, so it is intentionally omitted here.
#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = gm_ai_batch_creates)]
pub struct NewBatchCreate {
    pub user_id: i32,
    pub idempotency_key: String,
    pub task_kind: String,
}
