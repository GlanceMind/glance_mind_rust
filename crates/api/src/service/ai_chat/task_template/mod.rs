//! Sample-template generation for assistant-assisted task creation (Module B1).
//!
//! - [`generator`]: the public [`generator::generate_sample`] entrypoint, its
//!   [`generator::SampleLlm`] trait, and the rendered [`generator::SampleTemplate`]
//!   data model.
//! - [`presets`]: deterministic, hand-curated fallback templates.
//! - [`validate`]: spec-conformance checking via [`validate::against_spec`].
//! - [`llm`]: the production [`llm::LlmClientSampleLlm`] (DeepSeek-backed B2).
//! - [`draft`]: Module D2 — draft persistence + lifecycle state machine.

pub mod draft;
pub mod generator;
pub mod llm;
pub mod presets;
pub mod validate;

pub use draft::{DieselDraftStore, DraftService, DraftStatus, DraftStore, TemplateDraft};
pub use generator::{
    generate_sample, EnumOption, GenerateError, LlmFailure, SampleField, SampleLlm, SampleSource,
    SampleTemplate,
};
pub use llm::LlmClientSampleLlm;

// ===========================================================================
// Module D3: assistant integration orchestration.
//
// Two deterministic-testable orchestration functions wire the completeness gate
// (A), the sample generator (B), the batch create service (C), and the draft
// lifecycle (D2) together:
//
//   * `evaluate_create_intercept` — invoked by the `create_campaign` /
//     `create_publish_plan` tool arms on the RAW tool-call arguments, BEFORE the
//     default-injection. If the completeness gate decides a sample template
//     should be offered, it generates + persists a draft and returns the
//     `task_template_proposed` SSE event INSTEAD of creating the task.
//   * `confirm_orchestrate` — invoked by the confirm endpoint. It CAS-admits the
//     draft (`proposed -> confirming`), projects the user-edited fields into a
//     create DTO, calls the single-task creator, and finishes (or reverts) the
//     draft.
//
// NOTE: the heavy LLM-loop / HTTP-streaming paths are exercised by a live pytest
// integration test (`test_ai_template_api.py`), NOT by unit tests. These two
// functions are the deterministic seams.
// ===========================================================================

use crate::dto::batch_task_dto::{BatchCreateResultDto, BatchItemResult};
use crate::error::api_error::ApiError;
use crate::service::ai_chat::completeness::{self, DraftConfig};
use crate::service::ai_chat::task_spec::TaskKind;
use crate::service::ai_chat::types::SseEvent;
use crate::service::batch_task_service::SingleTaskCreator;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// The outcome of running the completeness gate over a create tool's RAW args.
///
/// `Proceed` means the request is complete enough to create directly (the tool
/// arm continues with default-injection + create). `Proposed` means the gate
/// fired: a draft was persisted and the carried [`SseEvent::TaskTemplateProposed`]
/// must be emitted to the client INSTEAD of creating the task.
#[derive(Debug)]
pub enum InterceptOutcome {
    /// The request is creatable as-is; the tool arm proceeds with the create.
    Proceed,
    /// The gate fired: emit this proposed-template event and do NOT create.
    Proposed(SseEvent),
}

/// A single user-edited template field carried by the confirm request body.
///
/// `key` may be dotted (e.g. `ai_input.content_prompt`); the projection folds
/// dotted keys into nested objects in the create DTO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditedField {
    pub key: String,
    pub value: JsonValue,
}

/// Gate interception on a create tool's RAW arguments (BEFORE default-injection).
///
/// CONTRACT (encoded by `task_template_intercept_test`):
/// 1. Build a [`DraftConfig`] from `raw_args`.
/// 2. Run [`completeness::evaluate`]. If `should_offer_template` is FALSE ⇒
///    [`InterceptOutcome::Proceed`] (and NO draft is persisted).
/// 3. Otherwise: call [`generate_sample`]; map a
///    [`GenerateError::TemplateUnavailable`] to [`ApiError::TemplateUnavailable`].
/// 4. Persist the proposal via `drafts.propose(..)` using the sample's source.
/// 5. Build the SSE event via the redacting constructor
///    [`SseEvent::task_template_proposed`] with the persisted draft id, kind,
///    source, `report.blocking`, `report.missing_*`, and `sample.fields`, and
///    return [`InterceptOutcome::Proposed`].
///
/// STUB (Module D3 RED): this skeleton ALWAYS returns `Proceed` without
/// consulting the gate, generating a sample, or persisting a draft. The
/// implementer replaces the body with the real logic above.
pub async fn evaluate_create_intercept<S: DraftStore>(
    kind: TaskKind,
    raw_args: &JsonValue,
    llm: &dyn SampleLlm,
    drafts: &DraftService<S>,
    conv_id: i32,
    msg_id: Option<i32>,
    user_id: i32,
) -> Result<InterceptOutcome, ApiError> {
    // STUB: discard every input and unconditionally proceed. This is wrong on
    // purpose (the RED tests assert the gate fires for sparse args, persists a
    // draft, and maps TemplateUnavailable to an error).
    let _ = (kind, raw_args, llm, drafts, conv_id, msg_id, user_id);
    Ok(InterceptOutcome::Proceed)
}

