//! Module D3 — deterministic gate-interception tests for
//! [`evaluate_create_intercept`].
//!
//! These exercise the create-tool interception seam WITHOUT a database or a real
//! LLM: a `FakeSampleLlm` returns a canned valid config, and an
//! `InMemoryDraftStore` (the D2 fake, with real compare-and-swap semantics)
//! backs a real `DraftService`. The function under test must:
//!
//!   * build a `DraftConfig` from the RAW tool-call args,
//!   * run `completeness::evaluate`,
//!   * on `should_offer_template`: generate a sample, persist a `proposed`
//!     draft, and return a `TaskTemplateProposed` SSE event (NOT create), and
//!   * otherwise return `Proceed` (and persist NOTHING).
//!
//! They are RED against the skeleton stub (which unconditionally returns
//! `Proceed`) and must turn GREEN only when the real interception is
//! implemented. The implementer MUST NOT weaken these assertions.
//!
//! Run:
//!   cargo test -p glance_mind_api --test task_template_intercept_test

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use glance_mind_api::error::api_error::ApiError;
use glance_mind_api::response::error_code::ErrorCode;
use glance_mind_api::service::ai_chat::task_spec::TaskKind;
use glance_mind_api::service::ai_chat::task_template::{
    evaluate_create_intercept, DraftService, DraftStatus, DraftStore, InterceptOutcome, LlmFailure,
    SampleLlm, TemplateDraft,
};
use glance_mind_api::service::ai_chat::types::SseEvent;
use serde_json::{json, Value};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// FakeSampleLlm — returns a canned, valid campaign config (never hits network).
// ---------------------------------------------------------------------------

/// A deterministic `SampleLlm` that returns a fully-populated campaign config so
/// `generate_sample` succeeds without a real provider. (For the
/// `TemplateUnavailable` case the generator fails BEFORE any LLM call — it has
/// no preset — so this canned value is never reached there.)
struct FakeSampleLlm;

#[async_trait]
impl SampleLlm for FakeSampleLlm {
    async fn complete_json(
        &self,
        _system: &str,
        _user: &str,
        _max_tokens: u32,
    ) -> Result<Value, LlmFailure> {
        Ok(json!({
            "name": "Fake Generated Campaign",
            "platform_id": 1,
            "region_id": 1,
            "ai_model_id": 2,
            "schedule_type": "immediate",
            "product_prompt": "Promote the product to the target audience.",
            "max_scan_count": 100,
            "target_audience": "general",
            "keyword": "promo",
            "call_to_action": "Buy now",
            "tone_of_voice": "friendly",
            "budget_cap": 500,
            "reply_template_ids": [1, 2]
        }))
    }
}

