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

use crate::dto::batch_task_dto::{BatchCreateResultDto, BatchItemError, BatchItemResult};
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

/// Parse the wire `task_kind` string into a [`TaskKind`].
///
/// "campaign" ⇒ [`TaskKind::Campaign`], "publish_plan" ⇒
/// [`TaskKind::PublishPlan`]; anything else is a 400.
fn parse_task_kind(task_kind: &str) -> Result<TaskKind, ApiError> {
    match task_kind {
        "campaign" => Ok(TaskKind::Campaign),
        "publish_plan" => Ok(TaskKind::PublishPlan),
        other => Err(ApiError::BadRequest(format!(
            "unsupported task_kind: {other}"
        ))),
    }
}

/// Build a redacted [`BatchItemError`] from an [`ApiError`].
///
/// The raw error string (`ApiError`'s `Display`) can embed upstream provider
/// detail including credentials (`Authorization: Bearer …`, `sk-…`). We MUST
/// NOT echo it. Instead we derive the client-facing strings purely from the
/// error's classification: the stable [`ErrorCode`] plus its canned
/// English/Chinese messages. That guarantees no secret token survives into the
/// per-item error.
fn redact_error(e: &ApiError) -> BatchItemError {
    let code = e.to_error_code();
    BatchItemError {
        code: code.code(),
        msg: code.message().to_string(),
        msg_cn: code.message_cn().to_string(),
    }
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
    /// Contract (encoded by the deterministic property tests):
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
    ///        strings are redacted (no `Bearer`/`sk-` tokens).
    /// 5. `dedupe.complete(user, key, &result)`, then return `result`.
    pub async fn create_batch(
        &self,
        user_id: i32,
        dto: crate::dto::batch_task_dto::BatchCreateTasksDto,
    ) -> Result<BatchCreateResultDto, ApiError> {
        // (1) Single-item MVP: reject anything that is not exactly one item, and
        // do so BEFORE consulting the dedupe store or the creator.
        if dto.items.len() != 1 {
            return Err(ApiError::MultiItemNotSupported);
        }

        // (2) Validate the task kind up-front (still before any side effect).
        let kind = parse_task_kind(&dto.task_kind)?;

        // (3) Write-ahead dedupe gate.
        match self
            .dedupe
            .begin(user_id, &dto.idempotency_key, &dto.task_kind)
            .await?
        {
            DedupeOutcome::Completed(result) => return Ok(result),
            DedupeOutcome::InProgress => return Err(ApiError::BatchInProgress),
            DedupeOutcome::Fresh => {}
        }

        // (4) Fresh batch: create the single underlying resource.
        //
        // `dto.items` has exactly one element (checked above); clone it so the
        // creator owns the value.
        let item = dto.items[0].clone();
        let result = match self.creator.create(user_id, kind, item).await {
            Ok(id) => BatchCreateResultDto {
                results: vec![BatchItemResult {
                    index: 0,
                    status: "created".to_string(),
                    id: Some(id),
                    error: None,
                }],
                created_count: 1,
                failed_count: 0,
                atomic_rolled_back: false,
            },
            Err(e) => BatchCreateResultDto {
                results: vec![BatchItemResult {
                    index: 0,
                    status: "failed".to_string(),
                    id: None,
                    error: Some(redact_error(&e)),
                }],
                created_count: 0,
                failed_count: 1,
                atomic_rolled_back: false,
            },
        };

        // (5) Persist completion (idempotent replay returns this verbatim), then
        // return the result.
        self.dedupe
            .complete(user_id, &dto.idempotency_key, &result)
            .await?;
        Ok(result)
    }
}

// ===========================================================================
// Production implementations.
//
// These are the concrete `SingleTaskCreator` / `BatchDedupeStore` impls the
// HTTP handler wires up. The deterministic unit suite does NOT use these — it
// constructs the service with its own in-memory fakes. The DB-backed
// integration suite (`batch_task_test.rs`) exercises `DieselBatchDedupeStore`.
// ===========================================================================

