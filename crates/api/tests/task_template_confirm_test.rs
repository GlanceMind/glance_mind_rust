//! Module D3 — deterministic confirm-orchestration tests for
//! [`confirm_orchestrate`] (+ `project_fields_to_dto`).
//!
//! These exercise the confirm seam WITHOUT a database or real create services:
//! an `InMemoryDraftStore` (the D2 fake with real compare-and-swap) is seeded
//! with a `proposed` draft, and a `FakeCreator` records the DTO `Value` it is
//! handed (or fails on demand). The function under test must:
//!
//!   * `begin_confirm` the draft (`proposed -> confirming`; propagate
//!     `DraftExpired` / `DraftNotActionable`),
//!   * project the edited fields into the create DTO (folding dotted keys into
//!     nested objects),
//!   * call the creator, and on success `finish_confirm` (draft -> `confirmed`),
//!   * on creator failure `revert_confirm` (draft back to `proposed`) and return
//!     the typed error.
//!
//! They are RED against the skeleton stub (which returns a vacuous result
//! without touching the draft lifecycle or the creator). The implementer MUST
//! NOT weaken these assertions.
//!
//! Run:
//!   cargo test -p glance_mind_api --test task_template_confirm_test

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use glance_mind_api::dto::batch_task_dto::BatchCreateResultDto;
use glance_mind_api::error::api_error::ApiError;
use glance_mind_api::response::error_code::ErrorCode;
use glance_mind_api::service::ai_chat::task_spec::TaskKind;
use glance_mind_api::service::ai_chat::task_template::{
    confirm_orchestrate, DraftService, DraftStatus, DraftStore, EditedField, TemplateDraft,
};
use glance_mind_api::service::ai_chat::SYSTEM_PROMPT;
use glance_mind_api::service::batch_task_service::SingleTaskCreator;
use serde_json::{json, Value};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// FakeCreator — records the (kind, dto Value) it receives, or fails on demand.
// ---------------------------------------------------------------------------

struct FakeCreator {
    /// The created entity id to return on success.
    created_id: i32,
    /// If true, `create` returns `Err` (simulating a validation/billing failure).
    fail: bool,
    /// Captures the (kind, dto Value) of the last `create` call.
    seen: Mutex<Option<(TaskKind, Value)>>,
    /// Counts how many times `create` was invoked.
    calls: Mutex<u32>,
}

impl FakeCreator {
    fn ok(created_id: i32) -> Self {
        Self {
            created_id,
            fail: false,
            seen: Mutex::new(None),
            calls: Mutex::new(0),
        }
    }

    fn failing() -> Self {
        Self {
            created_id: 0,
            fail: true,
            seen: Mutex::new(None),
            calls: Mutex::new(0),
        }
    }

    fn seen_dto(&self) -> Option<(TaskKind, Value)> {
        self.seen.lock().unwrap().clone()
    }

    fn call_count(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}

#[async_trait]
impl SingleTaskCreator for FakeCreator {
    async fn create(&self, _user_id: i32, kind: TaskKind, item: Value) -> Result<i32, ApiError> {
        *self.calls.lock().unwrap() += 1;
        *self.seen.lock().unwrap() = Some((kind, item));
        if self.fail {
            // A user-fixable failure (e.g. insufficient balance). The orchestrator
            // must revert the draft to `proposed` and surface this typed error.
            Err(ApiError::BadRequest("insufficient balance".to_string()))
        } else {
            Ok(self.created_id)
        }
    }
}

// ---------------------------------------------------------------------------
// In-memory fake store with REAL compare-and-swap semantics (D2 pattern).
// ---------------------------------------------------------------------------

struct InMemoryDraftStore {
    rows: Mutex<HashMap<Uuid, TemplateDraft>>,
}

impl InMemoryDraftStore {
    fn new() -> Self {
        Self {
            rows: Mutex::new(HashMap::new()),
        }
    }

    fn seed(&self, draft: TemplateDraft) {
        self.rows.lock().unwrap().insert(draft.id, draft);
    }

    fn status_of(&self, id: Uuid) -> Option<DraftStatus> {
        self.rows.lock().unwrap().get(&id).map(|d| d.status)
    }

