//! Module C — deterministic (no-DB) contract tests for `BatchTaskService::create_batch`.
//!
//! These encode the single-item MVP behavioral contract. They are RED against
//! the wrong skeleton stub (which returns a vacuous result without consulting
//! the dedupe store or creator) and must turn GREEN only when the real flow is
//! implemented. The implementer MUST NOT weaken these assertions.
//!
//! Run:
//!   cargo test -p glance_mind_api --test batch_task_property_test

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use glance_mind_api::dto::batch_task_dto::{BatchCreateResultDto, BatchCreateTasksDto};
use glance_mind_api::error::api_error::ApiError;
use glance_mind_api::response::error_code::ErrorCode;
use glance_mind_api::service::ai_chat::task_spec::TaskKind;
use glance_mind_api::service::batch_task_service::{
    BatchDedupeStore, BatchTaskService, DedupeOutcome, SingleTaskCreator,
};
use proptest::prelude::*;
use serde_json::{json, Value as JsonValue};

// ---------------------------------------------------------------------------
// Fakes
// ---------------------------------------------------------------------------

/// What the fake creator should return for each call.
#[derive(Clone)]
enum CreatorOutcome {
    Ok(i32),
    /// Fail with this exact (unredacted) message.
    Err(String),
}

/// Shared call-log so a test can inspect call count after the fake is moved
/// into the service.
type CallLog = Arc<Mutex<Vec<(i32, TaskKind, JsonValue)>>>;

/// Records every `create` call and returns a fixed outcome.
struct FakeCreator {
    calls: CallLog,
    outcome: CreatorOutcome,
}

impl FakeCreator {
    /// Returns the fake plus a shared handle to its call log.
    fn new(outcome: CreatorOutcome) -> (Self, CallLog) {
        let calls: CallLog = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                calls: calls.clone(),
                outcome,
            },
            calls,
        )
    }
}

#[async_trait]
impl SingleTaskCreator for FakeCreator {
    async fn create(&self, user_id: i32, kind: TaskKind, item: JsonValue) -> Result<i32, ApiError> {
        self.calls.lock().unwrap().push((user_id, kind, item));
        match &self.outcome {
            CreatorOutcome::Ok(id) => Ok(*id),
            CreatorOutcome::Err(msg) => Err(ApiError::InternalServerError(msg.clone())),
        }
    }
}

#[derive(Clone)]
enum DedupeStatus {
    InProgress,
    Completed(BatchCreateResultDto),
}

/// In-memory dedupe store. `begin` inserts an in_progress entry on first sight
/// (returning `Fresh`); a second `begin` for the same key returns whatever is
/// stored (InProgress unless `complete` has upgraded it to Completed). Optional
/// `forced` lets a test pin the first outcome (e.g. force InProgress).
struct InMemoryDedupe {
    map: Mutex<HashMap<(i32, String), DedupeStatus>>,
    forced: Option<DedupeStatus>,
}

impl InMemoryDedupe {
    fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            forced: None,
        }
    }

    /// Pin the outcome of the *first* begin to InProgress (no entry stored).
    fn forced_in_progress() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
            forced: Some(DedupeStatus::InProgress),
        }
    }
}

#[async_trait]
impl BatchDedupeStore for InMemoryDedupe {
    async fn begin(&self, user_id: i32, key: &str, _kind: &str) -> Result<DedupeOutcome, ApiError> {
        if let Some(forced) = &self.forced {
            return Ok(match forced {
                DedupeStatus::InProgress => DedupeOutcome::InProgress,
                DedupeStatus::Completed(r) => DedupeOutcome::Completed(r.clone()),
            });
        }
        let mut map = self.map.lock().unwrap();
        match map.get(&(user_id, key.to_string())) {
            Some(DedupeStatus::Completed(r)) => Ok(DedupeOutcome::Completed(r.clone())),
            Some(DedupeStatus::InProgress) => Ok(DedupeOutcome::InProgress),
            None => {
                map.insert((user_id, key.to_string()), DedupeStatus::InProgress);
                Ok(DedupeOutcome::Fresh)
            }
        }
    }

