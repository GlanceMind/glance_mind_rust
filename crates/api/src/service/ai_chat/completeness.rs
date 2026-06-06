//! Task-config completeness gate.
//!
//! Given a partially-filled draft config for a task (campaign or publish plan),
//! evaluate which expected fields are present vs missing, and decide whether the
//! assistant should proactively offer a sample template and/or block submission.
//!
//! `evaluate_with_threshold` is PURE: it performs no I/O, reads no clock, and uses
//! no randomness. `evaluate` is the thin wrapper that reads the threshold env var
//! once and delegates. `missing_threshold` resolves the env var default.

use super::task_spec::{FieldKind, FieldSpec, Importance, TaskConfigSpec, TaskKind};

/// Environment variable controlling how many missing fields trigger the
/// "offer a sample template" affordance.
pub const MISSING_THRESHOLD_ENV: &str = "AI_TASK_TEMPLATE_MISSING_THRESHOLD";

/// Default missing-field threshold when the env var is unset / unparsable.
pub const DEFAULT_MISSING_THRESHOLD: usize = 3;

/// A draft task configuration: a flat JSON object of user-supplied keys/values.
#[derive(Debug, Clone, Default)]
pub struct DraftConfig(pub serde_json::Map<String, serde_json::Value>);

impl DraftConfig {
    /// Build a draft from a `serde_json::Value::Object`. Non-objects yield empty.
    pub fn from_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Object(map) => DraftConfig(map),
            _ => DraftConfig(serde_json::Map::new()),
        }
    }

    /// Borrow the raw value for a key, if present.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }

    /// Resolve a (possibly nested) field key to its raw JSON value.
    ///
    /// A key containing a `.` is treated as a path: the segment before the first
    /// `.` names a nested object in the draft, and the remainder names a field
    /// within it (recursively). Flat keys are looked up directly. Returns `None`
    /// if any segment is absent or a non-object is encountered mid-path.
    fn resolve(&self, key: &str) -> Option<&serde_json::Value> {
        match key.split_once('.') {
            None => self.0.get(key),
            Some((head, rest)) => {
                let mut current = self.0.get(head)?;
                for segment in rest.split('.') {
                    current = current.as_object()?.get(segment)?;
                }
                Some(current)
            }
        }
    }
}

/// Decide whether `field` is "present" in `draft` per the completeness rules:
///
/// - The (possibly nested) key must exist and be non-null.
/// - For `String`-typed fields, the value (a JSON string) must be non-empty
///   after trimming whitespace.
/// - For the special `max_scan_count` key, the numeric value must be `> 0`.
fn is_present(draft: &DraftConfig, field: &FieldSpec) -> bool {
    let Some(value) = draft.resolve(field.key) else {
        return false;
    };

    if value.is_null() {
        return false;
    }

    // String-typed fields must be non-empty after trim. (A non-string JSON value
    // for a String-typed key is treated as present-if-non-null, leaving type
    // validation to a later layer.)
    if matches!(field.kind, FieldKind::String) {
        if let Some(s) = value.as_str() {
            if s.trim().is_empty() {
                return false;
            }
        }
    }

    // `max_scan_count` is only "present" when its numeric value is strictly
    // positive (zero / negative counts as missing).
    if field.key == "max_scan_count" {
        match value.as_i64() {
            Some(n) => return n > 0,
            None => match value.as_f64() {
                Some(n) => return n > 0.0,
                None => return false,
            },
        }
    }

    true
}

/// Result of evaluating a draft against its task config spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletenessReport {
    /// Expected keys that are considered "present" in the draft.
    pub present: Vec<String>,
    /// Required keys that are missing.
    pub missing_required: Vec<String>,
    /// Recommended keys that are missing.
    pub missing_recommended: Vec<String>,
    /// `missing_required.len() + missing_recommended.len()`.
    pub missing_count: usize,
    /// `missing_count >= threshold`.
    pub should_offer_template: bool,
    /// `!missing_required.is_empty()`.
    pub blocking: bool,
}

