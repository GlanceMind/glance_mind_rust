//! Validation for `ai_input.image_generations[]` (V2 ImageGenerationSpec).
//!
//! Mirrors the field semantics defined in
//! `glance_mind_protocol/proto/aipub.proto` (`ImageGenerationSpec`) and
//! enforces per-provider constraints documented in
//! `docs/image-provider-param-matrix.md`.
//!
//! Called from [`AipubService::create_plan`] before the budget freeze step.

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use serde_json::Value as JsonValue;

const VALID_OUTPUT_FORMATS: &[&str] = &["jpeg", "jpg", "png", "webp"];
const VALID_PROVIDER_HINTS: &[&str] = &["flux", "seedream", "openai"];
const VALID_MODES: &[&str] = &[
    "text_to_image",
    "image_edit",
    "i2i",
    "image_to_image",
    "edit",
];
const EXTRAS_BLACKLIST: &[&str] = &[
    "api_key",
    "authorization",
    "base_url",
    "n",
    "response_format",
];

const FLUX_REF_LIMIT: usize = 2;
const SEEDREAM_REF_LIMIT: usize = 10;
const PROMPT_MAX_LEN: usize = 4000;
const COUNT_MIN: u32 = 1;
const COUNT_MAX: u32 = 10;
const DIM_MIN: u32 = 256;
const DIM_MAX: u32 = 4096;
const DIM_MULTIPLE_OF: u32 = 8;
const SAFETY_MAX: u64 = 6;
const EXTRAS_KEY_RE: &str = r"^[a-z_][a-z0-9_]{0,40}$";
const EXTRAS_VALUE_MAX_LEN: usize = 256;

/// Returns `Ok(())` when `ai_input` either has no `image_generations` field
/// (legacy V1 path) or each spec inside is well-formed.
pub fn validate(ai_input: &JsonValue) -> Result<(), ApiError> {
    let arr = match ai_input.get("image_generations") {
        Some(v) => match v.as_array() {
            Some(a) => a,
            None => return invalid("image_generations must be an array"),
        },
        None => return Ok(()),
    };
    if arr.is_empty() {
        // Empty array is treated like absence — caller falls back to legacy.
        return Ok(());
    }

    let key_re = regex::Regex::new(EXTRAS_KEY_RE).expect("static regex");

    for (i, spec) in arr.iter().enumerate() {
        let prefix = format!("image_generations[{}]", i);
        validate_one(spec, &prefix, &key_re)?;
    }
    Ok(())
}

