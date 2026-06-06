//! Module D2: task-template draft persistence + lifecycle state machine.
//!
//! The draft is the persisted proposal of an assistant-assembled task config.
//! Its lifecycle is a small state machine:
//!
//! ```text
//!   proposed ──begin_confirm──▶ confirming ──finish_confirm──▶ confirmed
//!      │  ▲                          │
//!      │  └──── revert_confirm ──────┘   (create failed: hand the draft back)
//!      ├── cancel ──▶ cancelled
//!      ├── (stale, TTL) ──▶ expired
//!      └── propose(newer) ──▶ superseded
//! ```
//!
//! Transitions are compare-and-swap (CAS): `UPDATE ... WHERE id=? AND
//! status=from`. That makes concurrent confirms safe — exactly one wins. The
//! data layer is abstracted behind [`DraftStore`] so the lifecycle logic in
//! [`DraftService`] is deterministically testable with an in-memory fake, and
//! Diesel-backed in production.
//!
//! NOTE (Module D2 scope): this module owns draft persistence + transitions
//! ONLY. Wiring it to sample generation (Module B), batch-create (Module C),
//! the tool registry, and the chat handlers is the SEPARATE unit D3.

use crate::error::api_error::ApiError;
use crate::service::ai_chat::task_spec::TaskKind;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// The lifecycle state of a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    Proposed,
    Confirming,
    Confirmed,
    Cancelled,
    Expired,
    Superseded,
}

impl DraftStatus {
    /// The canonical wire string (snake_case), matching the DB CHECK constraint
    /// values and the serde representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            DraftStatus::Proposed => "proposed",
            DraftStatus::Confirming => "confirming",
            DraftStatus::Confirmed => "confirmed",
            DraftStatus::Cancelled => "cancelled",
            DraftStatus::Expired => "expired",
            DraftStatus::Superseded => "superseded",
        }
    }

    /// Parse a wire string back into a [`DraftStatus`]. Returns `None` for any
    /// value outside the known set.
    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "proposed" => Some(DraftStatus::Proposed),
            "confirming" => Some(DraftStatus::Confirming),
            "confirmed" => Some(DraftStatus::Confirmed),
            "cancelled" => Some(DraftStatus::Cancelled),
            "expired" => Some(DraftStatus::Expired),
            "superseded" => Some(DraftStatus::Superseded),
            _ => None,
        }
    }
}

/// A task-template draft as seen by the service layer.
#[derive(Debug, Clone)]
pub struct TemplateDraft {
    pub id: Uuid,
    pub conversation_id: i32,
    pub message_id: Option<i32>,
    pub user_id: i32,
    pub task_kind: TaskKind,
    pub draft_config: JsonValue,
    pub sample_source: String,
    pub status: DraftStatus,
    pub created_entity_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// The draft data layer. Faked in unit tests; Diesel-backed in production.
///
/// All transitions are compare-and-swap so they are safe under concurrency.
#[async_trait]
pub trait DraftStore: Send + Sync {
    /// Insert a fresh `proposed` draft and return it.
    async fn insert_proposed(
        &self,
        conv_id: i32,
        msg_id: Option<i32>,
        user_id: i32,
        kind: TaskKind,
        draft_config: JsonValue,
        sample_source: &str,
    ) -> Result<TemplateDraft, ApiError>;

    /// Move every OTHER `proposed` draft for `(conv_id, kind)` to `superseded`
    /// (i.e. all proposed drafts whose id != `except_id`). Returns the number of
    /// rows superseded.
    async fn supersede_prior_proposed(
        &self,
        conv_id: i32,
        kind: TaskKind,
        except_id: Uuid,
    ) -> Result<u64, ApiError>;

    /// Fetch a draft by id, scoped to `user_id` (ownership check). `None` if no
    /// such draft is visible to the user.
    async fn get(&self, draft_id: Uuid, user_id: i32) -> Result<Option<TemplateDraft>, ApiError>;

    /// Compare-and-swap the status: `UPDATE ... WHERE id=? AND status=from SET
    /// status=to`. Returns `true` iff exactly one row was updated (i.e. the
    /// draft was in `from`).
    async fn cas_status(
        &self,
        draft_id: Uuid,
        from: DraftStatus,
        to: DraftStatus,
    ) -> Result<bool, ApiError>;

    /// Finish a confirm: `confirming -> confirmed`, stamping the created entity
    /// id and the stored result.
    async fn finish_confirmed(
        &self,
        draft_id: Uuid,
        created_entity_id: i32,
        result: JsonValue,
    ) -> Result<(), ApiError>;
}

/// Lifecycle service over a [`DraftStore`]. `ttl_hours` bounds how long a
/// `proposed` draft stays actionable before it is expired on next touch.
pub struct DraftService<S: DraftStore> {
    store: S,
    ttl_hours: i64,
}

impl<S: DraftStore> DraftService<S> {
    pub fn new(store: S, ttl_hours: i64) -> Self {
        Self { store, ttl_hours }
    }

