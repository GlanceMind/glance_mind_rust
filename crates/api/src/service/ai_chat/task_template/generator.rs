//! Sample-template generator core (Module B1).
//!
//! Given a [`TaskKind`] and a partial [`DraftConfig`], produce a fully-populated
//! [`SampleTemplate`] that always passes [`super::validate::against_spec`]. The
//! generator first resolves a deterministic preset (see [`super::presets`]), then
//! optionally asks an [`SampleLlm`] to flesh it out, merging values with the
//! precedence `user_draft > llm > preset`. On any LLM failure (or an LLM result
//! that cannot be repaired into a valid config) it falls back to the preset, so
//! the only error it can ever return is [`GenerateError::TemplateUnavailable`]
//! (when no preset exists for a genuinely-unsupported combo).
//!
//! Secrets (API keys) are NEVER passed through this API: they live entirely
//! inside the [`SampleLlm`] implementation / its environment.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::completeness::DraftConfig;
use super::super::task_spec::{FieldKind, Importance, TaskConfigSpec, TaskKind};
use super::presets;
use super::validate::against_spec;

/// Where the values in a [`SampleTemplate`] ultimately came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SampleSource {
    /// Every value originated from the LLM completion.
    AiGenerated,
    /// Every value originated from the deterministic preset.
    Preset,
    /// Values were drawn from two or more of draft / llm / preset.
    Hybrid,
}

/// A selectable option for an enum-typed [`SampleField`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumOption {
    pub value: serde_json::Value,
    pub label: String,
}

/// A single rendered field in a [`SampleTemplate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleField {
    pub key: String,
    pub label_cn: String,
    pub group: String,
    pub importance: Importance,
    #[serde(rename = "type")]
    pub field_type: FieldKind,
    pub value: serde_json::Value,
    pub editable: bool,
    pub options: Option<Vec<EnumOption>>,
}

/// A fully-rendered sample template for a task kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleTemplate {
    pub task_kind: TaskKind,
    pub source: SampleSource,
    pub fields: Vec<SampleField>,
}

/// Why an LLM completion call failed.
#[derive(Debug)]
pub enum LlmFailure {
    /// The provider returned an empty body / no content.
    Empty,
    /// The provider returned content that was not parseable JSON.
    Malformed(String),
    /// The provider returned a non-success HTTP status.
    Upstream(u16),
    /// The provider rate-limited the request (HTTP 429 / equivalent).
    RateLimit,
    /// The call timed out.
    Timeout,
    /// Any other transport / provider failure.
    Other(String),
}

/// The only error [`generate_sample`] can return.
#[derive(Debug, PartialEq, Eq)]
pub enum GenerateError {
    /// No preset exists for the requested (kind, draft) combo.
    TemplateUnavailable,
}

/// Abstraction over the JSON-completion LLM the generator calls. Implementations
/// own their own credentials; no key is ever threaded through this trait.
#[async_trait::async_trait]
pub trait SampleLlm: Send + Sync {
    /// Ask the model to produce a JSON object given a system + user prompt.
    async fn complete_json(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<serde_json::Value, LlmFailure>;
}

/// Number of tokens requested from the LLM for a sample-template completion.
const MAX_TOKENS: u32 = 800;

/// Where a single merged field value originated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provenance {
    Draft,
    Llm,
    Preset,
}