    async fn complete(
        &self,
        user_id: i32,
        key: &str,
        result: &BatchCreateResultDto,
    ) -> Result<(), ApiError> {
        self.map.lock().unwrap().insert(
            (user_id, key.to_string()),
            DedupeStatus::Completed(result.clone()),
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn dto(task_kind: &str, key: &str, items: Vec<JsonValue>) -> BatchCreateTasksDto {
    BatchCreateTasksDto {
        task_kind: task_kind.to_string(),
        mode: None,
        idempotency_key: key.to_string(),
        source_draft_id: None,
        items,
    }
}

fn err_code(e: &ApiError) -> ErrorCode {
    e.to_error_code()
}

fn calls_made(log: &CallLog) -> usize {
    log.lock().unwrap().len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// (1) items.len() == 2 ⇒ MultiItemNotSupported (400), creator NOT called.
#[tokio::test]
async fn rejects_multi_item() {
    let (creator, calls) = FakeCreator::new(CreatorOutcome::Ok(1));
    let dedupe = InMemoryDedupe::new();
    let svc = BatchTaskService::new(creator, dedupe);

    let result = svc
        .create_batch(42, dto("campaign", "k-multi", vec![json!({}), json!({})]))
        .await;

    let err = result.expect_err("two items must be rejected");
    assert_eq!(
        err_code(&err),
        ErrorCode::MultiItemNotSupported,
        "multi-item batch must map to MultiItemNotSupported (4503/400)"
    );
    assert_eq!(
        calls_made(&calls),
        0,
        "creator must NOT be called when item count is rejected"
    );
}

/// (2) unknown task_kind ⇒ 400, creator NOT called.
#[tokio::test]
async fn rejects_unknown_task_kind() {
    let (creator, calls) = FakeCreator::new(CreatorOutcome::Ok(7));
    let dedupe = InMemoryDedupe::new();
    let svc = BatchTaskService::new(creator, dedupe);

    let result = svc
        .create_batch(
            1,
            dto("not_a_real_kind", "k-unknown", vec![json!({"x": 1})]),
        )
        .await;

    let err = result.expect_err("unknown task_kind must be rejected");
    assert_eq!(
        err_code(&err).http_status(),
        axum::http::StatusCode::BAD_REQUEST,
        "unknown task_kind must yield a 400"
    );
    assert_eq!(
        calls_made(&calls),
        0,
        "creator must NOT be called for an unknown task_kind"
    );
}

/// (3) dedupe begin returns InProgress ⇒ Err(BatchInProgress) (409), creator NOT called.
#[tokio::test]
async fn in_progress_returns_batch_in_progress() {
    let (creator, calls) = FakeCreator::new(CreatorOutcome::Ok(99));
    let dedupe = InMemoryDedupe::forced_in_progress();
    let svc = BatchTaskService::new(creator, dedupe);

    let result = svc
        .create_batch(5, dto("campaign", "k-inflight", vec![json!({"a": 1})]))
        .await;

    let err = result.expect_err("an in-progress batch must be rejected");
    assert_eq!(
        err_code(&err),
        ErrorCode::BatchInProgress,
        "an in-progress duplicate must map to BatchInProgress"
    );
    assert_eq!(
        err_code(&err).http_status(),
        axum::http::StatusCode::CONFLICT,
        "BatchInProgress must be a 409"
    );
    assert_eq!(
        calls_made(&calls),
        0,
        "creator must NOT be called when a duplicate is in progress"
    );
}

/// (4-Ok) creator Ok(123) ⇒ created_count==1, results[0].id==Some(123), status "created".
#[tokio::test]
async fn single_create_success_result_shape() {
    let (creator, calls) = FakeCreator::new(CreatorOutcome::Ok(123));
    let dedupe = InMemoryDedupe::new();
    let svc = BatchTaskService::new(creator, dedupe);

    let result = svc
        .create_batch(8, dto("campaign", "k-ok", vec![json!({"name": "x"})]))
        .await
        .expect("a fresh single-item create must succeed");

    assert_eq!(result.created_count, 1, "created_count must be 1");
    assert_eq!(result.failed_count, 0, "failed_count must be 0");
    assert!(
        !result.atomic_rolled_back,
        "atomic_rolled_back must be false"
    );
    assert_eq!(result.results.len(), 1, "exactly one item result");
    let item = &result.results[0];
    assert_eq!(item.index, 0, "item index must be 0");
    assert_eq!(item.status, "created", "item status must be \"created\"");
    assert_eq!(item.id, Some(123), "item id must be Some(123)");
    assert!(item.error.is_none(), "successful item must carry no error");
    assert_eq!(
        calls_made(&calls),
        1,
        "creator must be called exactly once on a fresh create"
    );
}

/// (4-Err) creator Err whose message embeds a Bearer token ⇒ failed_count==1,
/// results[0].error present and REDACTED (contains no "Bearer").
#[tokio::test]
async fn single_create_failure_result_is_redacted() {
    let leaky = "upstream 401: Authorization: Bearer xyz sk-secret-123";
    let (creator, _calls) = FakeCreator::new(CreatorOutcome::Err(leaky.to_string()));
    let dedupe = InMemoryDedupe::new();
    let svc = BatchTaskService::new(creator, dedupe);

    let result = svc
        .create_batch(3, dto("campaign", "k-fail", vec![json!({"name": "y"})]))
        .await
        .expect("a creator error must still return Ok(result) with a failed item");

    assert_eq!(
        result.created_count, 0,
        "created_count must be 0 on failure"
    );
    assert_eq!(result.failed_count, 1, "failed_count must be 1 on failure");
    assert_eq!(result.results.len(), 1, "exactly one item result");
    let item = &result.results[0];
    assert_eq!(item.status, "failed", "item status must be \"failed\"");
    assert!(item.id.is_none(), "failed item must have no id");
    let error = item
        .error
        .as_ref()
        .expect("a failed item must carry an error detail");
    assert!(
        !error.msg.contains("Bearer") && !error.msg_cn.contains("Bearer"),
        "error strings must be redacted: must not contain 'Bearer' (got msg={:?}, msg_cn={:?})",
        error.msg,
        error.msg_cn
    );
    assert!(
        !error.msg.contains("sk-") && !error.msg_cn.contains("sk-"),
        "error strings must be redacted: must not contain 'sk-' token"
    );
}

// ---------------------------------------------------------------------------
// Property: idempotent replay
// ---------------------------------------------------------------------------

proptest! {
    /// Two `create_batch` calls with the SAME key ⇒ the creator runs exactly
    /// once total, and both calls return identical results.
    #[test]
    fn prop_idempotent_replay(
        key in "[a-zA-Z0-9_-]{1,32}",
        n in 1u64..1_000_000u64,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let item = json!({ "name": format!("item-{n}"), "n": n });
            let (creator, calls) = FakeCreator::new(CreatorOutcome::Ok(777));
            let dedupe = InMemoryDedupe::new();
            let svc = BatchTaskService::new(creator, dedupe);

            let first = svc
                .create_batch(11, dto("campaign", &key, vec![item.clone()]))
                .await
                .expect("first create must succeed");
            let second = svc
                .create_batch(11, dto("campaign", &key, vec![item.clone()]))
                .await
                .expect("replay must succeed");

            prop_assert_eq!(
                calls_made(&calls),
                1,
                "creator must run exactly once across a replayed key"
            );
            prop_assert_eq!(
                &first, &second,
                "a replayed idempotency key must return an identical result"
            );
            prop_assert_eq!(first.created_count, 1, "first replay created_count must be 1");
            Ok(())
        })?;
    }
}
