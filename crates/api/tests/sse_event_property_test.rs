//! Module D1 — RED tests for the task-template (sample-template) SSE events.
//!
//! These exercise the two new [`SseEvent`] variants
//! (`TaskTemplateGenerating` / `TaskTemplateProposed`) and the secret-redacting
//! construction path. They drive `to_sse_string()` directly and parse the
//! emitted `event:` / `data:` lines back into `serde_json::Value`, comparing
//! structurally (key-order-insensitive) against the committed canonical fixture
//! `tests/fixtures/sample_template/task_template_proposed.json`.
//!
//! They MUST compile and FAIL ON ASSERTIONS against the deliberately-wrong
//! skeleton (wrong `task_template_generating` event name; a
//! `task_template_proposed` payload that omits `fields` and skips redaction; a
//! `redact_secret_values` stub that returns its input unchanged). They turn
//! GREEN only once the implementer wires the correct emit + real redaction.
//! The implementer MUST NOT weaken these assertions.
//!
//! Run:
//!   cargo test -p glance_mind_api --test sse_event_property_test

use glance_mind_api::service::ai_chat::task_spec::{FieldKind, Importance, TaskKind};
use glance_mind_api::service::ai_chat::task_template::generator::{
    EnumOption, SampleField, SampleSource,
};
use glance_mind_api::service::ai_chat::types::{SseEvent, SENSITIVE_FIELDS};
use proptest::prelude::*;
use serde_json::{json, Value};
use uuid::Uuid;
// ASSERTION-CHANGE-JUSTIFIED: Removing the WHOLE temporary scaffolding test
// `temp_verify_fixture_is_faithful_serialization`. It was a throwaway meta-check
// that the committed fixture equals the faithful serde serialization of
// `fixture_fields()`; it passed (proving the fixture is the correct GREEN target)
// and is intentionally not part of the shipped RED suite. No RED assertion of the
// task-template SSE behavior is weakened — those live in the four tests below.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The fixed draft id baked into the canonical fixture.
const FIXTURE_DRAFT_ID: &str = "11111111-1111-4111-8111-111111111111";

/// Load + parse the committed canonical fixture as a JSON value.
fn fixture_value() -> Value {
    let raw = include_str!("fixtures/sample_template/task_template_proposed.json");
    serde_json::from_str(raw).expect("canonical fixture must be valid JSON")
}

/// Split a single SSE block into (event_name, data_json_value).
///
/// Expects exactly the shape `to_sse_string()` emits:
/// `event: <name>\ndata: <json>\n\n`. Panics with a descriptive message if the
/// block is not parseable so RED failures point at the real defect.
fn parse_sse_block(block: &str) -> (String, Value) {
    let event_line = block
        .lines()
        .find(|l| l.starts_with("event: "))
        .unwrap_or_else(|| panic!("emitted SSE block has no `event:` line: {block:?}"));
    let event_name = event_line
        .strip_prefix("event: ")
        .expect("event line prefix")
        .to_string();

    let data_line = block
        .lines()
        .find(|l| l.starts_with("data: "))
        .unwrap_or_else(|| panic!("emitted SSE block has no `data:` line: {block:?}"));
    let data_json = data_line.strip_prefix("data: ").expect("data line prefix");
    let data: Value = serde_json::from_str(data_json)
        .unwrap_or_else(|e| panic!("`data:` line is not valid JSON ({e}): {data_json:?}"));

    (event_name, data)
}

/// A non-enum `SampleField` (its `options` serialize to `null`).
fn field(
    key: &str,
    label_cn: &str,
    group: &str,
    importance: Importance,
    kind: FieldKind,
    value: Value,
) -> SampleField {
    SampleField {
        key: key.to_string(),
        label_cn: label_cn.to_string(),
        group: group.to_string(),
        importance,
        field_type: kind,
        value,
        editable: true,
        options: None,
    }
}