    fn entity_id_of(&self, id: Uuid) -> Option<i32> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .and_then(|d| d.created_entity_id)
    }
}

#[async_trait]
impl DraftStore for InMemoryDraftStore {
    async fn insert_proposed(
        &self,
        conv_id: i32,
        msg_id: Option<i32>,
        user_id: i32,
        kind: TaskKind,
        draft_config: Value,
        sample_source: &str,
    ) -> Result<TemplateDraft, ApiError> {
        let draft = TemplateDraft {
            id: Uuid::new_v4(),
            conversation_id: conv_id,
            message_id: msg_id,
            user_id,
            task_kind: kind,
            draft_config,
            sample_source: sample_source.to_string(),
            status: DraftStatus::Proposed,
            created_entity_id: None,
            created_at: Utc::now(),
        };
        self.rows.lock().unwrap().insert(draft.id, draft.clone());
        Ok(draft)
    }

    async fn supersede_prior_proposed(
        &self,
        conv_id: i32,
        kind: TaskKind,
        except_id: Uuid,
    ) -> Result<u64, ApiError> {
        let mut rows = self.rows.lock().unwrap();
        let mut n = 0u64;
        for d in rows.values_mut() {
            if d.id != except_id
                && d.conversation_id == conv_id
                && d.task_kind == kind
                && d.status == DraftStatus::Proposed
            {
                d.status = DraftStatus::Superseded;
                n += 1;
            }
        }
        Ok(n)
    }

    async fn get(&self, draft_id: Uuid, user_id: i32) -> Result<Option<TemplateDraft>, ApiError> {
        Ok(self
            .rows
            .lock()
            .unwrap()
            .get(&draft_id)
            .filter(|d| d.user_id == user_id)
            .cloned())
    }