/// Generate a fully-populated, spec-valid sample template for `kind`.
///
/// Contract (see the module docs):
/// 1. Resolve a deterministic preset; if none exists ⇒
///    [`GenerateError::TemplateUnavailable`] (before any LLM call).
/// 2. Build a `system` + `user` prompt (the system prompt names "json") and call
///    the LLM. Merge values with precedence `user_draft > llm > preset` and
///    validate the `draft+llm` portion via [`against_spec`].
/// 3. On an invalid first completion, do exactly ONE repair re-call with the
///    validation errors appended; on a `RateLimit` error, do exactly ONE retry.
/// 4. Any unrecoverable outcome falls back to the preset. The returned template
///    ALWAYS passes [`against_spec`], and the only error is `TemplateUnavailable`.
pub async fn generate_sample(
    kind: TaskKind,
    draft: &DraftConfig,
    llm: &dyn SampleLlm,
) -> Result<SampleTemplate, GenerateError> {
    let preset = match presets::resolve(kind, draft) {
        Some(p) => p,
        None => return Err(GenerateError::TemplateUnavailable),
    };

    let spec = TaskConfigSpec::for_kind(kind, draft);
    let preset_values = template_value_map(&preset);

    let system = build_system_prompt(kind, &spec);
    let user = build_user_prompt(kind, draft, &preset);

    // --- First completion (with one bounded RateLimit retry). ---
    let first = match llm.complete_json(&system, &user, MAX_TOKENS).await {
        Ok(json) => Some(json),
        Err(LlmFailure::RateLimit) => {
            // Exactly one bounded retry on rate-limit.
            llm.complete_json(&system, &user, MAX_TOKENS).await.ok()
        }
        // Empty / Malformed / Upstream / Timeout / Other ⇒ preset fallback.
        Err(_) => None,
    };

    let Some(first_json) = first else {
        return Ok(preset);
    };

    // Validate the draft+llm portion (NOT preset-backfilled): the preset is a
    // fallback, never a silent backfill that masks an invalid completion.
    if let Some(template) = try_build_from_llm(kind, &spec, draft, &first_json, &preset_values) {
        return Ok(template);
    }

    // --- One repair re-call, seeding the errors back into the prompt. ---
    let errors = validation_errors(kind, draft, &first_json);
    let repair_user = format!(
        "{user}\n\nYour previous JSON failed validation with these errors:\n{}\n\
         Return a corrected JSON object that fixes every error above.",
        errors.join("\n")
    );

    if let Ok(repaired) = llm.complete_json(&system, &repair_user, MAX_TOKENS).await {
        if let Some(template) = try_build_from_llm(kind, &spec, draft, &repaired, &preset_values) {
            return Ok(template);
        }
    }

    // Repair failed (or errored) ⇒ preset fallback.
    Ok(preset)
}

/// Validate the `draft > llm` merge of the Required keys (no preset backfill).
/// Returns `Ok(())` if every Required key is satisfied by the draft or the LLM
/// JSON alone, else the per-key error messages.
fn validate_draft_llm(
    kind: TaskKind,
    draft: &DraftConfig,
    llm_json: &Value,
) -> Result<(), Vec<String>> {
    let spec = TaskConfigSpec::for_kind(kind, draft);
    let mut config = serde_json::Map::new();
    for f in spec.fields.iter() {
        if !matches!(f.importance, Importance::Required) {
            continue;
        }
        if let Some(v) = merged_value(f.key, draft, llm_json, None) {
            insert_dotted(&mut config, f.key, v);
        }
    }
    against_spec(&Value::Object(config), kind)
}

/// Convenience: the validation errors for the `draft > llm` merge.
fn validation_errors(kind: TaskKind, draft: &DraftConfig, llm_json: &Value) -> Vec<String> {
    validate_draft_llm(kind, draft, llm_json)
        .err()
        .unwrap_or_default()
}

/// Attempt to build an AI/Hybrid template from an LLM completion.
///
/// Validates the `draft > llm` Required merge; on success, renders the full
/// template (Required + Recommended) with precedence `draft > llm > preset` and
/// the appropriate [`SampleSource`]. Returns `None` if the completion does not
/// validate (caller then repairs / falls back).
fn try_build_from_llm(
    kind: TaskKind,
    spec: &TaskConfigSpec,
    draft: &DraftConfig,
    llm_json: &Value,
    preset_values: &BTreeMap<String, Value>,
) -> Option<SampleTemplate> {
    if validate_draft_llm(kind, draft, llm_json).is_err() {
        return None;
    }

    let mut fields = Vec::new();
    let mut provenances = Vec::new();
    for f in spec.fields.iter() {
        if matches!(f.importance, Importance::Optional) {
            continue;
        }
        let Some((value, prov)) = merged_value_with_source(f.key, draft, llm_json, preset_values)
        else {
            continue;
        };
        provenances.push(prov);
        fields.push(presets::sample_field_from_spec(f, value));
    }

    // AiGenerated iff every emitted value came from the LLM alone; otherwise the
    // values were drawn from >= 2 of draft / llm / preset ⇒ Hybrid.
    let source = if provenances.iter().all(|p| *p == Provenance::Llm) {
        SampleSource::AiGenerated
    } else {
        SampleSource::Hybrid
    };

    Some(SampleTemplate {
        task_kind: kind,
        source,
        fields,
    })
}

/// Resolve a key's value under precedence `draft > llm > preset(optional)`.
fn merged_value(
    key: &str,
    draft: &DraftConfig,
    llm_json: &Value,
    preset_values: Option<&BTreeMap<String, Value>>,
) -> Option<Value> {
    merged_value_with_source(key, draft, llm_json, preset_values.unwrap_or(&EMPTY_PRESET))
        .map(|(v, _)| v)
}

