//! Module C: Batch Create API DTOs.
//!
//! Single-item MVP (`items.len() == 1`). The request carries the task kind,
//! an idempotency key (the dedupe identity), and a one-element `items` array.
//! The response reports per-item create results plus aggregate counts.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use validator::Validate;

/// `POST /ai-tasks/batch` request body.
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct BatchCreateTasksDto {
    /// "campaign" | "publish_plan". Unknown kinds are rejected (400).
    pub task_kind: String,
    /// Optional creation mode hint (unused by the MVP service logic).
    pub mode: Option<String>,
    /// Idempotency identity. Must be non-empty.
    #[validate(length(min = 1, message = "idempotency_key must not be empty"))]
    pub idempotency_key: String,
    /// Module D threads this through; accepted but unused here.
    pub source_draft_id: Option<uuid::Uuid>,
    /// The items to create. MVP supports exactly one.
    pub items: Vec<JsonValue>,
}

/// Per-item create result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchItemResult {
    pub index: usize,
    /// "created" | "failed".
    pub status: String,
    /// Created resource id (when status == "created").
    pub id: Option<i32>,
    /// Redacted error detail (when status == "failed").
    pub error: Option<BatchItemError>,
}

/// Redacted error detail for a failed item. No secrets (Bearer tokens, sk-…
/// keys) may appear in `msg` / `msg_cn`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchItemError {
    pub code: i32,
    pub msg: String,
    pub msg_cn: String,
}

/// `POST /ai-tasks/batch` response body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchCreateResultDto {
    pub results: Vec<BatchItemResult>,
    pub created_count: usize,
    pub failed_count: usize,
    /// Always false in the single-item MVP (multi-item atomic saga is deferred).
    pub atomic_rolled_back: bool,
}
