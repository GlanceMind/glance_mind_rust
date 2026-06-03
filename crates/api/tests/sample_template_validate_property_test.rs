//! Property tests for sample-template validation + serde (Module B1).
//!
//! These encode invariants that must hold for ALL inputs and are written against
//! the public API. They must FAIL against the deliberately-wrong skeleton stubs
//! (RED), then pass once the real logic is implemented.

use glance_mind_api::service::ai_chat::completeness::DraftConfig;
use glance_mind_api::service::ai_chat::task_spec::{
    FieldKind, Importance, TaskConfigSpec, TaskKind,
};
use glance_mind_api::service::ai_chat::task_template::generator::{
    EnumOption, SampleField, SampleSource, SampleTemplate,
};
use glance_mind_api::service::ai_chat::task_template::validate::against_spec;
use proptest::prelude::*;
use serde_json::{json, Value};

/// The seven campaign Required keys, with the JSON type each demands.
const CAMPAIGN_REQUIRED: &[(&str, FieldKind)] = &[
    ("name", FieldKind::String),
    ("platform_id", FieldKind::Int),
    ("region_id", FieldKind::Int),
    ("ai_model_id", FieldKind::Int),
    ("schedule_type", FieldKind::Enum),
    ("product_prompt", FieldKind::String),
    ("max_scan_count", FieldKind::Int),
];

/// Build a campaign config from per-key inclusion flags; included keys get a
/// type-correct value, others are omitted.
fn build_campaign_config(flags: &[bool]) -> Value {
    let mut map = serde_json::Map::new();
    for (i, (key, kind)) in CAMPAIGN_REQUIRED.iter().enumerate() {
        if flags.get(i).copied().unwrap_or(false) {
            let val = match kind {
                FieldKind::Int => json!(7),
                FieldKind::String => json!("a non-empty value"),
                FieldKind::Enum => json!("immediate"),
                FieldKind::Bool => json!(true),
                FieldKind::Json => json!({ "k": "v" }),
            };
            map.insert(key.to_string(), val);
        }
    }
    Value::Object(map)
}

fn flags_strategy() -> impl Strategy<Value = Vec<bool>> {
    proptest::collection::vec(any::<bool>(), CAMPAIGN_REQUIRED.len())
}

/// A SampleField with an arbitrary-but-typed shape for serde round-trip checks.
fn sample_field_strategy() -> impl Strategy<Value = SampleField> {
    let key = "[a-z][a-z_]{0,12}";
    let label = ".{0,16}";
    let group = "[a-z]{1,8}";
    (
        key,
        label,
        group,
        prop_oneof![
            Just(Importance::Required),
            Just(Importance::Recommended),
            Just(Importance::Optional)
        ],
        prop_oneof![
            Just(FieldKind::String),
            Just(FieldKind::Int),
            Just(FieldKind::Enum),
            Just(FieldKind::Bool),
            Just(FieldKind::Json)
        ],
        any::<i64>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(key, label_cn, group, importance, field_type, num, editable, has_opts)| {
                let value = match field_type {
                    FieldKind::Int => json!(num),
                    FieldKind::String | FieldKind::Enum => json!(format!("v{num}")),
                    FieldKind::Bool => json!(num % 2 == 0),
                    FieldKind::Json => json!({ "n": num }),
                };
                let options = if has_opts && matches!(field_type, FieldKind::Enum) {
                    Some(vec![EnumOption {
                        value: json!(format!("opt{num}")),
                        label: format!("label {num}"),
                    }])
                } else {
                    None
                };
                SampleField {
                    key,
                    label_cn,
                    group,
                    importance,
                    field_type,
                    value,
                    editable,
                    options,
                }
            },
        )
}

fn sample_template_strategy() -> impl Strategy<Value = SampleTemplate> {
    (
        prop_oneof![Just(TaskKind::Campaign), Just(TaskKind::PublishPlan)],
        prop_oneof![
            Just(SampleSource::AiGenerated),
            Just(SampleSource::Preset),
            Just(SampleSource::Hybrid)
        ],
        proptest::collection::vec(sample_field_strategy(), 0..6),
    )
        .prop_map(|(task_kind, source, fields)| SampleTemplate {
            task_kind,
            source,
            fields,
        })
}

proptest! {
    /// If against_spec accepts a config, then every Required key (after
    /// conditional resolution) is present, non-null, and type-correct.
    #[test]
    fn prop_validated_config_satisfies_spec(flags in flags_strategy()) {
        let config = build_campaign_config(&flags);

        if against_spec(&config, TaskKind::Campaign).is_ok() {
            // The accept verdict implies a fully-typed Required set. Derive the
            // Required spec independently and check each key in the config.
            let spec = TaskConfigSpec::for_kind(
                TaskKind::Campaign,
                &DraftConfig::from_value(config.clone()),
            );
            let obj = config.as_object().expect("config is an object");

            for f in spec.fields.iter() {
                if !matches!(f.importance, Importance::Required) {
                    continue;
                }
                let v = obj.get(f.key);
                prop_assert!(
                    v.is_some_and(|x| !x.is_null()),
                    "against_spec accepted a config missing Required key `{}`: {}",
                    f.key,
                    config
                );
                let v = v.unwrap();
                let type_ok = match f.kind {
                    FieldKind::Int => v.is_i64() || v.is_u64(),
                    FieldKind::String | FieldKind::Enum => v.is_string(),
                    FieldKind::Bool => v.is_boolean(),
                    FieldKind::Json => v.is_object() || v.is_array(),
                };
                prop_assert!(
                    type_ok,
                    "against_spec accepted a config with wrong-typed Required key `{}` = {} (expected {:?})",
                    f.key,
                    v,
                    f.kind
                );
            }
        }
    }

    /// SampleTemplate survives a JSON serialize → deserialize round-trip
    /// unchanged, including the renamed `type` wire key for each field.
    #[test]
    fn prop_roundtrip_sample_template_json(template in sample_template_strategy()) {
        let wire = serde_json::to_value(&template)
            .expect("SampleTemplate must serialize to JSON");

        // The field-type wire key must be `type`, not `field_type`.
        if let Some(fields) = wire.get("fields").and_then(|f| f.as_array()) {
            for (i, f) in fields.iter().enumerate() {
                prop_assert!(
                    f.get("type").is_some(),
                    "field #{i} must serialize its kind under the `type` wire key: {f}"
                );
                prop_assert!(
                    f.get("field_type").is_none(),
                    "field #{i} must NOT expose `field_type` on the wire: {f}"
                );
            }
        }

        let back: SampleTemplate = serde_json::from_value(wire.clone())
            .expect("SampleTemplate must deserialize back from its own JSON");

        // Re-serializing the round-tripped value must be byte-identical.
        let wire2 = serde_json::to_value(&back)
            .expect("round-tripped SampleTemplate must re-serialize");
        prop_assert_eq!(
            wire,
            wire2,
            "SampleTemplate JSON round-trip must be identity"
        );
    }
}
