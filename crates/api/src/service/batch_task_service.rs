//! Module C: Batch Create API service (single-item MVP).
//!
//! `BatchTaskService::create_batch` is the one public entry point. It is
//! deliberately decoupled from real campaign/aipub creation and from the
//! database via two traits:
//!
//!   * [`SingleTaskCreator`] — creates exactly one underlying resource and
//!     returns its id. Faked in unit tests; in prod it dispatches to the
//!     campaign / publish-plan services.
//!   * [`BatchDedupeStore`] — the write-ahead idempotency ledger
//!     (`gm_ai_batch_creates`). Faked in unit tests; Diesel-backed in prod.
//!
//! NOTE (Module C skeleton): the body of `create_batch` below is an
//! intentionally-WRONG stub so the RED tests fail on assertions. The
//! implementer replaces it with the real flow described in
//! [`BatchTaskService::create_batch`]'s contract docs.

use crate::dto::batch_task_dto::BatchCreateResultDto;
use crate::error::api_error::ApiError;
use crate::service::ai_chat::task_spec::TaskKind;
use async_trait::async_trait;
use serde_json::Value as JsonValue;

/// Creates a single underlying resource (campaign or publish-plan) and returns
/// its database id.
#[async_trait]
pub trait SingleTaskCreator: Send + Sync {
    async fn create(&self, user_id: i32, kind: TaskKind, item: JsonValue) -> Result<i32, ApiError>;
}

/// Outcome of attempting to begin (write-ahead) a batch.
#[derive(Debug, Clone)]
pub enum DedupeOutcome {
    /// No prior row — this is a fresh batch; the caller proceeds.
    Fresh,
    /// A prior batch with this key already completed; return its result verbatim.
    Completed(BatchCreateResultDto),
    /// A prior batch with this key is still running; reject with 409.
    InProgress,
}

/// Write-ahead idempotency ledger.
#[async_trait]
pub trait BatchDedupeStore: Send + Sync {
    /// Insert an `in_progress` row for `(user_id, key)`. On unique violation,
    /// read the existing row and report whether it is `Completed` or still
    /// `InProgress`. A successful insert reports `Fresh`.
    async fn begin(&self, user_id: i32, key: &str, kind: &str) -> Result<DedupeOutcome, ApiError>;

    /// Mark the `(user_id, key)` row `completed` and persist `result`.
    async fn complete(
        &self,
        user_id: i32,
        key: &str,
        result: &BatchCreateResultDto,
    ) -> Result<(), ApiError>;
}

/// Orchestrates a single-item batch create with write-ahead idempotency.
pub struct BatchTaskService<C: SingleTaskCreator, D: BatchDedupeStore> {
    creator: C,
    dedupe: D,
}

impl<C: SingleTaskCreator, D: BatchDedupeStore> BatchTaskService<C, D> {
    pub fn new(creator: C, dedupe: D) -> Self {
        Self { creator, dedupe }
    }

    /// Create a single-item batch.
    ///
    /// Contract (encoded by the RED tests — the implementer must satisfy all):
    /// 1. `dto.items.len() != 1` ⇒ `Err(ApiError::MultiItemNotSupported)` (400);
    ///    the creator is NOT called.
    /// 2. `dto.task_kind` parses: "campaign" ⇒ `TaskKind::Campaign`,
    ///    "publish_plan" ⇒ `TaskKind::PublishPlan`; anything else ⇒ 400.
    /// 3. `dedupe.begin(user, key, kind)`:
    ///      * `Completed(r)` ⇒ return `r` (creator NOT called),
    ///      * `InProgress`   ⇒ `Err(ApiError::BatchInProgress)` (409, creator NOT called),
    ///      * `Fresh`        ⇒ proceed.
    /// 4. `creator.create(user, kind, items[0])`:
    ///      * `Ok(id)` ⇒ results: [{index:0, status:"created", id:Some(id), error:None}],
    ///        created_count:1, failed_count:0, atomic_rolled_back:false.
    ///      * `Err(e)` ⇒ results: [{index:0, status:"failed", id:None,
    ///        error:Some(redacted)}], created_count:0, failed_count:1. The error
    ///        strings MUST be redacted (no `Bearer`/`sk-` tokens).
    /// 5. `dedupe.complete(user, key, &result)`, then return `result`.
    pub async fn create_batch(
        &self,
        _user_id: i32,
        _dto: crate::dto::batch_task_dto::BatchCreateTasksDto,
    ) -> Result<BatchCreateResultDto, ApiError> {
        // INTENTIONALLY-WRONG STUB (Module C RED phase).
        // Does not validate item count or task_kind, never consults the dedupe
        // store, and never invokes the creator. Returns a vacuous result so the
        // contract tests fail on assertions rather than panic. Replace wholesale
        // with the real flow.
        //
        // Touch the fields so the unused-field lint does not turn into an error
        // in CI; this performs no behavioral work.
        let _ = (&self.creator, &self.dedupe);
        Ok(BatchCreateResultDto {
            results: Vec::new(),
            created_count: 0,
            failed_count: 0,
            atomic_rolled_back: false,
        })
    }
}

// ===========================================================================
// Production stubs (Module C RED phase — wrong but compiling).
//
// These are the concrete `SingleTaskCreator` / `BatchDedupeStore` impls the
// HTTP handler wires up. They are deliberately incomplete so that any
// DB-backed integration test exercising them fails on assertions. The
// deterministic unit suite does NOT use these — it constructs the service with
// its own in-memory fakes.
// ===========================================================================

use crate::config::database::DBPool;
use crate::state::user_state::UserState;

/// Diesel-backed idempotency ledger (prod). STUB: see `begin`/`complete`.
#[derive(Clone)]
pub struct DieselBatchDedupeStore {
    #[allow(dead_code)]
    pool: DBPool,
}

impl DieselBatchDedupeStore {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BatchDedupeStore for DieselBatchDedupeStore {
    async fn begin(
        &self,
        _user_id: i32,
        _key: &str,
        _kind: &str,
    ) -> Result<DedupeOutcome, ApiError> {
        // WRONG STUB: always reports Fresh; never inserts the write-ahead row,
        // so a replay is NOT deduplicated. Real impl inserts an in_progress row
        // and, on unique violation, reads the existing row.
        Ok(DedupeOutcome::Fresh)
    }

    async fn complete(
        &self,
        _user_id: i32,
        _key: &str,
        _result: &BatchCreateResultDto,
    ) -> Result<(), ApiError> {
        // WRONG STUB: no-op; never persists completion.
        Ok(())
    }
}

/// Dispatches single-item creation to the campaign / publish-plan services
/// (prod). STUB: returns an error regardless of kind.
#[derive(Clone)]
pub struct DispatchSingleTaskCreator {
    #[allow(dead_code)]
    state: UserState,
}

impl DispatchSingleTaskCreator {
    pub fn new(state: UserState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SingleTaskCreator for DispatchSingleTaskCreator {
    async fn create(
        &self,
        _user_id: i32,
        _kind: TaskKind,
        _item: JsonValue,
    ) -> Result<i32, ApiError> {
        // WRONG STUB: no real campaign/aipub creation in the MVP skeleton.
        Err(ApiError::InternalServerError(
            "batch single-task creator not implemented".to_string(),
        ))
    }
}