fn validate_one(spec: &JsonValue, prefix: &str, key_re: &regex::Regex) -> Result<(), ApiError> {
    // prompts
    let prompts = spec
        .get("prompts")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| invalid_err(&format!("{}.prompts must be an array", prefix)))?;
    if prompts.is_empty() {
        return invalid(&format!(
            "{}.prompts must contain at least 1 prompt",
            prefix
        ));
    }
    for (j, p) in prompts.iter().enumerate() {
        let s = p
            .as_str()
            .ok_or_else(|| invalid_err(&format!("{}.prompts[{}] must be a string", prefix, j)))?;
        if s.trim().is_empty() {
            return invalid(&format!("{}.prompts[{}] is empty", prefix, j));
        }
        if s.len() > PROMPT_MAX_LEN {
            return invalid(&format!(
                "{}.prompts[{}] exceeds {} chars (got {})",
                prefix,
                j,
                PROMPT_MAX_LEN,
                s.len()
            ));
        }
    }

    // count
    let count = spec.get("count").and_then(JsonValue::as_u64).unwrap_or(1) as u32;
    if !(COUNT_MIN..=COUNT_MAX).contains(&count) {
        return invalid(&format!(
            "{}.count must be in [{},{}] (got {})",
            prefix, COUNT_MIN, COUNT_MAX, count
        ));
    }

    // mode
    let mode = spec.get("mode").and_then(JsonValue::as_str);
    if let Some(m) = mode {
        if !VALID_MODES.contains(&m.to_ascii_lowercase().as_str()) {
            return invalid(&format!(
                "{}.mode {:?} unknown; supported: {:?}",
                prefix, m, VALID_MODES
            ));
        }
    }

    // model + provider routing
    let model = spec.get("model").and_then(JsonValue::as_str).unwrap_or("");
    let provider_hint = spec
        .get("provider_hint")
        .and_then(JsonValue::as_str)
        .map(str::to_ascii_lowercase);
    if let Some(ref h) = provider_hint {
        if !VALID_PROVIDER_HINTS.contains(&h.as_str()) {
            return invalid(&format!(
                "{}.provider_hint {:?} unknown; supported: {:?}",
                prefix, h, VALID_PROVIDER_HINTS
            ));
        }
    }
    let provider = resolve_provider(model, provider_hint.as_deref());

    // reference_image_urls
    let refs: Vec<&str> = spec
        .get("reference_image_urls")
        .and_then(JsonValue::as_array)
        .map(|a| a.iter().filter_map(JsonValue::as_str).collect())
        .unwrap_or_default();
    let mode_lc = mode.unwrap_or("").to_ascii_lowercase();
    let edit_mode = matches!(
        mode_lc.as_str(),
        "image_edit" | "edit" | "i2i" | "image_to_image"
    ) || (!refs.is_empty() && mode.is_none());
    if edit_mode && refs.is_empty() {
        return invalid(&format!(
            "{} mode=image_edit requires reference_image_urls.len() >= 1",
            prefix
        ));
    }
    for (j, url) in refs.iter().enumerate() {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return invalid(&format!(
                "{}.reference_image_urls[{}] must start with http:// or https://",
                prefix, j
            ));
        }
    }
    let ref_limit = match provider {
        ProviderRoute::Flux => FLUX_REF_LIMIT,
        ProviderRoute::Seedream => SEEDREAM_REF_LIMIT,
        ProviderRoute::OpenAI => 0,
    };
    if refs.len() > ref_limit {
        return invalid(&format!(
            "{} provider={:?} accepts at most {} reference_image_urls (got {})",
            prefix,
            provider.as_str(),
            ref_limit,
            refs.len()
        ));
    }
    if matches!(provider, ProviderRoute::OpenAI) && !refs.is_empty() {
        return invalid(&format!(
            "{} legacy openai provider does not support reference_image_urls; \
             pick flux-kontext-* or seedream-* model",
            prefix
        ));
    }

    // size: aspect_ratio vs width/height mutual exclusion (Flux). For
    // SeeDream, aspect_ratio is informational only (provider expects pixel
    // size), but we still enforce mutual exclusion to catch misconfig early.
    let aspect_ratio = spec.get("aspect_ratio").and_then(JsonValue::as_str);
    let width_px = spec
        .get("width_px")
        .and_then(JsonValue::as_u64)
        .map(|v| v as u32)
        .filter(|&v| v != 0);
    let height_px = spec
        .get("height_px")
        .and_then(JsonValue::as_u64)
        .map(|v| v as u32)
        .filter(|&v| v != 0);
    if aspect_ratio.is_some() && (width_px.is_some() || height_px.is_some()) {
        return invalid(&format!(
            "{} aspect_ratio and width_px/height_px are mutually exclusive",
            prefix
        ));
    }
    if let Some(ar) = aspect_ratio {
        validate_aspect_ratio(ar, &provider, prefix)?;
    }
    if let (Some(w), None) | (None, Some(w)) = (width_px, height_px) {
        return invalid(&format!(
            "{} width_px and height_px must both be set or both unset (got w={:?})",
            prefix, w
        ));
    }
    if let (Some(w), Some(h)) = (width_px, height_px) {
        for (label, dim) in [("width_px", w), ("height_px", h)] {
            if !(DIM_MIN..=DIM_MAX).contains(&dim) {
                return invalid(&format!(
                    "{}.{} must be in [{},{}] (got {})",
                    prefix, label, DIM_MIN, DIM_MAX, dim
                ));
            }
            if dim % DIM_MULTIPLE_OF != 0 {
                return invalid(&format!(
                    "{}.{} must be a multiple of {} (got {})",
                    prefix, label, DIM_MULTIPLE_OF, dim
                ));
            }
        }
    }

    // output_format
    if let Some(f) = spec.get("output_format").and_then(JsonValue::as_str) {
        if !VALID_OUTPUT_FORMATS.contains(&f.to_ascii_lowercase().as_str()) {
            return invalid(&format!(
                "{}.output_format {:?} unknown; supported: {:?}",
                prefix, f, VALID_OUTPUT_FORMATS
            ));
        }
    }

    // safety_tolerance
    if let Some(st) = spec.get("safety_tolerance").and_then(JsonValue::as_u64) {
        if st > SAFETY_MAX {
            return invalid(&format!(
                "{}.safety_tolerance must be in [0,{}] (got {})",
                prefix, SAFETY_MAX, st
            ));
        }
    }

    // seed (always u64; no upper bound — provider will saturate)
    if let Some(s) = spec.get("seed") {
        if !s.is_u64() && !s.is_null() {
            return invalid(&format!("{}.seed must be a non-negative integer", prefix));
        }
    }

    // watermark (bool)
    if let Some(w) = spec.get("watermark") {
        if !w.is_boolean() && !w.is_null() {
            return invalid(&format!("{}.watermark must be a boolean", prefix));
        }
    }

    // extras
    if let Some(extras) = spec.get("extras").and_then(JsonValue::as_object) {
        for (k, v) in extras {
            if EXTRAS_BLACKLIST.contains(&k.to_ascii_lowercase().as_str()) {
                return invalid(&format!("{}.extras key {:?} is blacklisted", prefix, k));
            }
            if !key_re.is_match(k) {
                return invalid(&format!(
                    "{}.extras key {:?} must match {}",
                    prefix, k, EXTRAS_KEY_RE
                ));
            }
            let value_str = match v {
                JsonValue::String(s) => s.clone(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Number(n) => n.to_string(),
                _ => {
                    return invalid(&format!(
                        "{}.extras[{:?}] must be string/bool/number",
                        prefix, k
                    ))
                }
            };
            if value_str.len() > EXTRAS_VALUE_MAX_LEN {
                return invalid(&format!(
                    "{}.extras[{:?}] value exceeds {} chars",
                    prefix, k, EXTRAS_VALUE_MAX_LEN
                ));
            }
        }
    }

    Ok(())
}

