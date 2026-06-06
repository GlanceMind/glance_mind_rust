//! Module D2 — deterministic (no-DB) lifecycle tests for `DraftService`.
//!
//! These encode the draft state-machine contract. They run against an
//! `InMemoryDraftStore` whose `cas_status` faithfully fakes a real
//! compare-and-swap (it mutates a row ONLY if its current status == `from`),
//! so the concurrency admission test is meaningful without a database.
//!
//! They are RED against the wrong skeleton stub (which returns `begin_confirm`
//! Ok without any CAS, never supersedes on propose, and treats cancel as a
//! no-op) and must turn GREEN only when the real lifecycle is implemented. The
//! implementer MUST NOT weaken these assertions.
//!
//! Run:
//!   cargo test -p glance_mind_api --test draft_lifecycle_test

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use glance_mind_api::error::api_error::ApiError;
use glance_mind_api::response::error_code::ErrorCode;
use glance_mind_api::service::ai_chat::task_spec::TaskKind;
use glance_mind_api::service::ai_chat::task_template::draft::{
    DraftService, DraftStatus, DraftStore, TemplateDraft,
};
use serde_json::json;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// In-memory fake store with REAL compare-and-swap semantics.
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

    /// Read a stored draft's status directly (test introspection).
    fn status_of(&self, id: Uuid) -> Option<DraftStatus> {
        self.rows.lock().unwrap().get(&id).map(|d| d.status)
    }

    /// Read a stored draft's created_entity_id directly (test introspection).
    fn entity_id_of(&self, id: Uuid) -> Option<i32> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .and_then(|d| d.created_entity_id)
    }

    /// Seed a draft row with an explicit status + created_at (test setup).
    fn seed(&self, draft: TemplateDraft) {
        self.rows.lock().unwrap().insert(draft.id, draft);
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
        draft_config: serde_json::Value,
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

    /// REAL CAS: flip status from -> to ONLY if the current status == from.
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
        result: serde_json::Value,
    ) -> Result<(), ApiError> {
        let mut rows = self.rows.lock().unwrap();
        if let Some(d) = rows.get_mut(&draft_id) {
            // confirming -> confirmed, stamping the entity id + result.
            if d.status == DraftStatus::Confirming {
                d.status = DraftStatus::Confirmed;
                d.created_entity_id = Some(created_entity_id);
                let _ = result;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TTL_HOURS: i64 = 24;

fn err_code(e: &ApiError) -> ErrorCode {
    e.to_error_code()
}

/// Build a draft row directly (for seeding into the store).
fn draft_row(
    id: Uuid,
    conv_id: i32,
    user_id: i32,
    kind: TaskKind,
    status: DraftStatus,
    created_at: DateTime<Utc>,
) -> TemplateDraft {
    TemplateDraft {
        id,
        conversation_id: conv_id,
        message_id: None,
        user_id,
        task_kind: kind,
        draft_config: json!({"name": "x"}),
        sample_source: "ai_generated".to_string(),
        status,
        created_entity_id: None,
        created_at,
    }
}

// ---------------------------------------------------------------------------
// DraftStatus round-trip (serde snake_case + &str)
// ---------------------------------------------------------------------------

#[test]
fn draft_status_str_round_trips() {
    for s in [
        DraftStatus::Proposed,
        DraftStatus::Confirming,
        DraftStatus::Confirmed,
        DraftStatus::Cancelled,
        DraftStatus::Expired,
        DraftStatus::Superseded,
    ] {
        let wire = s.as_str();
        assert_eq!(
            DraftStatus::from_wire(wire),
            Some(s),
            "DraftStatus must round-trip through its &str wire form ({wire})"
        );
        // serde snake_case must agree with as_str().
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            json,
            format!("\"{wire}\""),
            "serde repr must equal the snake_case wire form"
        );
    }
}

// ---------------------------------------------------------------------------
// propose
// ---------------------------------------------------------------------------

/// A prior proposed draft for the same conv+kind becomes Superseded; the new
/// one is Proposed.
#[tokio::test]
async fn propose_inserts_proposed_and_supersedes_prior() {
    let store = InMemoryDraftStore::new();

    // Seed a prior proposed draft for conv 7 / campaign.
    let prior_id = Uuid::new_v4();
    store.seed(draft_row(
        prior_id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        Utc::now(),
    ));

    let svc = DraftService::new(store, TTL_HOURS);
    let new_draft = svc
        .propose(
            7,
            None,
            100,
            TaskKind::Campaign,
            json!({"name": "fresh"}),
            "ai_generated",
        )
        .await
        .expect("propose must succeed");

    assert_eq!(
        new_draft.status,
        DraftStatus::Proposed,
        "the newly proposed draft must be Proposed"
    );

    // Reach back into the store to confirm the prior draft was superseded and
    // the new one is proposed.
    let store = svc.into_store();
    assert_eq!(
        store.status_of(prior_id),
        Some(DraftStatus::Superseded),
        "the prior proposed draft for the same conv+kind must become Superseded"
    );
    assert_eq!(
        store.status_of(new_draft.id),
        Some(DraftStatus::Proposed),
        "the newly proposed draft must remain Proposed"
    );
}

// ---------------------------------------------------------------------------
// begin_confirm
// ---------------------------------------------------------------------------

/// Two concurrent begin_confirm on ONE proposed draft ⇒ exactly one returns the
/// confirming draft; the other gets DraftNotActionable.
#[tokio::test]
async fn begin_confirm_cas_admits_one() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        Utc::now(),
    ));
    let svc = DraftService::new(store, TTL_HOURS);
    let now = Utc::now();

    // Drive both begins "concurrently". The in-memory CAS is atomic under its
    // Mutex, so exactly one transition proposed -> confirming may succeed.
    let (a, b) = tokio::join!(
        svc.begin_confirm(id, 100, now),
        svc.begin_confirm(id, 100, now),
    );

    let oks = [&a, &b].iter().filter(|r| r.is_ok()).count();
    let actionable_errs = [&a, &b]
        .iter()
        .filter(|r| matches!(r, Err(e) if err_code(e) == ErrorCode::DraftNotActionable))
        .count();

    assert_eq!(
        oks, 1,
        "exactly one concurrent begin_confirm may win the CAS (got {oks} Ok). \
         a={a:?}, b={b:?}"
    );
    assert_eq!(
        actionable_errs, 1,
        "the losing begin_confirm must fail with DraftNotActionable (got {actionable_errs}). \
         a={a:?}, b={b:?}"
    );

    // The winner's returned draft must be in Confirming.
    let winner = a.or(b).expect("one begin_confirm must have returned Ok");
    assert_eq!(
        winner.status,
        DraftStatus::Confirming,
        "the winning begin_confirm must return the draft in Confirming"
    );

    let store = svc.into_store();
    assert_eq!(
        store.status_of(id),
        Some(DraftStatus::Confirming),
        "the stored draft must end in Confirming after one successful begin_confirm"
    );
}