/// The exact `fields` vector that the canonical fixture encodes, in fixture order.
///
/// Built from typed `SampleField`s so the emitted `to_sse_string()` payload and
/// the committed fixture describe the SAME representative campaign proposal.
fn fixture_fields() -> Vec<SampleField> {
    use FieldKind::*;
    use Importance::*;
    vec![
        field(
            "name",
            "活动名称",
            "basic",
            Required,
            String,
            json!("春季潮鞋推广"),
        ),
        field("platform_id", "平台", "basic", Required, Int, json!(2)),
        field("region_id", "地区", "basic", Required, Int, json!(1)),
        field("ai_model_id", "AI 模型", "ai", Required, Int, json!(3)),
        SampleField {
            key: "schedule_type".to_string(),
            label_cn: "调度类型".to_string(),
            group: "schedule".to_string(),
            importance: Required,
            field_type: Enum,
            value: json!("immediate"),
            editable: true,
            options: Some(vec![
                EnumOption {
                    value: json!("immediate"),
                    label: "立即执行".to_string(),
                },
                EnumOption {
                    value: json!("scheduled"),
                    label: "定时执行".to_string(),
                },
                EnumOption {
                    value: json!("recurring"),
                    label: "周期执行".to_string(),
                },
            ]),
        },
        field(
            "product_prompt",
            "产品提示词",
            "ai",
            Required,
            String,
            json!("面向年轻人的潮流运动鞋，主打舒适与设计感。"),
        ),
        field(
            "max_scan_count",
            "最大扫描数",
            "schedule",
            Required,
            Int,
            json!(5),
        ),
        field(
            "target_audience",
            "目标受众",
            "ai",
            Recommended,
            String,
            json!("18-30 岁喜欢潮流文化的年轻人"),
        ),
        field(
            "keyword",
            "关键词",
            "targeting",
            Recommended,
            String,
            json!("sneakers, 潮鞋, trendy shoes"),
        ),
        field(
            "call_to_action",
            "行动号召",
            "ai",
            Recommended,
            String,
            json!("点击主页链接抢先选购"),
        ),
        field(
            "tone_of_voice",
            "语气风格",
            "ai",
            Recommended,
            String,
            json!("年轻、活力、真诚"),
        ),
        field(
            "budget_cap",
            "预算上限",
            "budget",
            Recommended,
            Int,
            json!(200),
        ),
        field(
            "reply_template_ids",
            "回复模板",
            "reply",
            Recommended,
            Json,
            json!([]),
        ),
    ]
}

// ---------------------------------------------------------------------------
// 1. Fixture-exact wire shape for `task_template_proposed`.
// ---------------------------------------------------------------------------

#[test]
fn task_template_proposed_to_sse_string_matches_fixture() {
    let draft_id = Uuid::parse_str(FIXTURE_DRAFT_ID).unwrap();
    // Build THROUGH the redacting construction path with the SAME content as the
    // committed fixture (no secrets present, so redaction is a no-op here).
    let event = SseEvent::task_template_proposed(
        draft_id,
        TaskKind::Campaign,
        SampleSource::AiGenerated,
        false,
        Vec::new(),
        fixture_fields(),
    );

    let block = event.to_sse_string();
    let (event_name, data) = parse_sse_block(&block);

    assert_eq!(
        event_name, "task_template_proposed",
        "wire event name must be exactly `task_template_proposed`"
    );

    let expected = fixture_value();
    assert_eq!(
        data, expected,
        "emitted `data` object must deep-equal the canonical fixture \
         (key ordering ignored); got {data:#}"
    );
}

// ---------------------------------------------------------------------------
// 2. `task_template_generating` wire shape.
// ---------------------------------------------------------------------------

#[test]
fn task_template_generating_to_sse_string_shape() {
    let draft_id = Uuid::parse_str(FIXTURE_DRAFT_ID).unwrap();
    let event = SseEvent::TaskTemplateGenerating {
        draft_id,
        task_kind: TaskKind::Campaign,
    };

    let block = event.to_sse_string();
    let (event_name, data) = parse_sse_block(&block);

    assert_eq!(
        event_name, "task_template_generating",
        "wire event name must be exactly `task_template_generating`"
    );
    assert_eq!(
        data,
        json!({ "draft_id": FIXTURE_DRAFT_ID, "task_kind": "campaign" }),
        "data must be exactly {{draft_id, task_kind}} with snake_case task_kind; got {data:#}"
    );
}

// ---------------------------------------------------------------------------
// Property strategies
// ---------------------------------------------------------------------------