fn validate_aspect_ratio(ar: &str, provider: &ProviderRoute, prefix: &str) -> Result<(), ApiError> {
    let parts: Vec<&str> = ar.split(':').collect();
    if parts.len() != 2 {
        return invalid(&format!(
            "{}.aspect_ratio {:?} must be W:H format",
            prefix, ar
        ));
    }
    let (w, h) = match (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => (w, h),
        _ => {
            return invalid(&format!(
                "{}.aspect_ratio {:?} components must be positive integers",
                prefix, ar
            ))
        }
    };
    if matches!(provider, ProviderRoute::Flux) {
        let r = w as f64 / h as f64;
        let lower = 3.0_f64 / 7.0;
        let upper = 7.0_f64 / 3.0;
        // Allow tiny epsilon for boundary ratios (21:9 = 7:3 exactly).
        let eps = 1e-6;
        if r < lower - eps || r > upper + eps {
            return invalid(&format!(
                "{}.aspect_ratio {:?} = {:.4} outside Flux range [3:7={:.4}, 7:3={:.4}]",
                prefix, ar, r, lower, upper
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderRoute {
    Flux,
    Seedream,
    OpenAI,
}

impl ProviderRoute {
    fn as_str(&self) -> &'static str {
        match self {
            ProviderRoute::Flux => "flux",
            ProviderRoute::Seedream => "seedream",
            ProviderRoute::OpenAI => "openai",
        }
    }
}

fn resolve_provider(model: &str, hint: Option<&str>) -> ProviderRoute {
    if let Some(h) = hint {
        match h {
            "flux" => return ProviderRoute::Flux,
            "seedream" => return ProviderRoute::Seedream,
            "openai" => return ProviderRoute::OpenAI,
            _ => {}
        }
    }
    if model.starts_with("flux-kontext-") || model.starts_with("flux-") {
        ProviderRoute::Flux
    } else if model.starts_with("seedream-") {
        ProviderRoute::Seedream
    } else {
        ProviderRoute::OpenAI
    }
}

fn invalid_err(msg: &str) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg.to_string()))
}

fn invalid(msg: &str) -> Result<(), ApiError> {
    Err(invalid_err(msg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok(v: serde_json::Value) {
        validate(&v).expect("should validate");
    }
    fn err_with(v: serde_json::Value, needle: &str) {
        let e = validate(&v).expect_err("should fail");
        match e {
            ApiError::BusinessError(BusinessError::InvalidInput(msg)) => {
                assert!(msg.contains(needle), "expected {:?} in {}", needle, msg);
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn no_image_generations_passes() {
        ok(json!({}));
        ok(json!({"image_generations": []}));
    }

    #[test]
    fn flux_t2i_full_passes() {
        ok(json!({
            "image_generations": [{
                "prompts": ["a sunset"],
                "count": 1,
                "model": "flux-kontext-pro",
                "mode": "text_to_image",
                "aspect_ratio": "16:9",
                "seed": 42,
                "output_format": "png",
                "safety_tolerance": 2,
                "extras": {"prompt_upsampling": "true"}
            }]
        }));
    }

    #[test]
    fn seedream_edit_full_passes() {
        ok(json!({
            "image_generations": [{
                "prompts": ["edit pls"],
                "count": 2,
                "model": "seedream-4-5-251128",
                "mode": "image_edit",
                "reference_image_urls": [
                    "https://x/a.png",
                    "https://x/b.png",
                    "https://x/c.png"
                ],
                "width_px": 1280,
                "height_px": 720,
                "watermark": false,
                "extras": {"sequential_image_generation": "disabled"}
            }]
        }));
    }

    #[test]
    fn empty_prompt_rejected() {
        err_with(
            json!({"image_generations": [{"prompts": [""], "model": "flux-kontext-pro"}]}),
            "is empty",
        );
    }

    #[test]
    fn count_out_of_range() {
        err_with(
            json!({"image_generations": [{"prompts": ["x"], "count": 11, "model": "flux-kontext-pro"}]}),
            "count must be in",
        );
    }

    #[test]
    fn flux_aspect_ratio_out_of_range() {
        err_with(
            json!({"image_generations": [{"prompts": ["x"], "model": "flux-kontext-pro", "aspect_ratio": "8:1"}]}),
            "outside Flux range",
        );
    }

    #[test]
    fn flux_aspect_ratio_boundary_21x9_ok() {
        // 21:9 == 7:3, on the upper boundary
        ok(json!({
            "image_generations": [{
                "prompts": ["x"],
                "model": "flux-kontext-pro",
                "aspect_ratio": "21:9"
            }]
        }));
    }

    #[test]
    fn aspect_ratio_xor_explicit_size() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro",
                "aspect_ratio": "1:1", "width_px": 1024, "height_px": 1024
            }]}),
            "mutually exclusive",
        );
    }

    #[test]
    fn explicit_size_must_be_multiple_of_8() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "seedream-4-0-250828",
                "width_px": 1023, "height_px": 720
            }]}),
            "multiple of 8",
        );
    }

    #[test]
    fn explicit_size_range() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "seedream-4-0-250828",
                "width_px": 100, "height_px": 100
            }]}),
            "must be in",
        );
    }

    #[test]
    fn flux_refs_limit_2() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro", "mode": "image_edit",
                "reference_image_urls": ["https://x/a.png", "https://x/b.png", "https://x/c.png"]
            }]}),
            "at most 2",
        );
    }

    #[test]
    fn seedream_refs_limit_10() {
        let urls: Vec<String> = (0..11).map(|i| format!("https://x/{}.png", i)).collect();
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "seedream-4-0-250828", "mode": "image_edit",
                "reference_image_urls": urls
            }]}),
            "at most 10",
        );
    }

    #[test]
    fn edit_mode_requires_refs() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro", "mode": "image_edit"
            }]}),
            "reference_image_urls.len() >= 1",
        );
    }

    #[test]
    fn ref_must_be_http_url() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "seedream-4-0-250828", "mode": "image_edit",
                "reference_image_urls": ["data:image/png;base64,xx"]
            }]}),
            "must start with http",
        );
    }

    #[test]
    fn unknown_provider_hint_rejected() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro", "provider_hint": "midjourney"
            }]}),
            "provider_hint",
        );
    }

    #[test]
    fn extras_blacklist_rejected() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro",
                "extras": {"api_key": "sk-evil"}
            }]}),
            "blacklisted",
        );
    }

    #[test]
    fn safety_tolerance_out_of_range() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro", "safety_tolerance": 7
            }]}),
            "safety_tolerance must be in",
        );
    }

    #[test]
    fn output_format_unknown() {
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "flux-kontext-pro", "output_format": "tiff"
            }]}),
            "output_format",
        );
    }

    #[test]
    fn legacy_openai_rejects_refs() {
        // openai-routed model with refs hits the ref-limit check first,
        // which is the primary signal — confirm the path rejects in any way.
        err_with(
            json!({"image_generations": [{
                "prompts": ["x"], "model": "gpt-4o-image", "mode": "image_edit",
                "reference_image_urls": ["https://x/a.png"]
            }]}),
            "openai",
        );
    }
}
