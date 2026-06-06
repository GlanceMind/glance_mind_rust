//! Deterministic, hand-curated sample-template presets.
//!
//! [`resolve`] returns the nearest fully-populated, spec-valid [`SampleTemplate`]
//! for a `(kind, draft)` combo. "Fully-populated" means every Required key (after
//! aipub conditional resolution) carries a non-null, type-correct value, so the
//! returned template always passes [`super::validate::against_spec`]. `None` is
//! returned ONLY for a genuinely-unsupported combo (e.g. an unknown aipub
//! `plan_type`).

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::super::completeness::DraftConfig;
use super::super::task_spec::{FieldKind, FieldSpec, Importance, TaskConfigSpec, TaskKind};
use super::generator::{EnumOption, SampleField, SampleSource, SampleTemplate};

/// The aipub `plan_type`s for which a preset exists.
pub(super) const KNOWN_PLAN_TYPES: &[&str] = &[
    "batch_text",
    "account_grooming",
    "single_video",
    "direct_publish",
    "reddit_text",
    "reddit_image",
    "reddit_link",
];

/// Resolve the nearest valid preset for `(kind, draft)`.
///
/// Returns a fully-populated, [`against_spec`](super::validate::against_spec)-valid
/// [`SampleTemplate`] (source [`SampleSource::Preset`]) for every reachable combo
/// — campaign, and aipub for each known `plan_type`. Returns `None` for an
/// unsupported combo (an aipub `plan_type` outside [`KNOWN_PLAN_TYPES`]).
pub fn resolve(kind: TaskKind, draft: &DraftConfig) -> Option<SampleTemplate> {
    let values = match kind {
        TaskKind::Campaign => campaign_values(draft),
        TaskKind::PublishPlan => {
            // Check the RAW plan_type ourselves: `for_kind` leniently defaults an
            // unknown plan_type to `batch_text`, but an unknown type has no preset.
            let plan_type = draft
                .get("plan_type")
                .and_then(|v| v.as_str())
                .unwrap_or("batch_text");
            if !KNOWN_PLAN_TYPES.contains(&plan_type) {
                return None;
            }
            publish_plan_values(draft, plan_type)
        }
    };

    let spec = TaskConfigSpec::for_kind(kind, draft);
    Some(build_template(kind, &spec, &values, SampleSource::Preset))
}

/// Render a [`SampleTemplate`] for `kind` from a `spec` and a key→value map.
///
/// Iterates the spec's fields in order, emitting a [`SampleField`] for every
/// Required and Recommended key that has a value in `values` (carrying the spec's
/// `key/label_cn/group/importance/field_type`). Enum fields get their `options`.
/// Optional fields are skipped.
pub(super) fn build_template(
    kind: TaskKind,
    spec: &TaskConfigSpec,
    values: &BTreeMap<&'static str, Value>,
    source: SampleSource,
) -> SampleTemplate {
    let mut fields = Vec::new();
    for f in spec.fields.iter() {
        // Only Required + Recommended are populated; Optional is uncounted.
        if matches!(f.importance, Importance::Optional) {
            continue;
        }
        let Some(value) = values.get(f.key) else {
            continue;
        };
        fields.push(sample_field_from_spec(f, value.clone()));
    }
    SampleTemplate {
        task_kind: kind,
        source,
        fields,
    }
}

/// Build a [`SampleField`] carrying the spec metadata + a concrete value.
pub(super) fn sample_field_from_spec(f: &FieldSpec, value: Value) -> SampleField {
    let options = if matches!(f.kind, FieldKind::Enum) {
        Some(enum_options_for(f.key))
    } else {
        None
    };
    SampleField {
        key: f.key.to_string(),
        label_cn: f.label_cn.to_string(),
        group: f.group.to_string(),
        importance: f.importance,
        field_type: f.kind,
        value,
        editable: true,
        options,
    }
}

/// Realistic enum options for a known enum-typed key.
fn enum_options_for(key: &str) -> Vec<EnumOption> {
    match key {
        "schedule_type" => vec![
            opt("immediate", "立即执行"),
            opt("scheduled", "定时执行"),
            opt("recurring", "周期执行"),
        ],
        "content_type" => vec![
            opt("text", "文本"),
            opt("image", "图片"),
            opt("video", "视频"),
            opt("link", "链接"),
        ],
        "plan_type" => vec![
            opt("batch_text", "批量文本"),
            opt("account_grooming", "账号养成"),
            opt("single_video", "单条视频"),
            opt("direct_publish", "直接发布"),
            opt("reddit_text", "Reddit 文本"),
            opt("reddit_image", "Reddit 图片"),
            opt("reddit_link", "Reddit 链接"),
        ],
        _ => Vec::new(),
    }
}

fn opt(value: &str, label: &str) -> EnumOption {
    EnumOption {
        value: json!(value),
        label: label.to_string(),
    }
}

/// A showcase campaign category, used to seed realistic campaign presets.
struct CampaignCategory {
    name: &'static str,
    product_prompt: &'static str,
    target_audience: &'static str,
    keyword: &'static str,
    call_to_action: &'static str,
    tone_of_voice: &'static str,
    /// Lowercased keyword cues that, if present in the draft, select this category.
    cues: &'static [&'static str],
}