    async fn cas_status(
        &self,
        draft_id: Uuid,
        from: DraftStatus,
        to: DraftStatus,
    ) -> Result<bool, ApiError> {
        let mut rows = self.rows.lock().unwrap();
        match rows.get_mut(&draft_id) {
            Some(d) if d.status == from => {
                d.status = to;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    async fn finish_confirmed(
        &self,
        draft_id: Uuid,
        created_entity_id: i32,
        _result: Value,
    ) -> Result<(), ApiError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(d) = rows.get_mut(&draft_id) {
            if d.status == DraftStatus::Confirming {
                d.status = DraftStatus::Confirmed;
                d.created_entity_id = Some(created_entity_id);
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TTL_HOURS: i64 = 24;
const CONV_ID: i32 = 7;
const USER_ID: i32 = 100;

/// Seed a `proposed` campaign draft and return (service, draft_id).
fn seeded_proposed_campaign() -> (DraftService<InMemoryDraftStore>, Uuid) {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    store.seed(TemplateDraft {
        id,
        conversation_id: CONV_ID,
        message_id: None,
        user_id: USER_ID,
        task_kind: TaskKind::Campaign,
        draft_config: json!({ "name": "seed" }),
        sample_source: "ai_generated".to_string(),
        status: DraftStatus::Proposed,
        created_entity_id: None,
        created_at: Utc::now(),
    });
    (DraftService::new(store, TTL_HOURS), id)
}

fn edited(key: &str, value: Value) -> EditedField {
    EditedField {
        key: key.to_string(),
        value,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Confirm projects the edited fields into a well-formed campaign DTO `Value`
/// that the creator receives; the draft ends `confirmed` and the result reports
/// created_count == 1 with the created id.
#[tokio::test]
async fn confirm_projects_fields_and_creates() {
    let (drafts, draft_id) = seeded_proposed_campaign();
    let creator = FakeCreator::ok(4242);

    let fields = vec![
        edited("name", json!("Edited Campaign Name")),
        edited("platform_id", json!(2)),
        edited("region_id", json!(1)),
        edited("ai_model_id", json!(2)),
        edited("schedule_type", json!("immediate")),
        edited("product_prompt", json!("Promote the edited product")),
        edited("max_scan_count", json!(75)),
    ];

    let result: BatchCreateResultDto =
        confirm_orchestrate(&drafts, &creator, draft_id, USER_ID, fields, Utc::now())
            .await
            .expect("confirm must succeed when the creator succeeds");

    // The creator must have been called exactly once with a campaign DTO Value
    // carrying the edited values.
    assert_eq!(
        creator.call_count(),
        1,
        "the creator must be invoked exactly once on a successful confirm"
    );
    let (seen_kind, seen_dto) = creator
        .seen_dto()
        .expect("the creator must have received a projected DTO Value");
    assert_eq!(
        seen_kind,
        TaskKind::Campaign,
        "the creator must be called with the draft's task_kind (campaign)"
    );
    assert_eq!(
        seen_dto.get("name").and_then(|v| v.as_str()),
        Some("Edited Campaign Name"),
        "the projected DTO must carry the edited `name`, got {seen_dto}"
    );
    assert_eq!(
        seen_dto.get("platform_id").and_then(|v| v.as_i64()),
        Some(2),
        "the projected DTO must carry the edited `platform_id`, got {seen_dto}"
    );
    assert_eq!(
        seen_dto.get("product_prompt").and_then(|v| v.as_str()),
        Some("Promote the edited product"),
        "the projected DTO must carry the edited `product_prompt`, got {seen_dto}"
    );

    // The draft must end `confirmed` with the created entity id stamped.
    let store = drafts.into_store();
    assert_eq!(
        store.status_of(draft_id),
        Some(DraftStatus::Confirmed),
        "a successful confirm must move the draft to `confirmed`"
    );
    assert_eq!(
        store.entity_id_of(draft_id),
        Some(4242),
        "a successful confirm must stamp the created entity id on the draft"
    );

    // The aggregate result reports exactly one created task.
    assert_eq!(
        result.created_count, 1,
        "a successful confirm must report created_count == 1, got {result:?}"
    );
    assert_eq!(
        result.failed_count, 0,
        "a successful confirm must report failed_count == 0, got {result:?}"
    );
    assert_eq!(
        result.results.first().and_then(|r| r.id),
        Some(4242),
        "the per-item result must carry the created id, got {result:?}"
    );
}

/// A dotted edited key `ai_input.content_prompt` must be folded into a nested
/// object under `ai_input` in the projected DTO Value.
#[tokio::test]
async fn confirm_projection_folds_dotted_keys() {
    let (drafts, draft_id) = seeded_proposed_campaign();
    let creator = FakeCreator::ok(7);

    let fields = vec![
        edited("name", json!("Nested Edit")),
        edited("platform_id", json!(1)),
        edited("region_id", json!(1)),
        edited("ai_model_id", json!(2)),
        edited("schedule_type", json!("immediate")),
        edited("product_prompt", json!("base prompt")),
        edited("max_scan_count", json!(10)),
        // The dotted key must NOT land as a flat "ai_input.content_prompt" key.
        edited("ai_input.content_prompt", json!("make a fun launch post")),
    ];

    confirm_orchestrate(&drafts, &creator, draft_id, USER_ID, fields, Utc::now())
        .await
        .expect("confirm must succeed");

    let (_, seen_dto) = creator
        .seen_dto()
        .expect("the creator must have received a projected DTO Value");

    // The flat dotted key must be absent...
    assert!(
        seen_dto.get("ai_input.content_prompt").is_none(),
        "the dotted key must NOT survive as a flat key, got {seen_dto}"
    );
    // ...and the value must be nested under `ai_input`.
    let nested = seen_dto
        .get("ai_input")
        .and_then(|v| v.get("content_prompt"))
        .and_then(|v| v.as_str());
    assert_eq!(
        nested,
        Some("make a fun launch post"),
        "the dotted key must be folded into a nested object \
         (ai_input.content_prompt), got {seen_dto}"
    );
}

/// When the creator fails, the draft reverts to `proposed` (so the user can
/// edit + retry) and the typed error is surfaced.
#[tokio::test]
async fn confirm_create_failure_reverts_to_proposed() {
    let (drafts, draft_id) = seeded_proposed_campaign();
    let creator = FakeCreator::failing();

    let fields = vec![
        edited("name", json!("Will Fail")),
        edited("platform_id", json!(1)),
        edited("region_id", json!(1)),
        edited("ai_model_id", json!(2)),
        edited("schedule_type", json!("immediate")),
        edited("product_prompt", json!("prompt")),
        edited("max_scan_count", json!(10)),
    ];

    let err = confirm_orchestrate(&drafts, &creator, draft_id, USER_ID, fields, Utc::now())
        .await
        .expect_err("confirm must surface the creator's error");

    assert_eq!(
        err.to_error_code(),
        ErrorCode::BadRequest,
        "the creator's typed error must be surfaced verbatim, got {err:?}"
    );

    // The draft must be handed back to the user as `proposed`.
    let store = drafts.into_store();
    assert_eq!(
        store.status_of(draft_id),
        Some(DraftStatus::Proposed),
        "a failed create must revert the draft to `proposed` for retry"
    );
    assert_eq!(
        store.entity_id_of(draft_id),
        None,
        "a failed create must not stamp any created entity id"
    );
}

/// begin_confirm errors must propagate: a stale (TTL-exceeded) `proposed` draft
/// yields `DraftExpired`, and a `cancelled` draft yields `DraftNotActionable`.
/// Neither calls the creator.
#[tokio::test]
async fn confirm_on_expired_or_cancelled_rejected() {
    let now = Utc::now();

    // --- expired: created 25h ago, ttl 24h ---
    {
        let store = InMemoryDraftStore::new();
        let id = Uuid::new_v4();
        store.seed(TemplateDraft {
            id,
            conversation_id: CONV_ID,
            message_id: None,
            user_id: USER_ID,
            task_kind: TaskKind::Campaign,
            draft_config: json!({ "name": "stale" }),
            sample_source: "ai_generated".to_string(),
            status: DraftStatus::Proposed,
            created_entity_id: None,
            created_at: now - Duration::hours(25),
        });
        let drafts = DraftService::new(store, TTL_HOURS);
        let creator = FakeCreator::ok(1);

        let err = confirm_orchestrate(&drafts, &creator, id, USER_ID, vec![], now)
            .await
            .expect_err("a stale draft must not be confirmable");
        assert_eq!(
            err.to_error_code(),
            ErrorCode::DraftExpired,
            "confirming a stale draft must surface DraftExpired, got {err:?}"
        );
        assert_eq!(
            creator.call_count(),
            0,
            "an expired draft must NOT invoke the creator"
        );
    }

    // --- cancelled: terminal status ---
    {
        let store = InMemoryDraftStore::new();
        let id = Uuid::new_v4();
        store.seed(TemplateDraft {
            id,
            conversation_id: CONV_ID,
            message_id: None,
            user_id: USER_ID,
            task_kind: TaskKind::Campaign,
            draft_config: json!({ "name": "cancelled" }),
            sample_source: "ai_generated".to_string(),
            status: DraftStatus::Cancelled,
            created_entity_id: None,
            created_at: now,
        });
        let drafts = DraftService::new(store, TTL_HOURS);
        let creator = FakeCreator::ok(1);

        let err = confirm_orchestrate(&drafts, &creator, id, USER_ID, vec![], now)
            .await
            .expect_err("a cancelled draft must not be confirmable");
        assert_eq!(
            err.to_error_code(),
            ErrorCode::DraftNotActionable,
            "confirming a cancelled draft must surface DraftNotActionable, got {err:?}"
        );
        assert_eq!(
            creator.call_count(),
            0,
            "a cancelled draft must NOT invoke the creator"
        );
    }
}

// ---------------------------------------------------------------------------
// D6 — SYSTEM_PROMPT directive (non-authoritative UX nudge).
// ---------------------------------------------------------------------------

/// The SYSTEM_PROMPT must mention the backend sample-template flow so the model
/// stops the field-by-field interrogation for sparse create intents.
#[test]
fn system_prompt_mentions_template_flow() {
    assert!(
        SYSTEM_PROMPT.contains("样例模板"),
        "SYSTEM_PROMPT must mention the backend `样例模板` flow for sparse create intents"
    );
}