use crate::config::database::DBPool;
use crate::state::user_state::UserState;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// Diesel-backed idempotency ledger (prod).
#[derive(Clone)]
pub struct DieselBatchDedupeStore {
    pool: DBPool,
}

impl DieselBatchDedupeStore {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BatchDedupeStore for DieselBatchDedupeStore {
    async fn begin(&self, user_id: i32, key: &str, kind: &str) -> Result<DedupeOutcome, ApiError> {
        use glance_mind_db::entity::ai_batch_create::{BatchCreate, NewBatchCreate};
        use glance_mind_db::schema::gm_ai_batch_creates::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Attempt the write-ahead INSERT. The DB defaults `status` to
        // 'in_progress'. A replayed request collides on the
        // UNIQUE(user_id, idempotency_key) constraint.
        let inserted: Result<BatchCreate, DieselError> =
            diesel::insert_into(gm_ai_batch_creates::table)
                .values(&NewBatchCreate {
                    user_id,
                    idempotency_key: key.to_string(),
                    task_kind: kind.to_string(),
                })
                .returning(BatchCreate::as_returning())
                .get_result(&mut conn);

        use glance_mind_db::schema::gm_ai_batch_creates;

        match inserted {
            // Fresh insert succeeded — proceed.
            Ok(_) => Ok(DedupeOutcome::Fresh),
            // Unique violation: a prior batch for this (user, key) exists. Read
            // it back and report its current state.
            Err(DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                let existing: BatchCreate = gm_ai_batch_creates::table
                    .filter(dsl::user_id.eq(user_id))
                    .filter(dsl::idempotency_key.eq(key))
                    .select(BatchCreate::as_select())
                    .first(&mut conn)
                    .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

                if existing.status == "completed" {
                    let stored = existing.result.ok_or_else(|| {
                        ApiError::DatabaseError(
                            "completed batch row has no result payload".to_string(),
                        )
                    })?;
                    let result: BatchCreateResultDto = serde_json::from_value(stored)
                        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;
                    Ok(DedupeOutcome::Completed(result))
                } else {
                    Ok(DedupeOutcome::InProgress)
                }
            }
            Err(e) => Err(ApiError::DatabaseError(e.to_string())),
        }
    }

    async fn complete(
        &self,
        user_id: i32,
        key: &str,
        result: &BatchCreateResultDto,
    ) -> Result<(), ApiError> {
        use glance_mind_db::schema::gm_ai_batch_creates::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let payload =
            serde_json::to_value(result).map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        diesel::update(
            dsl::gm_ai_batch_creates
                .filter(dsl::user_id.eq(user_id))
                .filter(dsl::idempotency_key.eq(key)),
        )
        .set((
            dsl::status.eq("completed"),
            dsl::result.eq(Some(payload)),
            dsl::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

/// Dispatches single-item creation to the campaign / publish-plan services
/// (prod).
#[derive(Clone)]
pub struct DispatchSingleTaskCreator {
    state: UserState,
}

impl DispatchSingleTaskCreator {
    pub fn new(state: UserState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl SingleTaskCreator for DispatchSingleTaskCreator {
    async fn create(&self, user_id: i32, kind: TaskKind, item: JsonValue) -> Result<i32, ApiError> {
        match kind {
            TaskKind::Campaign => {
                let dto: crate::dto::campaign_dto::CampaignCreateDto = serde_json::from_value(item)
                    .map_err(|e| ApiError::BadRequest(format!("invalid campaign item: {e}")))?;
                let created = self
                    .state
                    .campaign_service
                    .create_campaign(user_id, dto)
                    .await?;
                Ok(created.id)
            }
            TaskKind::PublishPlan => {
                let dto: crate::dto::aipub_dto::CreatePlanDto = serde_json::from_value(item)
                    .map_err(|e| ApiError::BadRequest(format!("invalid publish_plan item: {e}")))?;
                let created = self.state.aipub_service.create_plan(user_id, dto).await?;
                Ok(created.id)
            }
        }
    }
}
