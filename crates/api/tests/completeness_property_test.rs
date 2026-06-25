//! Property tests for the task-config completeness gate (Module A).
//!
//! These encode invariants that must hold for ALL drafts/thresholds. They are
//! written against the public API and must FAIL against the deliberately-wrong
//! Module-A skeleton stubs (RED), then pass once the real logic is implemented.

use glance_mind_api::service::ai_chat::completeness::{
    evaluate_with_threshold, CompletenessReport, DraftConfig,
};
use glance_mind_api::service::ai_chat::task_spec::{TaskConfigSpec, TaskKind};
use proptest::prelude::*;
use std::collections::BTreeSet;

/// All keys that can appear in a campaign draft (required + recommended +
/// a couple of optional/uncounted ones, to exercise the "ignore optional" path).
const CAMPAIGN_KEYS: &[&str] = &[
    // required
    "name",
    "platform_id",
    "region_id",
    "ai_model_id",
    "schedule_type",
    "product_prompt",
    "max_scan_count",
    // recommended
    "target_audience",
    "keyword",
    "call_to_action",
    "tone_of_voice",
    "budget_cap",
    "reply_template_ids",
    // optional / uncounted
    "enable_ai_refactor",
    "persona_id",
];

/// Publish-plan `plan_type` values that drive conditional-required resolution.
const PLAN_TYPES: &[&str] = &[
    "batch_text",
    "account_grooming",
    "reddit_text",
    "reddit_image",
    "reddit_link",
    "single_video",
    "direct_publish",
    "page_manage",
];

fn to_set(v: &[String]) -> BTreeSet<String> {
    v.iter().cloned().collect()
}