fn arb_task_kind() -> impl Strategy<Value = TaskKind> {
    prop_oneof![Just(TaskKind::Campaign), Just(TaskKind::PublishPlan)]
}

fn arb_source() -> impl Strategy<Value = SampleSource> {
    prop_oneof![
        Just(SampleSource::AiGenerated),
        Just(SampleSource::Preset),
        Just(SampleSource::Hybrid),
    ]
}

fn arb_field_kind() -> impl Strategy<Value = FieldKind> {
    prop_oneof![
        Just(FieldKind::String),
        Just(FieldKind::Int),
        Just(FieldKind::Enum),
        Just(FieldKind::Bool),
        Just(FieldKind::Json),
    ]
}

fn arb_importance() -> impl Strategy<Value = Importance> {
    prop_oneof![
        Just(Importance::Required),
        Just(Importance::Recommended),
        Just(Importance::Optional),
    ]
}

/// Arbitrary benign (secret-free) `SampleField` for the roundtrip property.
fn arb_benign_field() -> impl Strategy<Value = SampleField> {
    (
        "[a-z][a-z0-9_]{0,15}",
        "[\\PC]{0,12}",
        "[a-z]{1,8}",
        arb_importance(),
        arb_field_kind(),
        "[a-z0-9 ]{0,20}",
        any::<bool>(),
    )
        .prop_map(
            |(key, label_cn, group, importance, field_type, value, editable)| SampleField {
                key,
                label_cn,
                group,
                importance,
                field_type,
                value: json!(value),
                editable,
                options: None,
            },
        )
}

/// FieldKind → its expected snake_case wire `type` string.
fn field_kind_wire(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::String => "string",
        FieldKind::Int => "int",
        FieldKind::Enum => "enum",
        FieldKind::Bool => "bool",
        FieldKind::Json => "json",
    }
}

// ---------------------------------------------------------------------------
// 3. Roundtrip: emitted `data:` JSON recovers the structural field content.
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_task_template_event_roundtrip(
        kind in arb_task_kind(),
        source in arb_source(),
        blocking in any::<bool>(),
        missing in proptest::collection::vec("[a-z_]{1,12}", 0..4),
        fields in proptest::collection::vec(arb_benign_field(), 0..5),
    ) {
        let draft_id = Uuid::new_v4();
        let event = SseEvent::TaskTemplateProposed {
            draft_id,
            task_kind: kind,
            sample_source: source,
            blocking,
            missing: missing.clone(),
            fields: fields.clone(),
        };

        let block = event.to_sse_string();
        let (event_name, data) = parse_sse_block(&block);

        prop_assert_eq!(event_name, "task_template_proposed");

        // Scalars recover verbatim.
        let draft_id_str = draft_id.to_string();
        prop_assert_eq!(
            data.get("draft_id").and_then(|v| v.as_str()),
            Some(draft_id_str.as_str()),
            "draft_id must round-trip as its uuid string"
        );
        prop_assert_eq!(
            data.get("blocking").and_then(|v| v.as_bool()),
            Some(blocking),
            "blocking must round-trip"
        );
        let recovered_missing: Vec<String> = data
            .get("missing")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        prop_assert_eq!(recovered_missing, missing, "missing list must round-trip");

        // The fields array must be present with the same length...
        let emitted_fields = data
            .get("fields")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        prop_assert_eq!(
            emitted_fields.len(),
            fields.len(),
            "emitted `fields` count must equal the input field count"
        );

        // ...and each field recovers key + the snake_case wire `type` discriminator.
        for (i, original) in fields.iter().enumerate() {
            let emitted = &emitted_fields[i];
            prop_assert_eq!(
                emitted.get("key").and_then(|v| v.as_str()),
                Some(original.key.as_str()),
                "field[{}] key must round-trip", i
            );
            prop_assert_eq!(
                emitted.get("type").and_then(|v| v.as_str()),
                Some(field_kind_wire(original.field_type)),
                "field[{}] must serialize its datatype under the wire key `type`", i
            );
            prop_assert_eq!(
                emitted.get("value"),
                Some(&original.value),
                "field[{}] value must round-trip", i
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4. No secret value survives the redacting emit path.
// ---------------------------------------------------------------------------

/// Recursively collect every JSON string anywhere in a value.
fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|v| collect_strings(v, out)),
        Value::Object(o) => o.values().for_each(|v| collect_strings(v, out)),
        _ => {}
    }
}

