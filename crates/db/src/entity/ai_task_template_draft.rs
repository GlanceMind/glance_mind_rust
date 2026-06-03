//! Module D2: `gm_ai_task_template_drafts` entity.
//!
//! A draft persists an assistant-assembled task config (campaign or
//! publish_plan) and its lifecycle `status`. The `draft_config` JSON is
//! write-once: there is intentionally NO setter for it on [`DraftStatusUpdate`],
//! so the AsChangeset can only move the state machine forward and stamp the
//! created entity / result — never rewrite the proposed config.

use crate::schema::gm_ai_task_template_drafts;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// A persisted task-template draft row.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Serialize, Deserialize)]
#[diesel(table_name = gm_ai_task_template_drafts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TaskTemplateDraft {
    pub id: Uuid,
    pub conversation_id: i32,
    pub message_id: Option<i32>,
    pub user_id: i32,
    /// "campaign" | "publish_plan"
    pub task_kind: String,
    pub draft_config: JsonValue,
    /// "ai_generated" | "preset" | "hybrid"
    pub sample_source: String,
    /// "proposed" | "confirming" | "confirmed" | "cancelled" | "expired" | "superseded"
    pub status: String,
    pub created_entity_id: Option<i32>,
    pub result: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Insertable proposal. `status` defaults to 'proposed' at the DB level, so it
/// is intentionally omitted here.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_task_template_drafts)]
pub struct NewTaskTemplateDraft {
    pub conversation_id: i32,
    pub message_id: Option<i32>,
    pub user_id: i32,
    pub task_kind: String,
    pub draft_config: JsonValue,
    pub sample_source: String,
}

/// State-machine update. Deliberately omits `draft_config` so the proposed
/// config stays write-once; only `status`, the created entity id, the stored
/// result, and `updated_at` can be changed.
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = gm_ai_task_template_drafts)]
pub struct DraftStatusUpdate {
    pub status: Option<String>,
    pub created_entity_id: Option<Option<i32>>,
    pub result: Option<Option<JsonValue>>,
    pub updated_at: Option<DateTime<Utc>>,
}
