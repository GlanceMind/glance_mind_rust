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
use chrono::{DateTime, Utc};
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
        // WRONG STUB (Module D2 RED): inserts the proposed draft but does NOT
        // supersede prior proposed drafts. The real impl must call
        // `supersede_prior_proposed(conv_id, kind, except = new.id)`.
        let draft = self
            .store
            .insert_proposed(conv_id, msg_id, user_id, kind, draft_config, sample_source)
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
        // WRONG STUB (Module D2 RED): returns whatever `get` finds with NO
        // expiry check, NO state-machine check, and NO CAS to `confirming`.
        // The real impl must expire-if-stale, reject non-proposed drafts with
        // DraftNotActionable, and CAS proposed -> confirming (admitting exactly
        // one concurrent caller).
        let _ = (now, self.ttl_hours);
        match self.store.get(draft_id, user_id).await? {
            Some(draft) => Ok(draft),
            None => Err(ApiError::NotFound("draft not found".to_string())),
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
        // WRONG STUB (Module D2 RED): no-op. The real impl must call
        // `store.finish_confirmed(...)` to move confirming -> confirmed and
        // persist the entity id + result.
        let _ = (draft_id, created_entity_id, result);
        Ok(())
    }

    /// Revert a confirm when entity creation failed: `confirming -> proposed`,
    /// so the user can retry.
    pub async fn revert_confirm(&self, draft_id: Uuid) -> Result<(), ApiError> {
        // WRONG STUB (Module D2 RED): no-op. The real impl must CAS
        // confirming -> proposed.
        let _ = draft_id;
        Ok(())
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
        // WRONG STUB (Module D2 RED): unconditionally reports success without
        // touching the store. The real impl must get + expire-if-stale + CAS
        // proposed -> cancelled, and reject terminal drafts.
        let _ = (draft_id, user_id, now, self.ttl_hours);
        Ok(())
    }
}