/// The showcase categories, in priority order (first = default).
const CAMPAIGN_CATEGORIES: &[CampaignCategory] = &[
    CampaignCategory {
        name: "LEVEL8 Luggage Spring Push",
        product_prompt: "Promote LEVEL8 lightweight aluminum-frame luggage to frequent travelers.",
        target_audience: "Frequent travelers and digital nomads aged 25-45",
        keyword: "luggage",
        call_to_action: "Shop the LEVEL8 collection now",
        tone_of_voice: "aspirational",
        cues: &["level8", "luggage", "suitcase", "travel", "carry-on"],
    },
    CampaignCategory {
        name: "VOIT Running Shoes Launch",
        product_prompt: "Promote VOIT performance running shoes to runners and gym-goers.",
        target_audience: "Runners and fitness enthusiasts aged 18-40",
        keyword: "running shoes",
        call_to_action: "Grab your VOIT pair today",
        tone_of_voice: "energetic",
        cues: &["voit", "shoe", "sneaker", "running", "runner"],
    },
    CampaignCategory {
        name: "Whiteout Survival Player Acquisition",
        product_prompt: "Drive installs for the strategy game Whiteout Survival.",
        target_audience: "Mobile strategy gamers aged 20-45",
        keyword: "survival game",
        call_to_action: "Download Whiteout Survival free",
        tone_of_voice: "exciting",
        cues: &[
            "whiteout", "survival", "game", "gaming", "player", "install",
        ],
    },
    CampaignCategory {
        name: "RELX Vape Brand Awareness",
        product_prompt: "Build awareness for RELX next-generation vaping devices.",
        target_audience: "Adult smokers seeking alternatives aged 21-45",
        keyword: "vape",
        call_to_action: "Discover RELX",
        tone_of_voice: "sleek",
        cues: &["relx", "vape", "vaping", "pod", "e-cigarette"],
    },
    CampaignCategory {
        name: "KingSmith Treadmill Promotion",
        product_prompt: "Promote KingSmith foldable walking-pad treadmills for home offices.",
        target_audience: "Remote workers and home-fitness buyers aged 25-50",
        keyword: "treadmill",
        call_to_action: "Bring home a KingSmith walking pad",
        tone_of_voice: "practical",
        cues: &[
            "kingsmith",
            "treadmill",
            "walking pad",
            "walkingpad",
            "fitness",
        ],
    },
    CampaignCategory {
        name: "JOMOO Bathroom Fixtures Showcase",
        product_prompt: "Showcase JOMOO premium bathroom and kitchen fixtures.",
        target_audience: "Homeowners and renovators aged 30-55",
        keyword: "bathroom fixtures",
        call_to_action: "Upgrade with JOMOO",
        tone_of_voice: "elegant",
        cues: &["jomoo", "bathroom", "faucet", "fixture", "kitchen", "sink"],
    },
];

/// Pick the nearest showcase category by scanning the draft for keyword cues.
/// Defaults to the first category when nothing matches.
fn pick_campaign_category(draft: &DraftConfig) -> &'static CampaignCategory {
    let haystack = draft_text(draft).to_lowercase();
    for cat in CAMPAIGN_CATEGORIES {
        if cat.cues.iter().any(|cue| haystack.contains(cue)) {
            return cat;
        }
    }
    &CAMPAIGN_CATEGORIES[0]
}

/// Concatenate the draft's string-valued fields into a single searchable blob.
fn draft_text(draft: &DraftConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (k, v) in draft.0.iter() {
        parts.push(k.clone());
        if let Some(s) = v.as_str() {
            parts.push(s.to_string());
        }
    }
    parts.join(" ")
}

/// Build the campaign preset value map from the nearest showcase category.
fn campaign_values(draft: &DraftConfig) -> BTreeMap<&'static str, Value> {
    let cat = pick_campaign_category(draft);
    let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
    // Required (7)
    m.insert("name", json!(cat.name));
    m.insert("platform_id", json!(1));
    m.insert("region_id", json!(1));
    m.insert("ai_model_id", json!(1));
    m.insert("schedule_type", json!("immediate"));
    m.insert("product_prompt", json!(cat.product_prompt));
    m.insert("max_scan_count", json!(100));
    // Recommended (6)
    m.insert("target_audience", json!(cat.target_audience));
    m.insert("keyword", json!(cat.keyword));
    m.insert("call_to_action", json!(cat.call_to_action));
    m.insert("tone_of_voice", json!(cat.tone_of_voice));
    m.insert("budget_cap", json!(500));
    m.insert("reply_template_ids", json!([1, 2]));
    m
}

/// Build the aipub preset value map for a known `plan_type`.
fn publish_plan_values(draft: &DraftConfig, plan_type: &str) -> BTreeMap<&'static str, Value> {
    // content_type is driven by the draft when supplied, else by the plan_type.
    let content_type = draft
        .get("content_type")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| default_content_type(plan_type).to_string());

    let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();
    // Always Required (2)
    m.insert("platform_id", json!(1));
    m.insert("content_type", json!(content_type));

    // Conditional Required keys, mirroring `publish_plan_fields` in task_spec.
    // Populate every key that *could* be required for any known plan_type so the
    // map is a superset; `build_template` emits only the keys the spec lists.
    m.insert("group_id", json!(1));
    m.insert("social_account_id", json!(1));
    m.insert("video_ai_model_id", json!(1));
    m.insert(
        "content",
        json!("Check out our latest update — link in bio."),
    );

    // Recommended (5)
    m.insert("name", json!(format!("Sample {plan_type} plan")));
    m.insert("plan_type", json!(plan_type));
    m.insert(
        "ai_input.content_prompt",
        json!("Write an upbeat launch post highlighting the product's key benefit."),
    );
    m.insert("schedule", json!({ "type": "immediate" }));
    m.insert(
        "behavior",
        json!({ "auto_like": false, "auto_follow": false }),
    );

    m
}

/// The content_type that pairs naturally with a plan_type when the draft is silent.
fn default_content_type(plan_type: &str) -> &'static str {
    match plan_type {
        "single_video" => "video",
        "reddit_image" => "image",
        "reddit_link" => "link",
        _ => "text",
    }
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