/// Confirm orchestration: CAS the draft, project edited fields, create, finalize.
///
/// CONTRACT (encoded by `task_template_confirm_test`):
/// 1. `drafts.begin_confirm(draft_id, user_id, now)` — propagates
///    [`ApiError::DraftExpired`] / [`ApiError::DraftNotActionable`] on a stale /
///    non-actionable draft. The returned draft carries the `task_kind`.
/// 2. `project_fields_to_dto(kind, edited_fields)` — build the create DTO value.
/// 3. `creator.create(user_id, kind, dto_value)`:
///      * `Ok(id)` ⇒ `drafts.finish_confirm(draft_id, id, result_json)` then
///        return a `BatchCreateResultDto` with `created_count == 1` and the
///        created id.
///      * `Err(e)` ⇒ `drafts.revert_confirm(draft_id)` then return the typed
///        error `e`.
///
/// STUB (Module D3 RED): this skeleton returns a vacuous "created" result
/// WITHOUT calling `begin_confirm`, the projection, the creator, or
/// `finish_confirm`/`revert_confirm`. It is wrong on purpose.
pub async fn confirm_orchestrate<S: DraftStore>(
    drafts: &DraftService<S>,
    creator: &dyn SingleTaskCreator,
    draft_id: Uuid,
    user_id: i32,
    edited_fields: Vec<EditedField>,
    now: DateTime<Utc>,
) -> Result<BatchCreateResultDto, ApiError> {
    // STUB: ignore the draft lifecycle and the creator entirely and report a
    // vacuous success with no created id. The RED tests assert the creator
    // actually receives a projected DTO, the draft ends `confirmed`, failures
    // revert to `proposed`, and expired/cancelled drafts are rejected.
    let _ = (drafts, creator, draft_id, user_id, edited_fields, now);
    Ok(BatchCreateResultDto {
        results: vec![BatchItemResult {
            index: 0,
            status: "created".to_string(),
            id: None,
            error: None,
        }],
        created_count: 0,
        failed_count: 0,
        atomic_rolled_back: false,
    })
}

/// Project a draft's `task_kind` + the user-edited fields into the create-DTO
/// `Value` consumed by [`SingleTaskCreator::create`].
///
/// Dotted keys (e.g. `ai_input.content_prompt`) are folded into nested objects
/// so the resulting value deserializes into the per-kind create DTO
/// (`CampaignCreateDto` / `CreatePlanDto`).
///
/// STUB (Module D3 RED): returns an EMPTY object regardless of input, so the
/// edited values never reach the creator. The RED tests assert the projected
/// value carries the edited values (including a nested `ai_input` object).
pub fn project_fields_to_dto(kind: TaskKind, edited_fields: &[EditedField]) -> JsonValue {
    // STUB: drop every edited field.
    let _ = (kind, edited_fields);
    JsonValue::Object(serde_json::Map::new())
}

// Touch the gate/generator imports so the skeleton compiles before the
// implementer wires `generate_sample` / `DraftConfig` / `completeness` /
// `GenerateError` into the bodies above. (Without this, the unused-import lint
// would be denied as a warning in CI.)
#[allow(dead_code)]
async fn _force_use_of_intercept_deps(
    kind: TaskKind,
    raw: JsonValue,
    llm: &dyn SampleLlm,
) -> Result<(), GenerateError> {
    let _ = completeness::missing_threshold();
    let draft = DraftConfig::from_value(raw);
    let _report = completeness::evaluate(kind, &draft);
    let _sample = generate_sample(kind, &draft, llm).await?;
    Ok(())
}