    /// Borrow the underlying store (used by tests to introspect a fake).
    pub fn store_ref(&self) -> &S {
        &self.store
    }

    /// Consume the service and return the underlying store (used by tests to
    /// assert the final persisted state of a fake).
    pub fn into_store(self) -> S {
        self.store
    }

    /// Propose a new draft: insert it `proposed`, then supersede every prior
    /// `proposed` draft for the same `(conv_id, kind)` so a conversation only
    /// ever has one live proposal per kind.
    ///
    /// CONTRACT (encoded by `propose_inserts_proposed_and_supersedes_prior`):
    /// the returned draft is `Proposed`, and any prior proposed draft for the
    /// same conv+kind is moved to `Superseded`.
    pub async fn propose(
        &self,
        conv_id: i32,
        msg_id: Option<i32>,
        user_id: i32,
        kind: TaskKind,
        draft_config: JsonValue,
        sample_source: &str,
    ) -> Result<TemplateDraft, ApiError> {
        // Insert the fresh `proposed` draft, then supersede every PRIOR proposed
        // draft for the same (conv, kind) so a conversation only ever has one
        // live proposal per kind. The new draft is excluded by id.
        let draft = self
            .store
            .insert_proposed(conv_id, msg_id, user_id, kind, draft_config, sample_source)
            .await?;
        self.store
            .supersede_prior_proposed(conv_id, kind, draft.id)
            .await?;
        Ok(draft)
    }

    /// Begin a confirm on a `proposed` draft.
    ///
    /// CONTRACT (encoded by the lifecycle tests):
    /// - missing draft ⇒ `Err(ApiError::NotFound)`;
    /// - stale `proposed` (created_at < now - ttl) ⇒ CAS `proposed -> expired`
    ///   then `Err(ApiError::DraftExpired)`;
    /// - status != `proposed` ⇒ `Err(ApiError::DraftNotActionable)`;
    /// - else CAS `proposed -> confirming`; if the CAS loses (false), re-read
    ///   and return `Err` reflecting the current state;
    /// - on success, return the (now `confirming`) draft.
    pub async fn begin_confirm(
        &self,
        draft_id: Uuid,
        user_id: i32,
        now: DateTime<Utc>,
    ) -> Result<TemplateDraft, ApiError> {
        let draft = match self.store.get(draft_id, user_id).await? {
            Some(d) => d,
            None => return Err(ApiError::NotFound("draft not found".to_string())),
        };

        // Expire-if-stale: a `proposed` draft past its TTL is transitioned to
        // `expired` (best-effort CAS) and rejected.
        if draft.status == DraftStatus::Proposed && self.is_stale(&draft, now) {
            let _ = self
                .store
                .cas_status(draft_id, DraftStatus::Proposed, DraftStatus::Expired)
                .await?;
            return Err(ApiError::DraftExpired);
        }

        // Only a `proposed` draft can begin a confirm.
        if draft.status != DraftStatus::Proposed {
            return Err(ApiError::DraftNotActionable);
        }

        // CAS proposed -> confirming. Exactly one concurrent caller wins.
        let won = self
            .store
            .cas_status(draft_id, DraftStatus::Proposed, DraftStatus::Confirming)
            .await?;

        // Re-read the post-CAS state regardless of who won, so we report the
        // truth (the winner sees `confirming`; the loser sees whatever the
        // winner moved it to).
        let current = self.store.get(draft_id, user_id).await?;
        if won {
            current.ok_or_else(|| ApiError::NotFound("draft not found".to_string()))
        } else {
            // Someone else won the CAS (or it raced into a terminal state):
            // this caller cannot act on it.
            Err(ApiError::DraftNotActionable)
        }
    }

    /// Finish a confirm once the underlying entity has been created:
    /// `confirming -> confirmed`, stamping the entity id and result.
    pub async fn finish_confirm(
        &self,
        draft_id: Uuid,
        created_entity_id: i32,
        result: JsonValue,
    ) -> Result<(), ApiError> {
        // Move confirming -> confirmed, stamping the created entity id + result.
        self.store
            .finish_confirmed(draft_id, created_entity_id, result)
            .await
    }

