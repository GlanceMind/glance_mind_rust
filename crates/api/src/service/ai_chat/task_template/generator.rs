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

use serde::{Deserialize, Serialize};

use super::super::completeness::DraftConfig;
use super::super::task_spec::{FieldKind, Importance, TaskKind};

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

/// Generate a fully-populated, spec-valid sample template for `kind`.
///
/// See the module docs for the full merge/fallback contract. This is a
/// deliberately-WRONG skeleton stub: it ignores the LLM, never resolves a
/// preset, and returns an empty AI-sourced template so the RED tests fail on
/// assertions rather than compilation.
pub async fn generate_sample(
    kind: TaskKind,
    _draft: &DraftConfig,
    _llm: &dyn SampleLlm,
) -> Result<SampleTemplate, GenerateError> {
    // WRONG STUB: does not call the LLM, does not resolve a preset, produces an
    // empty field set, and never errors. The real implementation must honor the
    // resolve→prompt→complete→merge→validate→repair→fallback contract.
    Ok(SampleTemplate {
        task_kind: kind,
        source: SampleSource::AiGenerated,
        fields: Vec::new(),
    })
}