/// A field carrying a secret in its string `value`.
fn arb_secret_value_field() -> impl Strategy<Value = SampleField> {
    prop_oneof![
        // `sk-…` API-key shape embedded in an otherwise-benign value.
        "[a-z0-9]{8,16}".prop_map(|tail| format!("key sk-{tail}")),
        // `Bearer …` authorization shape.
        "[A-Za-z0-9]{6,20}".prop_map(|t| format!("Bearer {t}")),
        // `api_key: …` inline assignment shape.
        "[A-Za-z0-9]{8,20}".prop_map(|t| format!("api_key={t}")),
    ]
    .prop_map(|secret| SampleField {
        key: "value".to_string(),
        label_cn: "字段".to_string(),
        group: "ai".to_string(),
        importance: Importance::Recommended,
        field_type: FieldKind::String,
        value: json!(secret),
        editable: true,
        options: None,
    })
}

/// A field whose KEY is a sensitive name and whose value is a plain secret token.
fn arb_secret_keyed_field() -> impl Strategy<Value = SampleField> {
    (
        prop_oneof![
            Just("token"),
            Just("secret"),
            Just("authorization"),
            Just("password"),
            Just("api_key"),
        ],
        "[A-Za-z0-9]{8,24}",
    )
        .prop_map(|(key, tok)| SampleField {
            key: key.to_string(),
            label_cn: "敏感字段".to_string(),
            group: "secret".to_string(),
            importance: Importance::Recommended,
            field_type: FieldKind::String,
            value: json!(tok),
            editable: false,
            options: None,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_no_secret_value_survives_emit(
        secret_value_fields in proptest::collection::vec(arb_secret_value_field(), 1..3),
        secret_keyed_fields in proptest::collection::vec(arb_secret_keyed_field(), 1..3),
        benign in proptest::collection::vec(arb_benign_field(), 0..3),
    ) {
        // Interleave benign + both kinds of secret-bearing fields.
        let mut fields = Vec::new();
        fields.extend(benign);
        fields.extend(secret_value_fields.clone());
        fields.extend(secret_keyed_fields.clone());

        // Build THROUGH the redacting construction path (the only sanctioned way
        // to construct a proposed-template event).
        let event = SseEvent::task_template_proposed(
            Uuid::new_v4(),
            TaskKind::Campaign,
            SampleSource::Hybrid,
            false,
            Vec::new(),
            fields,
        );

        let block = event.to_sse_string();
        let (_event_name, data) = parse_sse_block(&block);

        // Gather every string that reached the wire.
        let mut emitted_strings = Vec::new();
        collect_strings(&data, &mut emitted_strings);

        // (a) No content-based secret shape may survive anywhere.
        for s in &emitted_strings {
            prop_assert!(
                !s.contains("sk-"),
                "an `sk-` API-key token survived to the wire: {:?}", s
            );
            prop_assert!(
                !s.contains("Bearer "),
                "a `Bearer …` token survived to the wire: {:?}", s
            );
            prop_assert!(
                !s.to_ascii_lowercase().contains("api_key="),
                "an inline `api_key=` secret survived to the wire: {:?}", s
            );
        }

        // (b) No raw value carried under a SENSITIVE_FIELDS-named key may survive
        //     verbatim. The construction path must mask those values.
        let emitted_fields = data
            .get("fields")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for ef in &emitted_fields {
            let key = ef.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let key_lc = key.to_ascii_lowercase();
            let is_sensitive_key = SENSITIVE_FIELDS
                .iter()
                .any(|f| f.eq_ignore_ascii_case(&key_lc));
            if is_sensitive_key {
                // Cross-check against the original secret tokens we injected under
                // sensitive keys: none may appear unredacted.
                for original in &secret_keyed_fields {
                    if original.key.eq_ignore_ascii_case(key) {
                        let raw = original.value.as_str().unwrap_or("");
                        let emitted_val = ef.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        prop_assert!(
                            !raw.is_empty() && emitted_val != raw,
                            "value under sensitive key `{}` survived unredacted: {:?}",
                            key, emitted_val
                        );
                    }
                }
            }
        }
    }
}
