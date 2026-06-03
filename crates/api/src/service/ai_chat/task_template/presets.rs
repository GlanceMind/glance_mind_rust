//! Deterministic, hand-curated sample-template presets.
//!
//! [`resolve`] returns the nearest fully-populated, spec-valid [`SampleTemplate`]
//! for a `(kind, draft)` combo. "Fully-populated" means every Required key (after
//! aipub conditional resolution) carries a non-null, type-correct value, so the
//! returned template always passes [`super::validate::against_spec`]. `None` is
//! returned ONLY for a genuinely-unsupported combo (e.g. an unknown aipub
//! `plan_type`).

use super::super::completeness::DraftConfig;
use super::super::task_spec::TaskKind;
use super::generator::SampleTemplate;

/// Resolve the nearest valid preset for `(kind, draft)`.
///
/// This is a deliberately-WRONG skeleton stub: it always returns `None`, so the
/// RED tests that expect a populated preset for every reachable combo fail on
/// assertions.
pub fn resolve(kind: TaskKind, draft: &DraftConfig) -> Option<SampleTemplate> {
    // WRONG STUB: claims no preset exists for any combo. The real implementation
    // must return a fully-populated, spec-valid template for every reachable
    // campaign / aipub-plan_type combo, and None only for unsupported ones.
    let _ = (kind, draft);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ai_chat::task_spec::{Importance, TaskConfigSpec};
    use crate::service::ai_chat::task_template::validate::against_spec;
    use serde_json::json;

    /// aipub plan types that must each have a fully-populated, valid preset, paired
    /// with a content_type that is compatible with that plan type.
    const AIPUB_COMBOS: &[(&str, &str)] = &[
        ("batch_text", "text"),
        ("account_grooming", "text"),
        ("single_video", "video"),
        ("direct_publish", "text"),
        ("reddit_text", "text"),
        ("reddit_image", "image"),
        ("reddit_link", "link"),
    ];

    fn draft(v: serde_json::Value) -> DraftConfig {
        DraftConfig::from_value(v)
    }

    /// Serialize a [`SampleTemplate`]'s fields back into a flat config object so it
    /// can be re-validated by [`against_spec`]. Nested-dotted keys (e.g.
    /// `ai_input.content_prompt`) are expanded into nested objects.
    fn template_to_config(t: &SampleTemplate) -> serde_json::Value {
        let mut root = serde_json::Map::new();
        for f in &t.fields {
            match f.key.split_once('.') {
                None => {
                    root.insert(f.key.clone(), f.value.clone());
                }
                Some((head, rest)) => {
                    let entry = root
                        .entry(head.to_string())
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    if let serde_json::Value::Object(obj) = entry {
                        obj.insert(rest.to_string(), f.value.clone());
                    }
                }
            }
        }
        serde_json::Value::Object(root)
    }

    /// Assert that every Required key of `spec` is present (non-null) in the
    /// template's rendered fields.
    fn assert_all_required_filled(template: &SampleTemplate, spec: &TaskConfigSpec) {
        let keys: std::collections::BTreeSet<&str> =
            template.fields.iter().map(|f| f.key.as_str()).collect();
        for f in spec.fields.iter() {
            if matches!(f.importance, Importance::Required) {
                let filled = template
                    .fields
                    .iter()
                    .any(|sf| sf.key == f.key && !sf.value.is_null());
                assert!(
                    filled && keys.contains(f.key),
                    "preset must fill Required key `{}` (present non-null field); fields = {:?}",
                    f.key,
                    template.fields.iter().map(|sf| &sf.key).collect::<Vec<_>>()
                );
            }
        }
    }

    /// Every reachable combo (campaign + each aipub plan_type) yields a Some
    /// preset that fills 100% of Required keys and passes against_spec.
    #[test]
    fn every_reachable_combo_has_a_valid_preset() {
        // --- campaign ---
        let camp_draft = draft(json!({}));
        let camp = resolve(TaskKind::Campaign, &camp_draft).expect("campaign must have a preset");
        let camp_spec = TaskConfigSpec::for_kind(TaskKind::Campaign, &camp_draft);
        assert_all_required_filled(&camp, &camp_spec);
        let camp_config = template_to_config(&camp);
        assert!(
            against_spec(&camp_config, TaskKind::Campaign).is_ok(),
            "campaign preset must pass against_spec, config = {camp_config}"
        );

        // --- aipub, every plan_type ---
        for (plan_type, content_type) in AIPUB_COMBOS {
            let d = draft(json!({ "plan_type": plan_type, "content_type": content_type }));
            let template = resolve(TaskKind::PublishPlan, &d)
                .unwrap_or_else(|| panic!("aipub plan_type={plan_type} must have a preset"));
            let spec = TaskConfigSpec::for_kind(TaskKind::PublishPlan, &d);
            assert_all_required_filled(&template, &spec);
            let config = template_to_config(&template);
            assert!(
                against_spec(&config, TaskKind::PublishPlan).is_ok(),
                "aipub preset for plan_type={plan_type} must pass against_spec, config = {config}"
            );
        }
    }

    /// A genuinely-unsupported combo (unknown aipub plan_type) yields None.
    #[test]
    fn resolve_none_for_unsupported() {
        let d = draft(json!({ "plan_type": "__nonexistent_plan_type__" }));
        let result = resolve(TaskKind::PublishPlan, &d);
        assert!(
            result.is_none(),
            "an unknown aipub plan_type must resolve to None, got {:?}",
            result.map(|t| t.fields.len())
        );
    }
}