/// Resolve a key's value AND its [`Provenance`] under `draft > llm > preset`.
fn merged_value_with_source(
    key: &str,
    draft: &DraftConfig,
    llm_json: &Value,
    preset_values: &BTreeMap<String, Value>,
) -> Option<(Value, Provenance)> {
    if let Some(v) = draft_value(draft, key) {
        return Some((v, Provenance::Draft));
    }
    if let Some(v) = resolve_dotted(llm_json, key) {
        if !v.is_null() {
            return Some((v.clone(), Provenance::Llm));
        }
    }
    preset_values
        .get(key)
        .cloned()
        .map(|v| (v, Provenance::Preset))
}

/// A draft lookup that walks dotted keys and rejects null / blank-string values.
fn draft_value(draft: &DraftConfig, key: &str) -> Option<Value> {
    let raw = match key.split_once('.') {
        None => draft.get(key)?,
        Some((head, rest)) => {
            let mut current = draft.get(head)?;
            for segment in rest.split('.') {
                current = current.as_object()?.get(segment)?;
            }
            current
        }
    };
    if raw.is_null() {
        return None;
    }
    if let Some(s) = raw.as_str() {
        if s.trim().is_empty() {
            return None;
        }
    }
    Some(raw.clone())
}

/// Resolve a (possibly dotted) key inside an arbitrary JSON value.
fn resolve_dotted<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in key.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// Insert a value under a (possibly dotted) key into a flat config map, creating
/// nested objects as needed.
fn insert_dotted(root: &mut serde_json::Map<String, Value>, key: &str, value: Value) {
    match key.split_once('.') {
        None => {
            root.insert(key.to_string(), value);
        }
        Some((head, rest)) => {
            let entry = root
                .entry(head.to_string())
                .or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Value::Object(obj) = entry {
                insert_dotted(obj, rest, value);
            }
        }
    }
}

/// Flatten a rendered template's fields into a `key -> value` map (keys kept
/// dotted) for use as preset backfill values.
fn template_value_map(template: &SampleTemplate) -> BTreeMap<String, Value> {
    template
        .fields
        .iter()
        .map(|f| (f.key.clone(), f.value.clone()))
        .collect()
}

/// Build the system prompt. MUST contain the literal word "json" (DeepSeek
/// JSON-mode requirement) and describe the expected field schema.
fn build_system_prompt(kind: TaskKind, spec: &TaskConfigSpec) -> String {
    let kind_label = match kind {
        TaskKind::Campaign => "social-monitor campaign",
        TaskKind::PublishPlan => "AI publish plan",
    };
    let mut s = String::new();
    s.push_str(&format!(
        "You configure a {kind_label}. Respond ONLY with a single JSON object \
         whose keys are the field keys below. Use the correct JSON type for each \
         field (Int=number, String/Enum=string, Bool=boolean, Json=object).\n\n\
         Fields:\n"
    ));
    for f in spec.fields.iter() {
        if matches!(f.importance, Importance::Optional) {
            continue;
        }
        s.push_str(&format!(
            "- {} ({:?}, {:?}): {}\n",
            f.key, f.kind, f.importance, f.label_cn
        ));
    }
    s.push_str("\nReturn a valid JSON object only — no prose, no markdown fences.");
    s
}

/// Build the user prompt, seeded from the draft + the preset example.
fn build_user_prompt(kind: TaskKind, draft: &DraftConfig, preset: &SampleTemplate) -> String {
    let draft_json = Value::Object(draft.0.clone());
    let example = template_to_config_value(preset);
    let kind_label = match kind {
        TaskKind::Campaign => "campaign",
        TaskKind::PublishPlan => "publish plan",
    };
    format!(
        "Fill in a complete {kind_label} configuration as JSON.\n\n\
         The user has already provided these values (keep them verbatim):\n{draft_json}\n\n\
         Here is a realistic example to model your answer on:\n{example}\n\n\
         Produce the final JSON object now."
    )
}

/// Flatten a template's fields into a nested config `Value` (for prompt examples).
fn template_to_config_value(template: &SampleTemplate) -> Value {
    let mut root = serde_json::Map::new();
    for f in &template.fields {
        insert_dotted(&mut root, &f.key, f.value.clone());
    }
    Value::Object(root)
}

/// An empty preset-value map, used when callers do not want preset backfill.
static EMPTY_PRESET: std::sync::LazyLock<BTreeMap<String, Value>> =
    std::sync::LazyLock::new(BTreeMap::new);