fn static_set(v: &[&'static str]) -> BTreeSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Build a campaign draft from a subset of keys (each present key gets a valid
/// non-empty value; `max_scan_count` gets a positive integer).
fn campaign_draft(selected: &[bool]) -> DraftConfig {
    let mut map = serde_json::Map::new();
    for (i, key) in CAMPAIGN_KEYS.iter().enumerate() {
        if selected.get(i).copied().unwrap_or(false) {
            let val = match *key {
                "max_scan_count" | "budget_cap" => serde_json::json!(42),
                "reply_template_ids" => serde_json::json!([1, 2]),
                "enable_ai_refactor" => serde_json::json!(true),
                _ => serde_json::json!("value"),
            };
            map.insert(key.to_string(), val);
        }
    }
    DraftConfig(map)
}

/// Strategy producing a random subset-selection over CAMPAIGN_KEYS.
fn campaign_selection() -> impl Strategy<Value = Vec<bool>> {
    proptest::collection::vec(any::<bool>(), CAMPAIGN_KEYS.len())
}

proptest! {
    /// present and (missing_required ∪ missing_recommended) partition the expected
    /// set: they are disjoint, and their union restricted to expected_keys equals
    /// expected_keys exactly. Optional keys never appear in any of the three.
    #[test]
    fn prop_present_and_missing_partition_expected(sel in campaign_selection()) {
        let draft = campaign_draft(&sel);
        let spec = TaskConfigSpec::for_kind(TaskKind::Campaign, &draft);
        let expected: BTreeSet<String> = static_set(&spec.expected_keys());

        // The campaign expected set is well-defined and non-empty; if the spec
        // returns nothing the contract is violated.
        prop_assert!(
            !expected.is_empty(),
            "expected_keys() must be non-empty for campaign"
        );

        let report: CompletenessReport =
            evaluate_with_threshold(TaskKind::Campaign, &draft, 3);

        let present = to_set(&report.present);
        let mut missing = to_set(&report.missing_required);
        missing.extend(to_set(&report.missing_recommended));

        // Disjoint.
        let overlap: Vec<_> = present.intersection(&missing).cloned().collect();
        prop_assert!(
            overlap.is_empty(),
            "present and missing must be disjoint; overlap = {overlap:?}"
        );

        // Union restricted to expected equals expected.
        let mut union = present.clone();
        union.extend(missing.iter().cloned());
        let union_in_expected: BTreeSet<String> =
            union.intersection(&expected).cloned().collect();
        prop_assert_eq!(
            union_in_expected,
            expected,
            "present ∪ missing (restricted to expected) must cover the whole expected set"
        );
    }

    /// Filling in a previously-absent expected key with a valid value never
    /// increases missing_count. Includes a deterministic strict-decrease anchor:
    /// adding a required key that was absent must strictly reduce missing_count.
    #[test]
    fn prop_adding_present_field_never_increases_missing(
        sel in campaign_selection(),
        idx in 0usize..CAMPAIGN_KEYS.len(),
    ) {
        let base = campaign_draft(&sel);
        let base_report = evaluate_with_threshold(TaskKind::Campaign, &base, 3);

        // Flip key `idx` to present.
        let mut sel2 = sel.clone();
        sel2[idx] = true;
        let bigger = campaign_draft(&sel2);
        let bigger_report = evaluate_with_threshold(TaskKind::Campaign, &bigger, 3);

        // missing_count is exactly the size of the two missing buckets.
        prop_assert_eq!(
            bigger_report.missing_count,
            bigger_report.missing_required.len() + bigger_report.missing_recommended.len(),
            "missing_count must equal missing_required.len() + missing_recommended.len()"
        );

        // Monotonicity: adding a present field never increases missing_count.
        prop_assert!(
            bigger_report.missing_count <= base_report.missing_count,
            "adding a present field must not increase missing_count: {} -> {} (key={})",
            base_report.missing_count,
            bigger_report.missing_count,
            CAMPAIGN_KEYS[idx]
        );

        // Deterministic strict-decrease anchor (independent of `idx`/`sel`):
        // take the draft WITHOUT platform_id (a required key) vs WITH it. The
        // "with" draft must have strictly fewer missing fields, all else equal.
        // `platform_id` is index 1 in CAMPAIGN_KEYS.
        let mut sel_without = sel.clone();
        sel_without[1] = false; // platform_id absent
        let mut sel_with = sel_without.clone();
        sel_with[1] = true; // platform_id present
        let without = evaluate_with_threshold(
            TaskKind::Campaign,
            &campaign_draft(&sel_without),
            3,
        );
        let with = evaluate_with_threshold(
            TaskKind::Campaign,
            &campaign_draft(&sel_with),
            3,
        );
        prop_assert!(
            with.missing_count < without.missing_count,
            "adding required `platform_id` (was absent) must strictly reduce missing_count: {} -> {}",
            without.missing_count,
            with.missing_count
        );
        prop_assert!(
            !with.missing_required.iter().any(|k| k == "platform_id"),
            "after adding platform_id it must not be in missing_required, got {:?}",
            with.missing_required
        );
    }

    /// should_offer_template is exactly (missing_count >= threshold) for any threshold.
    #[test]
    fn prop_threshold_boundary(sel in campaign_selection(), threshold in 0usize..20) {
        let draft = campaign_draft(&sel);
        let report = evaluate_with_threshold(TaskKind::Campaign, &draft, threshold);

        prop_assert_eq!(
            report.should_offer_template,
            report.missing_count >= threshold,
            "should_offer_template must equal (missing_count {} >= threshold {})",
            report.missing_count,
            threshold
        );
    }

    /// For aipub, changing plan_type never removes platform_id/content_type from
    /// the required set (they are always-required).
    #[test]
    fn prop_conditional_required_consistency(pt_idx in 0usize..PLAN_TYPES.len()) {
        let plan_type = PLAN_TYPES[pt_idx];
        let draft = DraftConfig::from_value(serde_json::json!({ "plan_type": plan_type }));
        let spec = TaskConfigSpec::for_kind(TaskKind::PublishPlan, &draft);
        let required = static_set(&spec.required_keys());

        prop_assert!(
            required.contains("platform_id"),
            "platform_id must remain required for plan_type={plan_type}; got {required:?}"
        );
        prop_assert!(
            required.contains("content_type"),
            "content_type must remain required for plan_type={plan_type}; got {required:?}"
        );
    }
}
