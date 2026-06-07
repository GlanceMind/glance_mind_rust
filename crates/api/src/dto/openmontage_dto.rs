//! OpenMontage DTOs
//!
//! Request/response DTOs for the OpenMontage Professional Video API facade.
//! All DTOs use snake_case JSON serialization (serde default).

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Input mode enum for OpenMontage pipelines.
/// Strictly matches the 6 supported modes from the frontend contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    TextToVideo,
    SourceScript,
    ImageToVideo,
    FirstLastFrame,
    ReferenceDriven,
    SourceClip,
}

// Secret material tokens that should never appear in user input
const SECRET_TOKENS: &[&str] = &[
    "api_key",
    "apikey",
    "secret",
    "password",
    "credential",
    "sk-",
    "xai-",
    "fal-",
    "xi_",
    // M0b-T5: Broadened provider secret prefixes
    "ghp_",        // GitHub personal access token
    "github_pat_", // GitHub fine-grained PAT
    "AKIA",        // AWS access key
    "ASIA",        // AWS session token
    "AIza",        // Google Cloud API key
    "hf_",         // HuggingFace token
    "xoxb-",       // Slack bot token
    "Bearer ",     // Bearer auth header (note trailing space)
];

/// Validate that the input_mode is allowed for the given pipeline and that all required asset roles are present.
/// Returns Ok if valid, Err with a message if invalid.
///
/// Per-pipeline contract (from OpenMontage frontend):
/// - animated-explainer: [text_to_video], required: []
/// - animation: [text_to_video], required: []
/// - avatar-spokesperson: [source_script, text_to_video], required: [avatar]
/// - cinematic: [source_clip, reference_driven, text_to_video], required: []
/// - screen-demo: [text_to_video, source_clip], required: []
/// - hybrid: [source_clip], required: [source_video]
pub fn validate_input_mode_for_pipeline(
    pipeline: &str,
    mode: InputMode,
    asset_roles: &[String],
) -> Result<(), String> {
    // Define the contract table
    let contract = match pipeline {
        "animated-explainer" => (vec![InputMode::TextToVideo], vec![]),
        "animation" => (vec![InputMode::TextToVideo], vec![]),
        "avatar-spokesperson" => (
            vec![InputMode::SourceScript, InputMode::TextToVideo],
            vec!["avatar"],
        ),
        "cinematic" => (
            vec![
                InputMode::SourceClip,
                InputMode::ReferenceDriven,
                InputMode::TextToVideo,
            ],
            vec![],
        ),
        "screen-demo" => (vec![InputMode::TextToVideo, InputMode::SourceClip], vec![]),
        "hybrid" => (vec![InputMode::SourceClip], vec!["source_video"]),
        _ => {
            return Err(format!("Unknown pipeline: {}", pipeline));
        }
    };

    let (allowed_modes, required_roles) = contract;

    // Check if mode is allowed
    if !allowed_modes.contains(&mode) {
        return Err(format!(
            "Input mode {:?} is not allowed for pipeline '{}'",
            mode, pipeline
        ));
    }

    // Check if all required roles are present
    for required_role in required_roles {
        if !asset_roles.iter().any(|r| r == required_role) {
            return Err(format!(
                "Pipeline '{}' with input mode {:?} requires asset role '{}', but it is missing",
                pipeline, mode, required_role
            ));
        }
    }

    Ok(())
}

/// Server-side context injected when converting DTO to protocol request
#[derive(Debug, Clone)]
pub struct ServerContext {
    pub job_id: String,
    pub user_id: i32,
    pub tenant_id: String,
    pub callback_secret_ref: Option<String>,
}

/// Create Job Request DTO
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateJobDto {
    pub title: String,
    pub prompt: String,
    pub target_platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_playbook: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_runtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_limit_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider_slots: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub asset_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_invocations: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub metadata: JsonValue,
    // M0b-T6: Cross-tier contract fields (frontend → Rust → engine)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_selection: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_json: Option<JsonValue>,
}

