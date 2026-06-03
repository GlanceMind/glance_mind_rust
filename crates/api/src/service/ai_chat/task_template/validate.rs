//! Validate a (partial) task config against its [`TaskConfigSpec`].
//!
//! [`against_spec`] returns `Ok(())` only when EVERY required key (after aipub
//! conditional-required resolution via [`TaskConfigSpec::for_kind`]) is present,
//! non-null, and of the correct JSON type for its [`FieldKind`]. Otherwise it
//! returns `Err` with one human-readable message per problem, each message
//! naming the offending key.

use super::super::completeness::DraftConfig;
use super::super::task_spec::{FieldKind, Importance, TaskConfigSpec, TaskKind};

/// Check that `config` satisfies the Required portion of the spec for `kind`.
///
/// Returns `Ok(())` when all conditional-resolved Required keys are present,
/// non-null, and type-correct; otherwise `Err(messages)` where each message
/// names a single offending key.
///
/// The spec is built via [`TaskConfigSpec::for_kind`] using a [`DraftConfig`]
/// derived from `config` itself (so aipub conditional-Required keys resolve from
/// the config's own `plan_type`). Dotted keys (e.g. `ai_input.content_prompt`)
/// resolve into nested objects. Recommended / Optional keys are NOT required for
/// validity.
pub fn against_spec(config: &serde_json::Value, kind: TaskKind) -> Result<(), Vec<String>> {
    let draft = DraftConfig::from_value(config.clone());
    let spec = TaskConfigSpec::for_kind(kind, &draft);

    let mut errors = Vec::new();
    for field in spec.fields.iter() {
        if !matches!(field.importance, Importance::Required) {
            continue;
        }

        let Some(value) = resolve_key(config, field.key) else {
            errors.push(format!("missing required key `{}`", field.key));
            continue;
        };

        if value.is_null() {
            errors.push(format!("required key `{}` is null", field.key));
            continue;
        }

        if !type_matches(value, field.kind) {
            errors.push(format!(
                "required key `{}` has the wrong type (expected {:?})",
                field.key, field.kind
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Resolve a (possibly dotted) key into a nested config object.
///
/// A key containing `.` is a path: the segment before the first `.` names a
/// nested object, the remainder names a field within it (recursively). Flat keys
/// are looked up directly. Returns `None` if any segment is absent or a
/// non-object is encountered mid-path.
fn resolve_key<'a>(config: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    let mut current = config;
    for segment in key.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// Whether `value`'s JSON type satisfies the given [`FieldKind`].
fn type_matches(value: &serde_json::Value, kind: FieldKind) -> bool {
    match kind {
        // Accept any JSON number that is an integer (i64 or u64). Reject floats
        // and string-encoded numbers.
        FieldKind::Int => value.is_i64() || value.is_u64(),
        FieldKind::String | FieldKind::Enum => value.is_string(),
        FieldKind::Bool => value.is_boolean(),
        FieldKind::Json => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A fully-populated, type-correct campaign config is accepted.
    #[test]
    fn accepts_full_campaign() {
        let config = json!({
            "name": "Spring Launch",
            "platform_id": 1,
            "region_id": 2,
            "ai_model_id": 3,
            "schedule_type": "immediate",
            "product_prompt": "Promote our new running shoe.",
            "max_scan_count": 100
        });

        let result = against_spec(&config, TaskKind::Campaign);
        assert!(
            result.is_ok(),
            "a full, type-correct campaign config must be accepted, got {result:?}"
        );
    }

    /// Dropping a Required key (region_id) yields Err whose message names that key.
    #[test]
    fn flags_missing_required() {
        let config = json!({
            "name": "Spring Launch",
            "platform_id": 1,
            // region_id intentionally omitted
            "ai_model_id": 3,
            "schedule_type": "immediate",
            "product_prompt": "Promote our new running shoe.",
            "max_scan_count": 100
        });

        let result = against_spec(&config, TaskKind::Campaign);
        let errors = result.expect_err("a campaign missing region_id must be rejected");
        assert!(
            errors.iter().any(|m| m.contains("region_id")),
            "the error list must name the missing key `region_id`, got {errors:?}"
        );
    }

    /// A Required int field given a string value is rejected, naming the key.
    #[test]
    fn rejects_wrong_type() {
        let config = json!({
            "name": "Spring Launch",
            "platform_id": "abc", // wrong type: should be an int
            "region_id": 2,
            "ai_model_id": 3,
            "schedule_type": "immediate",
            "product_prompt": "Promote our new running shoe.",
            "max_scan_count": 100
        });

        let result = against_spec(&config, TaskKind::Campaign);
        let errors = result.expect_err("a campaign with a non-int platform_id must be rejected");
        assert!(
            errors.iter().any(|m| m.contains("platform_id")),
            "the error list must name the wrong-typed key `platform_id`, got {errors:?}"
        );
    }
}