/// cancel then begin_confirm ⇒ DraftNotActionable.
#[tokio::test]
async fn confirm_on_cancelled_rejected() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        Utc::now(),
    ));
    let svc = DraftService::new(store, TTL_HOURS);
    let now = Utc::now();

    svc.cancel(id, 100, now).await.expect("cancel must succeed");

    let err = svc
        .begin_confirm(id, 100, now)
        .await
        .expect_err("begin_confirm on a cancelled draft must error");
    assert_eq!(
        err_code(&err),
        ErrorCode::DraftNotActionable,
        "confirming a cancelled draft must map to DraftNotActionable"
    );
}

/// created_at = now-25h, ttl 24h ⇒ Err(DraftExpired) AND the stored status
/// becomes expired.
#[tokio::test]
async fn begin_confirm_on_stale_proposed_expires() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    let now = Utc::now();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        now - Duration::hours(25),
    ));
    let svc = DraftService::new(store, TTL_HOURS);

    let err = svc
        .begin_confirm(id, 100, now)
        .await
        .expect_err("a stale proposed draft must not be confirmable");
    assert_eq!(
        err_code(&err),
        ErrorCode::DraftExpired,
        "confirming a stale (TTL-exceeded) draft must map to DraftExpired"
    );

    let store = svc.into_store();
    assert_eq!(
        store.status_of(id),
        Some(DraftStatus::Expired),
        "a stale proposed draft must be transitioned to Expired on begin_confirm"
    );
}