impl CreateJobDto {
    /// Recursively validate that no secret material appears in any field.
    /// Returns Err with the offending token if found.
    pub fn validate_no_secret_material(&self) -> Result<(), String> {
        let mut fields: Vec<(&str, String)> = vec![
            ("title", self.title.clone()),
            ("prompt", self.prompt.clone()),
            ("target_platform", self.target_platform.clone()),
        ];

        if let Some(ref lang) = self.language {
            fields.push(("language", lang.clone()));
        }
        if let Some(ref ar) = self.aspect_ratio {
            fields.push(("aspect_ratio", ar.clone()));
        }
        if let Some(ref im) = self.input_mode {
            fields.push(("input_mode", im.clone()));
        }
        if let Some(ref p) = self.pipeline {
            fields.push(("pipeline", p.clone()));
        }
        if let Some(ref sp) = self.style_playbook {
            fields.push(("style_playbook", sp.clone()));
        }
        if let Some(ref rr) = self.render_runtime {
            fields.push(("render_runtime", rr.clone()));
        }
        if let Some(ref qt) = self.quality_tier {
            fields.push(("quality_tier", qt.clone()));
        }
        if let Some(ref ap) = self.approval_policy {
            fields.push(("approval_policy", ap.clone()));
        }
        // M0b-T6: Include new cross-tier string fields in secret scan
        if let Some(ref ss) = self.source_script {
            fields.push(("source_script", ss.clone()));
        }
        if let Some(ref vs) = self.voice_selection {
            fields.push(("voice_selection", vs.clone()));
        }
        if let Some(ref pm) = self.production_mode {
            fields.push(("production_mode", pm.clone()));
        }
        if let Some(ref aud) = self.audience {
            fields.push(("audience", aud.clone()));
        }
        if let Some(ref obj) = self.objective {
            fields.push(("objective", obj.clone()));
        }

        // Check string fields
        for (field_name, value) in fields {
            for token in SECRET_TOKENS {
                if value.to_lowercase().contains(&token.to_lowercase()) {
                    return Err(format!(
                        "Field '{}' contains forbidden token: {}",
                        field_name, token
                    ));
                }
            }
        }

        // Check metadata JSON recursively
        if !self.metadata.is_null() {
            check_json_for_secrets(&self.metadata, "metadata")?;
        }

        // Check provider_slots keys/values
        if let Some(ref slots) = self.provider_slots {
            for (k, v) in slots {
                for token in SECRET_TOKENS {
                    let token_lower = token.to_lowercase();
                    if k.to_lowercase().contains(&token_lower)
                        || v.to_lowercase().contains(&token_lower)
                    {
                        return Err(format!(
                            "provider_slots contains forbidden token: {}",
                            token
                        ));
                    }
                }
            }
        }

        // Check tool_invocations JSON
        if let Some(ref ti) = self.tool_invocations {
            check_json_for_secrets(ti, "tool_invocations")?;
        }

        // M0b-T6: Check brand_json for secrets (esp. nested API keys)
        if let Some(ref bj) = self.brand_json {
            check_json_for_secrets(bj, "brand_json")?;
        }

        Ok(())
    }

    /// Validate budget_limit_usd is finite and non-negative.
    /// Returns Err if budget is NaN, infinite, or negative.
    pub fn validate_budget(&self) -> Result<(), String> {
        if let Some(budget) = self.budget_limit_usd {
            if !budget.is_finite() {
                return Err(
                    "budget_limit_usd must be a finite number (not NaN or infinity)".to_string(),
                );
            }
            if budget < 0.0 {
                return Err("budget_limit_usd must be non-negative".to_string());
            }
        }
        Ok(())
    }

    /// Convert DTO to OpenMontageProfessionalVideoRequest (protocol message).
    /// M1: maps text fields; asset_ids/input_mode→assets is deferred to part 3.
    /// Returns JSON Value for now (protocol struct will be used in part 3).
    pub fn to_protocol_request(&self, server_ctx: ServerContext) -> JsonValue {
        let mut req = serde_json::json!({
            "version": "v1",
            "request_id": uuid::Uuid::new_v4().to_string(),
            "idempotency_key": uuid::Uuid::new_v4().to_string(),
            "tenant_id": server_ctx.tenant_id,
            "user_id": server_ctx.user_id.to_string(),
            "title": self.title,
            "prompt": self.prompt,
            "target_platform": self.target_platform,
            "language": self.language.clone().unwrap_or_else(|| "en".to_string()),
            "duration_seconds": self.duration_seconds.unwrap_or(60),
            "aspect_ratio": self.aspect_ratio.clone().unwrap_or_else(|| "16:9".to_string()),
            "pipeline": self.pipeline.clone().unwrap_or_else(|| "animated-explainer".to_string()),
            "quality_tier": self.quality_tier.clone().unwrap_or_else(|| "standard".to_string()),
            "approval_policy": self.approval_policy.clone().unwrap_or_else(|| "auto".to_string()),
            "budget_limit_usd": self.budget_limit_usd.unwrap_or(10.0),
            "provider_preferences": self.provider_slots.clone().unwrap_or_default(),
            "metadata_json": self.metadata.to_string(),
        });

        // Optional fields
        if let Some(ref sp) = self.style_playbook {
            req["style_playbook"] = JsonValue::String(sp.clone());
        }
        if let Some(ref rr) = self.render_runtime {
            req["render_runtime"] = JsonValue::String(rr.clone());
        }
        if let Some(ref im) = self.input_mode {
            req["input_mode"] = JsonValue::String(im.clone());
        }

        // M0b-T6: Emit cross-tier contract fields into worker request_json
        if let Some(ref ss) = self.source_script {
            req["source_script"] = JsonValue::String(ss.clone());
        }
        if let Some(ref vs) = self.voice_selection {
            req["voice_selection"] = JsonValue::String(vs.clone());
        }
        if let Some(ref pm) = self.production_mode {
            req["production_mode"] = JsonValue::String(pm.clone());
        }
        if let Some(ref aud) = self.audience {
            req["audience"] = JsonValue::String(aud.clone());
        }
        if let Some(ref obj) = self.objective {
            req["objective"] = JsonValue::String(obj.clone());
        }
        if let Some(ref bj) = self.brand_json {
            req["brand_json"] = bj.clone();
        }

        // Server-only fields
        if let Some(ref cb_secret) = server_ctx.callback_secret_ref {
            req["callback"] = serde_json::json!({
                "callback_secret_ref": cb_secret,
            });
        }

        // M1: asset_ids and tool_invocations deferred to part 3
        // TODO(part-3): map asset_ids to OpenMontageInputAsset[], tool_invocations to OpenMontageToolInvocation[]

        req
    }
}

