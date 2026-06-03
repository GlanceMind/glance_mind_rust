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
///    source, `report.blocking`, the missing keys, and `sample.fields`, and
///    return [`InterceptOutcome::Proposed`].
pub async fn evaluate_create_intercept<S: DraftStore>(
    kind: TaskKind,
    raw_args: &JsonValue,
    llm: &dyn SampleLlm,
    drafts: &DraftService<S>,
    conv_id: i32,
    msg_id: Option<i32>,
    user_id: i32,
) -> Result<InterceptOutcome, ApiError> {
    // (1) Build the draft from the RAW tool-call args (BEFORE any default
    // injection by the tool arm). Evaluating the raw args is load-bearing: the
    // `injected_defaults_do_not_mask_missing` test proves the gate must see what
    // the user actually supplied, not the post-injection shape.
    let draft = DraftConfig::from_value(raw_args.clone());

    // (2) Completeness gate. If the request is complete enough to create as-is,
    // proceed WITHOUT persisting anything.
    let report = completeness::evaluate(kind, &draft);
    if !report.should_offer_template {
        return Ok(InterceptOutcome::Proceed);
    }

    // (3) The gate fired: generate a spec-valid sample template. A genuinely
    // unsupported (kind, draft) combo has no preset ⇒ `TemplateUnavailable`,
    // which we surface as a typed `ApiError` (NOT Proceed, NOT Proposed). No
    // draft is persisted on this path.
    let sample = match generate_sample(kind, &draft, llm).await {
        Ok(s) => s,
        Err(GenerateError::TemplateUnavailable) => return Err(ApiError::TemplateUnavailable),
    };

    // (4) Persist the proposal (`proposed`), superseding any prior live proposal
    // for this conversation+kind. The stored `draft_config` is the RAW args; the
    // `sample_source` is the generator's provenance as a wire string.
    let persisted = drafts
        .propose(
            conv_id,
            msg_id,
            user_id,
            kind,
            raw_args.clone(),
            sample_source_wire_str(sample.source),
        )
        .await?;

    // (5) Build the proposed-template SSE event via the redacting constructor.
    // `missing` is the union of the still-missing required + recommended keys.
    let missing = merge_missing(&report);
    let event = SseEvent::task_template_proposed(
        persisted.id,
        kind,
        sample.source,
        report.blocking,
        missing,
        sample.fields,
    );
    Ok(InterceptOutcome::Proposed(event))
}

/// The canonical wire string for a [`SampleSource`] (matches its `snake_case`
/// serde representation and the `sample_source` column values).
fn sample_source_wire_str(source: SampleSource) -> &'static str {
    match source {
        SampleSource::AiGenerated => "ai_generated",
        SampleSource::Preset => "preset",
        SampleSource::Hybrid => "hybrid",
    }
}

/// The union of the still-missing keys reported by the completeness gate
/// (required first, then recommended), used to populate the proposed event's
/// `missing` list so the UI can flag every unfilled field.
fn merge_missing(report: &completeness::CompletenessReport) -> Vec<String> {
    let mut missing =
        Vec::with_capacity(report.missing_required.len() + report.missing_recommended.len());
    missing.extend(report.missing_required.iter().cloned());
    missing.extend(report.missing_recommended.iter().cloned());
    missing
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
/// The originating `draft_id` is also threaded into the create DTO as
/// `source_draft_id` so the created entity links back to its draft.
pub async fn confirm_orchestrate<S: DraftStore>(
    drafts: &DraftService<S>,
    creator: &dyn SingleTaskCreator,
    draft_id: Uuid,
    user_id: i32,
    edited_fields: Vec<EditedField>,
    now: DateTime<Utc>,
) -> Result<BatchCreateResultDto, ApiError> {
    // (1) CAS-admit the draft `proposed -> confirming`. This propagates
    // `DraftExpired` (stale) / `DraftNotActionable` (terminal or lost race) and
    // never invokes the creator on those paths. The returned draft carries the
    // `task_kind` we project + create against.
    let draft = drafts.begin_confirm(draft_id, user_id, now).await?;

    // (2) Project the user-edited fields into the per-kind create DTO value,
    // folding dotted keys (`ai_input.content_prompt`) into nested objects.
    let mut dto_value = project_fields_to_dto(draft.task_kind, &edited_fields);

    // Thread the draft id into the create DTO as `source_draft_id` so the
    // created entity is linked back to its originating draft
    // (`gm_campaigns.source_draft_id`, verified by the live pytest). The per-kind
    // create DTOs accept this via `#[serde(default)]`; serde ignores it on kinds
    // that do not yet persist the link.
    if let JsonValue::Object(obj) = &mut dto_value {
        obj.insert(
            "source_draft_id".to_string(),
            JsonValue::String(draft_id.to_string()),
        );
    }

    // (3) Create the underlying entity. On success finalize the draft
    // (`confirming -> confirmed`, stamping the entity id); on failure revert it
    // (`confirming -> proposed`) so the user can edit + retry, and surface the
    // creator's typed error verbatim.
    match creator.create(user_id, draft.task_kind, dto_value).await {
        Ok(created_id) => {
            let result_json = serde_json::json!({ "created_entity_id": created_id });
            drafts
                .finish_confirm(draft_id, created_id, result_json)
                .await?;
            Ok(BatchCreateResultDto {
                results: vec![BatchItemResult {
                    index: 0,
                    status: "created".to_string(),
                    id: Some(created_id),
                    error: None,
                }],
                created_count: 1,
                failed_count: 0,
                atomic_rolled_back: false,
            })
        }
        Err(e) => {
            // Best-effort revert; the original creator error is the contract
            // return (a revert failure must not mask it).
            let _ = drafts.revert_confirm(draft_id).await;
            Err(e)
        }
    }
}

/// Project a draft's `task_kind` + the user-edited fields into the create-DTO
/// `Value` consumed by [`SingleTaskCreator::create`].
///
/// Dotted keys (e.g. `ai_input.content_prompt`) are folded into nested objects
/// so the resulting value deserializes into the per-kind create DTO
/// (`CampaignCreateDto` / `CreatePlanDto`).
pub fn project_fields_to_dto(kind: TaskKind, edited_fields: &[EditedField]) -> JsonValue {
    // The kind selects the target DTO shape downstream; the projection itself is
    // kind-agnostic (fold each edited field by its — possibly dotted — key).
    let _ = kind;
    let mut root = serde_json::Map::new();
    for field in edited_fields {
        insert_dotted(&mut root, &field.key, field.value.clone());
    }
    JsonValue::Object(root)
}

/// Insert `value` under a (possibly dotted) `key` into `root`, creating nested
/// objects for each dotted segment (`ai_input.content_prompt` ⇒
/// `{"ai_input": {"content_prompt": value}}`) so the result deserializes into
/// the per-kind create DTO rather than carrying a flat dotted key.
fn insert_dotted(root: &mut serde_json::Map<String, JsonValue>, key: &str, value: JsonValue) {
    match key.split_once('.') {
        None => {
            root.insert(key.to_string(), value);
        }
        Some((head, rest)) => {
            let entry = root
                .entry(head.to_string())
                .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
            if let JsonValue::Object(obj) = entry {
                insert_dotted(obj, rest, value);
            }
        }
    }
}