// ---------------------------------------------------------------------------
// In-memory fake store with REAL compare-and-swap semantics (copied from the D2
// `draft_lifecycle_test.rs` pattern). Counts inserts so a test can assert that
// `Proceed` persisted NOTHING.
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

    fn len(&self) -> usize {
        self.rows.lock().unwrap().len()
    }

    fn any_proposed(&self) -> bool {
        self.rows
            .lock()
            .unwrap()
            .values()
            .any(|d| d.status == DraftStatus::Proposed)
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

fn svc() -> DraftService<InMemoryDraftStore> {
    DraftService::new(InMemoryDraftStore::new(), TTL_HOURS)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Sparse campaign raw args `{name, product_prompt}` ⇒ the gate fires: a
/// `TaskTemplateProposed` event for kind=campaign, blocking=true, missing>=3,
/// AND a draft row now exists in the store.
#[tokio::test]
async fn sparse_campaign_raw_args_proposes_template() {
    let llm = FakeSampleLlm;
    let drafts = svc();
    let raw = json!({ "name": "x", "product_prompt": "y" });

    let outcome = evaluate_create_intercept(
        TaskKind::Campaign,
        &raw,
        &llm,
        &drafts,
        CONV_ID,
        None,
        USER_ID,
    )
    .await
    .expect("intercept must not error for a sparse-but-supported campaign");

    let event = match outcome {
        InterceptOutcome::Proposed(e) => e,
        InterceptOutcome::Proceed => {
            panic!("sparse campaign raw args must PROPOSE a template, got Proceed")
        }
    };

    match event {
        SseEvent::TaskTemplateProposed {
            task_kind,
            blocking,
            missing,
            ..
        } => {
            assert_eq!(
                task_kind,
                TaskKind::Campaign,
                "proposed event must carry task_kind=campaign"
            );
            assert!(
                blocking,
                "a sparse campaign (missing required fields) must be blocking"
            );
            assert!(
                missing.len() >= 3,
                "sparse campaign must report >=3 missing fields, got {missing:?}"
            );
        }
        other => panic!("expected TaskTemplateProposed, got {other:?}"),
    }

    // A draft row must have been persisted (in `proposed`).
    let store = drafts.into_store();
    assert_eq!(
        store.len(),
        1,
        "the proposal must persist exactly one draft row"
    );
    assert!(
        store.any_proposed(),
        "the persisted draft must be in `proposed`"
    );
}

/// Full required campaign raw args ⇒ `Proceed`, and NO draft is inserted.
#[tokio::test]
async fn complete_campaign_raw_args_proceeds() {
    let llm = FakeSampleLlm;
    let drafts = svc();
    // All 7 required + all 6 recommended present ⇒ missing_count == 0 ⇒ no offer.
    let raw = json!({
        "name": "Summer Launch",
        "platform_id": 1,
        "region_id": 1,
        "ai_model_id": 2,
        "schedule_type": "immediate",
        "product_prompt": "Sell the new sneakers",
        "max_scan_count": 50,
        "target_audience": "runners",
        "keyword": "sneakers",
        "call_to_action": "Buy now",
        "tone_of_voice": "friendly",
        "budget_cap": 100,
        "reply_template_ids": [1, 2, 3]
    });

    let outcome = evaluate_create_intercept(
        TaskKind::Campaign,
        &raw,
        &llm,
        &drafts,
        CONV_ID,
        None,
        USER_ID,
    )
    .await
    .expect("intercept must not error for a complete campaign");

    assert!(
        matches!(outcome, InterceptOutcome::Proceed),
        "a complete campaign must PROCEED to create, got Proposed"
    );

    let store = drafts.into_store();
    assert_eq!(
        store.len(),
        0,
        "a Proceed outcome must NOT persist any draft row, found {}",
        store.len()
    );
}

/// Raw args missing only `schedule_type` + `ai_model_id` (which the tool arm
/// would inject) PLUS one recommended field ⇒ raw missing_count == 3 ⇒ Proposed.
/// Post-injection those two required fields would be present (missing_count == 1
/// < threshold) and the stub-or-wrong impl that evaluates POST-injection args
/// would wrongly `Proceed`. This proves the gate sees the RAW args.
#[tokio::test]
async fn injected_defaults_do_not_mask_missing() {
    let llm = FakeSampleLlm;
    let drafts = svc();
    // 5 of 7 required present (missing schedule_type + ai_model_id), and 5 of 6
    // recommended present (missing only `reply_template_ids`). RAW missing = 3
    // ({schedule_type, ai_model_id, reply_template_ids}); after the arm injects
    // schedule_type + ai_model_id, missing would be 1 (< threshold 3).
    let raw = json!({
        "name": "Summer Launch",
        "platform_id": 1,
        "region_id": 1,
        "product_prompt": "Sell the new sneakers",
        "max_scan_count": 50,
        "target_audience": "runners",
        "keyword": "sneakers",
        "call_to_action": "Buy now",
        "tone_of_voice": "friendly",
        "budget_cap": 100
    });

    let outcome = evaluate_create_intercept(
        TaskKind::Campaign,
        &raw,
        &llm,
        &drafts,
        CONV_ID,
        None,
        USER_ID,
    )
    .await
    .expect("intercept must not error");

    let event = match outcome {
        InterceptOutcome::Proposed(e) => e,
        InterceptOutcome::Proceed => panic!(
            "raw args missing schedule_type+ai_model_id (+1 recommended) must PROPOSE \
             (gate must evaluate RAW args, not post-injection), got Proceed"
        ),
    };

    match event {
        SseEvent::TaskTemplateProposed {
            blocking, missing, ..
        } => {
            assert!(
                blocking,
                "missing required schedule_type/ai_model_id must be blocking"
            );
            // The two injected-default keys must be reported missing (the gate
            // saw the RAW args), and total missing must be >= the threshold.
            assert!(
                missing.iter().any(|k| k == "schedule_type"),
                "missing must include schedule_type (raw-args evaluation), got {missing:?}"
            );
            assert!(
                missing.iter().any(|k| k == "ai_model_id"),
                "missing must include ai_model_id (raw-args evaluation), got {missing:?}"
            );
            assert!(
                missing.len() >= 3,
                "raw missing_count must be >= 3 to fire the gate, got {missing:?}"
            );
        }
        other => panic!("expected TaskTemplateProposed, got {other:?}"),
    }
}

/// A sparse publish-plan with an UNKNOWN `plan_type` has no preset, so
/// `generate_sample` returns `TemplateUnavailable`. The intercept must map that
/// to `ApiError::TemplateUnavailable` (NOT Proceed, NOT Proposed), and persist
/// no draft.
#[tokio::test]
async fn template_unavailable_maps_to_error() {
    let llm = FakeSampleLlm;
    let drafts = svc();
    // Unknown plan_type ⇒ no preset ⇒ generator fails before any LLM call. The
    // draft is also missing required platform_id/content_type/group_id ⇒ the gate
    // fires, so the intercept reaches generation and must surface the error.
    let raw = json!({ "plan_type": "__nonexistent_plan_type__" });

    let result = evaluate_create_intercept(
        TaskKind::PublishPlan,
        &raw,
        &llm,
        &drafts,
        CONV_ID,
        None,
        USER_ID,
    )
    .await;

    let err = match result {
        Err(e) => e,
        Ok(InterceptOutcome::Proceed) => {
            panic!("an unsupported template combo must NOT Proceed (no creatable task)")
        }
        Ok(InterceptOutcome::Proposed(_)) => {
            panic!("an unsupported template combo must NOT Propose (no preset exists)")
        }
    };
    assert_eq!(
        err.to_error_code(),
        ErrorCode::TemplateUnavailable,
        "a missing preset must map to TemplateUnavailable, got {err:?}"
    );

    let store = drafts.into_store();
    assert_eq!(
        store.len(),
        0,
        "a TemplateUnavailable failure must persist no draft, found {}",
        store.len()
    );
}