fn check_json_for_secrets(value: &JsonValue, context: &str) -> Result<(), String> {
    match value {
        JsonValue::String(s) => {
            for token in SECRET_TOKENS {
                if s.to_lowercase().contains(&token.to_lowercase()) {
                    return Err(format!("{} contains forbidden token: {}", context, token));
                }
            }
        }
        JsonValue::Object(map) => {
            for (k, v) in map {
                for token in SECRET_TOKENS {
                    if k.to_lowercase().contains(&token.to_lowercase()) {
                        return Err(format!(
                            "{}.{} key contains forbidden token: {}",
                            context, k, token
                        ));
                    }
                }
                check_json_for_secrets(v, &format!("{}.{}", context, k))?;
            }
        }
        JsonValue::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                check_json_for_secrets(v, &format!("{}[{}]", context, i))?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Job Snapshot DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshotDto {
    pub job_id: String,
    pub project_id: String,
    pub status: String,
    pub pipeline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<String>,
    pub progress_pct: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_json: Option<JsonValue>,
    pub last_event_sequence: i64,
    pub next_event_sequence: i64,
    pub sync_required: bool,
    pub snapshot_json: JsonValue,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// Job Event DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEventDto {
    pub sequence: i64,
    pub event_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_pct: Option<i32>,
    pub event_json: JsonValue,
    pub emitted_at: String,
}

/// Job Events List DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEventsDto {
    pub events: Vec<JobEventDto>,
    pub next_sequence: u64,
}

/// Preflight DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightDto {
    pub passed: bool,
    pub status: String,
    #[serde(default)]
    pub blocking: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost_cents: Option<i64>,
}

/// Pipelines DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinesDto {
    pub pipelines: Vec<PipelineInfoDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInfoDto {
    pub name: String,
    pub description: String,
    pub stability: String,
}

/// Approval Decision DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalDto {
    pub approval_id: String,
    pub decision: String, // "approve" | "reject" | "revise"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_json: Option<JsonValue>,
}

/// Asset DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDto {
    pub asset_id: String,
    pub kind: String,
    pub role: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_px: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_px: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i32>,
}

/// Cancel Result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResultDto {
    pub job_id: String,
    pub cancel_requested: bool,
    pub message: String,
}

// ============================================================================
// M0b-T5: Public secret scanning and redaction utilities
// ============================================================================

/// Recursively scan a JSON value for secret-shaped tokens.
/// Returns Err if any secret is found, Ok otherwise.
pub fn scan_json_for_secrets(value: &JsonValue, context: &str) -> Result<(), String> {
    check_json_for_secrets(value, context)
}

/// Recursively redact secret-shaped tokens in a JSON value.
/// Returns a new JSON value with secrets replaced by "***REDACTED***".
pub fn redact_secrets_in_json(value: &JsonValue) -> JsonValue {
    match value {
        JsonValue::String(s) => {
            // Check if this string contains any secret token
            for token in SECRET_TOKENS {
                if s.to_lowercase().contains(&token.to_lowercase()) {
                    return JsonValue::String("***REDACTED***".to_string());
                }
            }
            value.clone()
        }
        JsonValue::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                // Check if the key itself contains a secret token
                let mut key_has_secret = false;
                for token in SECRET_TOKENS {
                    if k.to_lowercase().contains(&token.to_lowercase()) {
                        key_has_secret = true;
                        break;
                    }
                }
                if key_has_secret {
                    new_map.insert(k.clone(), JsonValue::String("***REDACTED***".to_string()));
                } else {
                    new_map.insert(k.clone(), redact_secrets_in_json(v));
                }
            }
            JsonValue::Object(new_map)
        }
        JsonValue::Array(arr) => {
            let new_arr: Vec<JsonValue> = arr.iter().map(redact_secrets_in_json).collect();
            JsonValue::Array(new_arr)
        }
        _ => value.clone(),
    }
}
