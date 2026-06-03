//! Validate a (partial) task config against its [`TaskConfigSpec`].
//!
//! [`against_spec`] returns `Ok(())` only when EVERY required key (after aipub
//! conditional-required resolution via [`TaskConfigSpec::for_kind`]) is present,
//! non-null, and of the correct JSON type for its [`FieldKind`]. Otherwise it
//! returns `Err` with one human-readable message per problem, each message
//! naming the offending key.

use super::super::completeness::DraftConfig;
use super::super::task_spec::TaskKind;

/// Check that `config` satisfies the Required portion of the spec for `kind`.
///
/// Returns `Ok(())` when all conditional-resolved Required keys are present,
/// non-null, and type-correct; otherwise `Err(messages)` where each message
/// names a single offending key.
///
/// This is a deliberately-WRONG skeleton stub: it accepts everything, so the RED
/// tests that expect rejection of missing / wrong-typed configs fail on
/// assertions.
pub fn against_spec(config: &serde_json::Value, kind: TaskKind) -> Result<(), Vec<String>> {
    // WRONG STUB: never inspects `config`, always reports valid. The real
    // implementation must resolve the spec and check presence + type per key.
    let _ = (config, kind, DraftConfig::default());
    Ok(())
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
