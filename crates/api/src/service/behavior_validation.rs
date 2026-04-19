//! Validation for plan-level `PublishBehavior` JSON.
//!
//! `behavior` arrives from the public API as an opaque `JsonValue` so
//! we don't lose forward-compat — proto fields graduating from
//! `extras` to typed fields keep working without a DTO change.
//!
//! This module enforces the *structural* invariants we owe the
//! downstream worker: visibility is a known enum string, all bool
//! fields are bool, and `extras` is a string→string map. Unknown
//! top-level keys are tolerated (proto adds them all the time and the
//! Python worker safely ignores unknown fields via
//! `UnifiedPublishContent.from_dict`), so the API only rejects shapes
//! that would actively *corrupt* the worker's parse.
//!
//! Source of truth: glance_mind_protocol/proto/aipub.proto §960
//! (PublishBehavior message).

use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use serde_json::Value as JsonValue;

/// Bool-typed top-level fields on PublishBehavior. Listed here rather
/// than reflected from the prost type because the JSON wire form is
/// the public contract — we're validating wire shape, not Rust types.
const BOOL_FIELDS: &[&str] = &[
    "allow_comments",
    "allow_sharing",
    "allow_download",
    "is_nsfw",
    "is_spoiler",
    "ai_generated_disclosure",
    "allow_duet",
    "allow_stitch",
    "disclose_branded_content",
    "allow_remix",
    "share_to_facebook",
];

/// Lowercase strings accepted for the `visibility` field. Mirrors the
/// proto Visibility enum (Unspecified / Public / Unlisted / Friends /
/// Private) using the shorter form already in use across the
/// front-end and worker (e.g. `_first_text`-style snake_case).
const VISIBILITY_VALUES: &[&str] = &[
    "unspecified",
    "public",
    "unlisted",
    "friends",
    "followers_only", // alias for `friends` per UI vocab
    "private",
];

/// Validate a plan's behavior payload.
///
/// `Ok(())` when the shape is acceptable (or `behavior` is `None`).
/// `Err(ApiError::BadRequest)` with a precise message otherwise.
pub fn validate(behavior: Option<&JsonValue>) -> Result<(), ApiError> {
    let Some(value) = behavior else {
        return Ok(());
    };

    let map = value
        .as_object()
        .ok_or_else(|| invalid("behavior must be a JSON object".to_string()))?;

    if let Some(vis) = map.get("visibility") {
        let vis_str = vis
            .as_str()
            .ok_or_else(|| invalid("behavior.visibility must be a string".to_string()))?;
        if !VISIBILITY_VALUES.contains(&vis_str.to_ascii_lowercase().as_str()) {
            return Err(invalid(format!(
                "behavior.visibility must be one of {:?}, got {:?}",
                VISIBILITY_VALUES, vis_str
            )));
        }
    }

    for field in BOOL_FIELDS {
        if let Some(v) = map.get(*field) {
            if !v.is_boolean() {
                return Err(invalid(format!(
                    "behavior.{} must be a boolean, got {}",
                    field,
                    type_name(v)
                )));
            }
        }
    }

    if let Some(extras) = map.get("extras") {
        let extras_obj = extras.as_object().ok_or_else(|| {
            invalid("behavior.extras must be a JSON object (string→string map)".to_string())
        })?;
        for (k, v) in extras_obj {
            if !v.is_string() {
                return Err(invalid(format!(
                    "behavior.extras.{} must be a string, got {}",
                    k,
                    type_name(v)
                )));
            }
        }
    }

    Ok(())
}

fn invalid(msg: String) -> ApiError {
    ApiError::BusinessError(BusinessError::InvalidInput(msg))
}

fn type_name(v: &JsonValue) -> &'static str {
    match v {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "bool",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn none_is_valid() {
        assert!(validate(None).is_ok());
    }

    #[test]
    fn empty_object_is_valid() {
        let v = json!({});
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn full_well_formed_payload_passes() {
        let v = json!({
            "visibility": "public",
            "allow_comments": true,
            "allow_sharing": false,
            "is_nsfw": false,
            "allow_duet": true,
            "extras": {"k": "v"},
        });
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn unknown_visibility_rejected() {
        let v = json!({"visibility": "garbage"});
        let err = validate(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("visibility"), "{err}");
    }

    #[test]
    fn non_string_visibility_rejected() {
        let v = json!({"visibility": 1});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn non_bool_is_nsfw_rejected() {
        let v = json!({"is_nsfw": "yes"});
        let err = validate(Some(&v)).unwrap_err();
        assert!(err.to_string().contains("is_nsfw"), "{err}");
    }

    #[test]
    fn non_object_top_level_rejected() {
        let v = json!("nope");
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn extras_must_be_string_to_string() {
        let v = json!({"extras": {"k": 1}});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn extras_must_be_object() {
        let v = json!({"extras": "wrong"});
        assert!(validate(Some(&v)).is_err());
    }

    #[test]
    fn unknown_top_level_keys_pass_through_for_forward_compat() {
        // Proto graduates fields all the time; reject only what we know
        // would corrupt the worker.
        let v = json!({"future_proto_field": "anything"});
        assert!(validate(Some(&v)).is_ok());
    }

    #[test]
    fn all_visibility_values_accepted() {
        for vis in VISIBILITY_VALUES {
            let v = json!({"visibility": vis});
            assert!(validate(Some(&v)).is_ok(), "rejected: {vis}");
        }
    }

    #[test]
    fn case_insensitive_visibility() {
        let v = json!({"visibility": "PUBLIC"});
        assert!(validate(Some(&v)).is_ok());
    }
}