    /// Revert a confirm when entity creation failed: `confirming -> proposed`,
    /// so the user can retry.
    ///
    /// D2 hardening (review finding): the CAS is `WHERE status='confirming'`. If
    /// it loses (the draft was concurrently moved out of `confirming`), surface
    /// that as [`ApiError::DraftNotActionable`] rather than a silent `Ok`, so a
    /// lost revert is observable. Uses the existing `bool` from `cas_status`, so
    /// no [`DraftStore`] trait signature changes (the in-memory test fakes are
    /// unaffected — they only revert a winning `confirming` draft).
    pub async fn revert_confirm(&self, draft_id: Uuid) -> Result<(), ApiError> {
        let won = self
            .store
            .cas_status(draft_id, DraftStatus::Confirming, DraftStatus::Proposed)
            .await?;
        if won {
            Ok(())
        } else {
            Err(ApiError::DraftNotActionable)
        }
    }

    /// Cancel a draft: expire-if-stale, then `proposed -> cancelled`. A terminal
    /// draft (already cancelled/confirmed/expired/superseded) ⇒
    /// `Err(ApiError::DraftNotActionable)`.
    pub async fn cancel(
        &self,
        draft_id: Uuid,
        user_id: i32,
        now: DateTime<Utc>,
    ) -> Result<(), ApiError> {
        let draft = match self.store.get(draft_id, user_id).await? {
            Some(d) => d,
            None => return Err(ApiError::NotFound("draft not found".to_string())),
        };

        // Expire-if-stale: a `proposed` draft past its TTL is expired and is no
        // longer cancellable.
        if draft.status == DraftStatus::Proposed && self.is_stale(&draft, now) {
            let _ = self
                .store
                .cas_status(draft_id, DraftStatus::Proposed, DraftStatus::Expired)
                .await?;
            return Err(ApiError::DraftExpired);
        }

        // Only a `proposed` draft can be cancelled; anything terminal (or
        // confirming) is not actionable.
        if draft.status != DraftStatus::Proposed {
            return Err(ApiError::DraftNotActionable);
        }

        // CAS proposed -> cancelled. If the CAS loses (someone moved it first),
        // the draft is no longer actionable for this caller.
        let won = self
            .store
            .cas_status(draft_id, DraftStatus::Proposed, DraftStatus::Cancelled)
            .await?;
        if won {
            Ok(())
        } else {
            Err(ApiError::DraftNotActionable)
        }
    }

    /// A `proposed` draft is stale once its `created_at` predates the TTL window
    /// ending at `now` (i.e. `created_at < now - ttl_hours`).
    fn is_stale(&self, draft: &TemplateDraft, now: DateTime<Utc>) -> bool {
        draft.created_at < now - Duration::hours(self.ttl_hours)
    }
}

/// The canonical DB `task_kind` string for a [`TaskKind`] (matches the
/// `task_kind IN ('campaign','publish_plan')` CHECK constraint).
fn task_kind_db_str(kind: TaskKind) -> &'static str {
    match kind {
        TaskKind::Campaign => "campaign",
        TaskKind::PublishPlan => "publish_plan",
    }
}

/// Parse a DB `task_kind` string back into a [`TaskKind`]. Returns an error for
/// any value outside the known set (which the CHECK constraint should preclude).
fn task_kind_from_db(s: &str) -> Result<TaskKind, ApiError> {
    match s {
        "campaign" => Ok(TaskKind::Campaign),
        "publish_plan" => Ok(TaskKind::PublishPlan),
        other => Err(ApiError::DatabaseError(format!(
            "unknown task_kind in draft row: {other}"
        ))),
    }
}

// ===========================================================================
// Diesel-backed prod store.
//
// The deterministic unit suite (`draft_lifecycle_test.rs`) does NOT use this —
// it constructs `DraftService` with its own in-memory CAS fake. The DB-backed
// integration suite (`draft_lifecycle_db_test.rs`) exercises the real schema.
// ===========================================================================

use crate::config::database::DBPool;
use diesel::prelude::*;
use glance_mind_db::entity::ai_task_template_draft::{NewTaskTemplateDraft, TaskTemplateDraft};

/// Diesel-backed draft store (prod).
#[derive(Clone)]
pub struct DieselDraftStore {
    pool: DBPool,
}

impl DieselDraftStore {
    pub fn new(pool: DBPool) -> Self {
        Self { pool }
    }

    /// Map a persisted row into the service-layer [`TemplateDraft`].
    fn row_to_draft(row: TaskTemplateDraft) -> Result<TemplateDraft, ApiError> {
        let task_kind = task_kind_from_db(&row.task_kind)?;
        let status = DraftStatus::from_wire(&row.status).ok_or_else(|| {
            ApiError::DatabaseError(format!("unknown draft status in row: {}", row.status))
        })?;
        Ok(TemplateDraft {
            id: row.id,
            conversation_id: row.conversation_id,
            message_id: row.message_id,
            user_id: row.user_id,
            task_kind,
            draft_config: row.draft_config,
            sample_source: row.sample_source,
            status,
            created_entity_id: row.created_entity_id,
            created_at: row.created_at,
        })
    }
}