/// finish a confirm, then begin_confirm again ⇒ DraftNotActionable.
#[tokio::test]
async fn confirm_on_confirmed_rejected() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    let now = Utc::now();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        now,
    ));
    let svc = DraftService::new(store, TTL_HOURS);

    // Take it all the way to confirmed.
    svc.begin_confirm(id, 100, now)
        .await
        .expect("begin_confirm must succeed");
    svc.finish_confirm(id, 555, json!({"ok": true}))
        .await
        .expect("finish_confirm must succeed");

    let err = svc
        .begin_confirm(id, 100, now)
        .await
        .expect_err("begin_confirm on a confirmed draft must error");
    assert_eq!(
        err_code(&err),
        ErrorCode::DraftNotActionable,
        "confirming an already-confirmed draft must map to DraftNotActionable"
    );
}

// ---------------------------------------------------------------------------
// cancel
// ---------------------------------------------------------------------------

/// cancel marks Cancelled; a second cancel ⇒ DraftNotActionable.
#[tokio::test]
async fn cancel_marks_cancelled_then_recancel_rejected() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    let now = Utc::now();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        now,
    ));
    let svc = DraftService::new(store, TTL_HOURS);

    svc.cancel(id, 100, now)
        .await
        .expect("first cancel must succeed");

    // The stored draft must be Cancelled.
    {
        let store = svc.store_ref();
        assert_eq!(
            store.status_of(id),
            Some(DraftStatus::Cancelled),
            "cancel must move a proposed draft to Cancelled"
        );
    }

    let err = svc
        .cancel(id, 100, now)
        .await
        .expect_err("re-cancelling a terminal draft must error");
    assert_eq!(
        err_code(&err),
        ErrorCode::DraftNotActionable,
        "re-cancelling a cancelled draft must map to DraftNotActionable"
    );
}

// ---------------------------------------------------------------------------
// finish_confirm / revert_confirm
// ---------------------------------------------------------------------------

/// after begin_confirm, finish_confirm ⇒ status Confirmed, created_entity_id set.
#[tokio::test]
async fn finish_confirm_sets_confirmed_with_entity_id() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    let now = Utc::now();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        now,
    ));
    let svc = DraftService::new(store, TTL_HOURS);

    svc.begin_confirm(id, 100, now)
        .await
        .expect("begin_confirm must succeed");
    svc.finish_confirm(id, 4242, json!({"campaign_id": 4242}))
        .await
        .expect("finish_confirm must succeed");

    let store = svc.into_store();
    assert_eq!(
        store.status_of(id),
        Some(DraftStatus::Confirmed),
        "finish_confirm must move confirming -> Confirmed"
    );
    assert_eq!(
        store.entity_id_of(id),
        Some(4242),
        "finish_confirm must persist the created_entity_id"
    );
}

/// revert_confirm returns a confirming draft to proposed (create failed path).
#[tokio::test]
async fn revert_confirm_returns_to_proposed() {
    let store = InMemoryDraftStore::new();
    let id = Uuid::new_v4();
    let now = Utc::now();
    store.seed(draft_row(
        id,
        7,
        100,
        TaskKind::Campaign,
        DraftStatus::Proposed,
        now,
    ));
    let svc = DraftService::new(store, TTL_HOURS);

    svc.begin_confirm(id, 100, now)
        .await
        .expect("begin_confirm must succeed");
    // Sanity: it is now confirming.
    assert_eq!(svc.store_ref().status_of(id), Some(DraftStatus::Confirming));

    svc.revert_confirm(id)
        .await
        .expect("revert_confirm must succeed");

    let store = svc.into_store();
    assert_eq!(
        store.status_of(id),
        Some(DraftStatus::Proposed),
        "revert_confirm must move confirming -> Proposed so the user can retry"
    );
}