/// Resolve the missing-field threshold from the environment, falling back to
/// [`DEFAULT_MISSING_THRESHOLD`]. Reads the process environment.
pub fn missing_threshold() -> usize {
    std::env::var(MISSING_THRESHOLD_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MISSING_THRESHOLD)
}

/// Evaluate a draft. Reads the threshold env once, then delegates to the pure
/// [`evaluate_with_threshold`].
pub fn evaluate(kind: TaskKind, draft: &DraftConfig) -> CompletenessReport {
    let threshold = missing_threshold();
    evaluate_with_threshold(kind, draft, threshold)
}

/// PURE evaluation against an explicit threshold. No I/O, no clock, no rng.
pub fn evaluate_with_threshold(
    kind: TaskKind,
    draft: &DraftConfig,
    threshold: usize,
) -> CompletenessReport {
    let spec = TaskConfigSpec::for_kind(kind, draft);

    let mut present = Vec::new();
    let mut missing_required = Vec::new();
    let mut missing_recommended = Vec::new();

    for field in &spec.fields {
        // Optional fields are never counted toward completeness in any bucket.
        if matches!(field.importance, Importance::Optional) {
            continue;
        }

        if is_present(draft, field) {
            present.push(field.key.to_string());
        } else {
            match field.importance {
                Importance::Required => missing_required.push(field.key.to_string()),
                Importance::Recommended => missing_recommended.push(field.key.to_string()),
                Importance::Optional => unreachable!("optional fields filtered above"),
            }
        }
    }

    let missing_count = missing_required.len() + missing_recommended.len();
    let should_offer_template = missing_count >= threshold;
    let blocking = !missing_required.is_empty();

    CompletenessReport {
        present,
        missing_required,
        missing_recommended,
        missing_count,
        should_offer_template,
        blocking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ai_chat::task_spec::TaskKind;
    use serde_json::json;

    fn draft(v: serde_json::Value) -> DraftConfig {
        DraftConfig::from_value(v)
    }

    /// A campaign draft with all 7 required + all 6 recommended fields populated
    /// must report zero missing, no template offer, and non-blocking.
    #[test]
    fn campaign_full_draft_has_zero_missing() {
        let d = draft(json!({
            // 7 required
            "name": "Summer Launch",
            "platform_id": "tiktok",
            "region_id": "us",
            "ai_model_id": "gpt-4o",
            "schedule_type": "daily",
            "product_prompt": "Sell the new sneakers",
            "max_scan_count": 50,
            // 6 recommended
            "target_audience": "runners",
            "keyword": "sneakers",
            "call_to_action": "Buy now",
            "tone_of_voice": "friendly",
            "budget_cap": 100,
            "reply_template_ids": [1, 2, 3]
        }));

        // Use a fixed threshold (not env) for determinism in parallel tests.
        let report = evaluate_with_threshold(TaskKind::Campaign, &d, 3);

        assert_eq!(
            report.missing_count, 0,
            "fully-populated campaign must have zero missing fields, got {report:?}"
        );
        assert!(
            report.missing_required.is_empty(),
            "fully-populated campaign must have empty missing_required, got {report:?}"
        );
        assert!(
            report.missing_recommended.is_empty(),
            "fully-populated campaign must have empty missing_recommended, got {report:?}"
        );
        assert!(
            !report.should_offer_template,
            "fully-populated campaign must not offer a template, got {report:?}"
        );
        assert!(
            !report.blocking,
            "fully-populated campaign must not block, got {report:?}"
        );
        // The 13 expected keys (7 required + 6 recommended) must all be recognized
        // as present. This also guards against a stub that trivially returns an
        // all-empty report (which would otherwise satisfy "zero missing").
        for key in [
            "name",
            "platform_id",
            "region_id",
            "ai_model_id",
            "schedule_type",
            "product_prompt",
            "max_scan_count",
            "target_audience",
            "keyword",
            "call_to_action",
            "tone_of_voice",
            "budget_cap",
            "reply_template_ids",
        ] {
            assert!(
                report.present.iter().any(|k| k == key),
                "expected key `{key}` must be present in a fully-populated draft, got {:?}",
                report.present
            );
        }
    }

    /// A sparse campaign draft (only name + product_prompt) is missing many
    /// required fields: must offer template and block, and the missing-required
    /// set must include the obvious required keys.
    #[test]
    fn campaign_sparse_draft_offers_template() {
        let d = draft(json!({ "name": "x", "product_prompt": "y" }));

        let report = evaluate_with_threshold(TaskKind::Campaign, &d, 3);

        assert!(
            report.missing_count >= 3,
            "sparse campaign must have >=3 missing, got {report:?}"
        );
        assert!(
            report.should_offer_template,
            "sparse campaign (>=threshold missing) must offer template, got {report:?}"
        );
        assert!(
            report.blocking,
            "sparse campaign with missing required must block, got {report:?}"
        );
        for key in [
            "platform_id",
            "region_id",
            "ai_model_id",
            "schedule_type",
            "max_scan_count",
        ] {
            assert!(
                report.missing_required.iter().any(|k| k == key),
                "missing_required must contain `{key}`, got {:?}",
                report.missing_required
            );
        }
    }

    /// A whitespace-only string is treated as missing (present = non-empty after trim).
    #[test]
    fn empty_string_is_missing() {
        let d = draft(json!({ "name": "  " }));

        let report = evaluate_with_threshold(TaskKind::Campaign, &d, 3);

        assert!(
            report.missing_required.iter().any(|k| k == "name"),
            "whitespace-only `name` must be in missing_required, got {:?}",
            report.missing_required
        );
        assert!(
            !report.present.iter().any(|k| k == "name"),
            "whitespace-only `name` must not be present, got {:?}",
            report.present
        );
    }

    /// `max_scan_count` counts as present only when > 0; zero is missing.
    #[test]
    fn max_scan_count_zero_is_missing() {
        let d = draft(json!({ "max_scan_count": 0 }));

        let report = evaluate_with_threshold(TaskKind::Campaign, &d, 3);

        assert!(
            report
                .missing_required
                .iter()
                .any(|k| k == "max_scan_count"),
            "max_scan_count == 0 must be in missing_required, got {:?}",
            report.missing_required
        );
    }

    /// When all 7 required fields are present but some recommended are missing,
    /// the report is non-blocking (no missing required) yet may still offer a
    /// template once the recommended-misses reach the threshold.
    #[test]
    fn all_required_present_is_non_blocking() {
        // 7 required present; 2 of 6 recommended present => 4 recommended missing.
        let d = draft(json!({
            "name": "Summer Launch",
            "platform_id": "tiktok",
            "region_id": "us",
            "ai_model_id": "gpt-4o",
            "schedule_type": "daily",
            "product_prompt": "Sell the new sneakers",
            "max_scan_count": 50,
            "target_audience": "runners",
            "keyword": "sneakers"
        }));

        let report = evaluate_with_threshold(TaskKind::Campaign, &d, 3);

        assert!(
            !report.blocking,
            "all-required-present draft must be non-blocking, got {report:?}"
        );
        assert!(
            report.missing_required.is_empty(),
            "all-required-present draft must have empty missing_required, got {:?}",
            report.missing_required
        );
        // 4 recommended missing >= threshold(3) => offer template.
        assert!(
            report.should_offer_template,
            "4 recommended missing (>=3) must still offer a template, got {report:?}"
        );
    }

    /// The default threshold is 3 when the env var is unset. We read & restore the
    /// env carefully to avoid cross-test interference.
    #[test]
    fn threshold_defaults_to_three() {
        // Capture prior value, ensure unset, assert, then restore.
        let prior = std::env::var(MISSING_THRESHOLD_ENV).ok();
        std::env::remove_var(MISSING_THRESHOLD_ENV);

        let got = missing_threshold();

        // Restore before asserting so a failure can't leak state.
        match &prior {
            Some(v) => std::env::set_var(MISSING_THRESHOLD_ENV, v),
            None => std::env::remove_var(MISSING_THRESHOLD_ENV),
        }

        assert_eq!(
            got, DEFAULT_MISSING_THRESHOLD,
            "missing_threshold() must default to {DEFAULT_MISSING_THRESHOLD} when env unset"
        );
        assert_eq!(
            DEFAULT_MISSING_THRESHOLD, 3,
            "the documented default constant must be 3"
        );
    }

    /// The same sparse draft offers a template at threshold 3 but not at 12.
    /// Uses `evaluate_with_threshold` for determinism (no process-env mutation).
    ///
    /// ASSERTION-CHANGE-JUSTIFIED: the original "high" threshold of 9 encoded a
    /// wrong expectation. The sparse campaign draft {name, product_prompt} is missing
    /// 5 required + 6 recommended = 11 expected configs, so missing_count == 11; with
    /// should_offer_template == (missing_count >= threshold), 11 >= 9 is true, making
    /// the "not offered at 9" assertion mathematically unsatisfiable alongside the
    /// (immovable) partition/sparse tests. Corrected the high threshold to 12 (> 11),
    /// which preserves the test's intent (offers at a low threshold, not at a high one).
    #[test]
    fn threshold_via_evaluate_with_threshold() {
        let d = draft(json!({ "name": "x", "product_prompt": "y" }));

        let low = evaluate_with_threshold(TaskKind::Campaign, &d, 3);
        let high = evaluate_with_threshold(TaskKind::Campaign, &d, 12);

        assert!(
            low.should_offer_template,
            "threshold 3 on sparse draft must offer template, got {low:?}"
        );
        assert!(
            !high.should_offer_template,
            "threshold 12 on sparse draft must NOT offer template, got {high:?}"
        );
    }

    // --- Nested-key (`ai_input.content_prompt`) coverage for publish_plan ---
    // Covers the dotted-path `resolve` branch (shipped untested per the code-quality
    // review). Spec: `ai_input.content_prompt` is a nested Recommended key, present
    // only when the `ai_input` object holds a non-empty `content_prompt` string.
    // A non-walking implementation would mark the present-case below as missing.

    #[test]
    fn aipub_nested_content_prompt_present_when_leaf_set() {
        let d = draft(json!({
            "platform_id": 1, "content_type": "post", "group_id": 7,
            "ai_input": { "content_prompt": "make a fun launch post" }
        }));
        let r = evaluate_with_threshold(TaskKind::PublishPlan, &d, 3);
        assert!(
            r.present.iter().any(|k| k == "ai_input.content_prompt"),
            "nested ai_input.content_prompt must be detected present, got {r:?}"
        );
        assert!(
            !r.missing_recommended
                .iter()
                .any(|k| k == "ai_input.content_prompt"),
            "nested key must not be simultaneously present and missing, got {r:?}"
        );
    }

    #[test]
    fn aipub_nested_content_prompt_missing_when_ai_input_absent() {
        let d = draft(json!({ "platform_id": 1, "content_type": "post", "group_id": 7 }));
        let r = evaluate_with_threshold(TaskKind::PublishPlan, &d, 3);
        assert!(
            r.missing_recommended
                .iter()
                .any(|k| k == "ai_input.content_prompt"),
            "absent nested key must be in missing_recommended, got {r:?}"
        );
    }

    #[test]
    fn aipub_nested_content_prompt_missing_when_parent_not_object() {
        let d = draft(
            json!({ "platform_id": 1, "content_type": "post", "group_id": 7, "ai_input": 5 }),
        );
        let r = evaluate_with_threshold(TaskKind::PublishPlan, &d, 3);
        assert!(
            r.missing_recommended
                .iter()
                .any(|k| k == "ai_input.content_prompt"),
            "a non-object ai_input must leave the nested key missing, got {r:?}"
        );
    }

    #[test]
    fn aipub_nested_content_prompt_missing_when_leaf_blank() {
        let d = draft(json!({
            "platform_id": 1, "content_type": "post", "group_id": 7,
            "ai_input": { "content_prompt": "   " }
        }));
        let r = evaluate_with_threshold(TaskKind::PublishPlan, &d, 3);
        assert!(
            r.missing_recommended
                .iter()
                .any(|k| k == "ai_input.content_prompt"),
            "a blank nested leaf must be treated as missing, got {r:?}"
        );
    }
}