#[async_trait]
impl DraftStore for DieselDraftStore {
    async fn insert_proposed(
        &self,
        conv_id: i32,
        msg_id: Option<i32>,
        user_id: i32,
        kind: TaskKind,
        draft_config: JsonValue,
        sample_source: &str,
    ) -> Result<TemplateDraft, ApiError> {
        use glance_mind_db::schema::gm_ai_task_template_drafts;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // INSERT RETURNING the fresh row. `status` defaults to 'proposed' at the
        // DB level (omitted from `NewTaskTemplateDraft`).
        let row: TaskTemplateDraft = diesel::insert_into(gm_ai_task_template_drafts::table)
            .values(&NewTaskTemplateDraft {
                conversation_id: conv_id,
                message_id: msg_id,
                user_id,
                task_kind: task_kind_db_str(kind).to_string(),
                draft_config,
                sample_source: sample_source.to_string(),
            })
            .returning(TaskTemplateDraft::as_returning())
            .get_result(&mut conn)
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Self::row_to_draft(row)
    }

    async fn supersede_prior_proposed(
        &self,
        conv_id: i32,
        kind: TaskKind,
        except_id: Uuid,
    ) -> Result<u64, ApiError> {
        use glance_mind_db::schema::gm_ai_task_template_drafts::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // UPDATE ... SET status='superseded'
        //   WHERE conversation_id=? AND task_kind=? AND status='proposed' AND id<>except
        let n = diesel::update(
            dsl::gm_ai_task_template_drafts
                .filter(dsl::conversation_id.eq(conv_id))
                .filter(dsl::task_kind.eq(task_kind_db_str(kind)))
                .filter(dsl::status.eq(DraftStatus::Proposed.as_str()))
                .filter(dsl::id.ne(except_id)),
        )
        .set((
            dsl::status.eq(DraftStatus::Superseded.as_str()),
            dsl::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(n as u64)
    }

    async fn get(&self, draft_id: Uuid, user_id: i32) -> Result<Option<TemplateDraft>, ApiError> {
        use glance_mind_db::schema::gm_ai_task_template_drafts::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // SELECT by id, scoped to the owning user.
        let row: Option<TaskTemplateDraft> = dsl::gm_ai_task_template_drafts
            .filter(dsl::id.eq(draft_id))
            .filter(dsl::user_id.eq(user_id))
            .select(TaskTemplateDraft::as_select())
            .first(&mut conn)
            .optional()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_draft(r)?)),
            None => Ok(None),
        }
    }

    async fn cas_status(
        &self,
        draft_id: Uuid,
        from: DraftStatus,
        to: DraftStatus,
    ) -> Result<bool, ApiError> {
        use glance_mind_db::schema::gm_ai_task_template_drafts::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // UPDATE ... SET status=to, updated_at=now() WHERE id=? AND status=from.
        // Exactly one row is affected iff the draft was in `from`.
        let affected = diesel::update(
            dsl::gm_ai_task_template_drafts
                .filter(dsl::id.eq(draft_id))
                .filter(dsl::status.eq(from.as_str())),
        )
        .set((
            dsl::status.eq(to.as_str()),
            dsl::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(affected == 1)
    }

    async fn finish_confirmed(
        &self,
        draft_id: Uuid,
        created_entity_id: i32,
        result: JsonValue,
    ) -> Result<(), ApiError> {
        use glance_mind_db::schema::gm_ai_task_template_drafts::dsl;

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // UPDATE ... SET status='confirmed', created_entity_id=?, result=?,
        //   updated_at=now() WHERE id=? AND status='confirming'.
        let affected = diesel::update(
            dsl::gm_ai_task_template_drafts
                .filter(dsl::id.eq(draft_id))
                .filter(dsl::status.eq(DraftStatus::Confirming.as_str())),
        )
        .set((
            dsl::status.eq(DraftStatus::Confirmed.as_str()),
            dsl::created_entity_id.eq(Some(created_entity_id)),
            dsl::result.eq(Some(result)),
            dsl::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // D2 hardening (review finding): the CAS is `WHERE status='confirming'`,
        // so zero affected rows means the draft was concurrently moved out of
        // `confirming` (raced revert / second confirm / expiry). Surface that
        // loss as a typed error instead of silently reporting success — without
        // changing the trait return type (the in-memory test fakes keep their
        // `Result<(), ApiError>` signature; they only ever call this on the
        // winning `confirming` path).
        if affected == 0 {
            return Err(ApiError::DraftNotActionable);
        }

        Ok(())
    }
}
