//! Integration tests for the sample-template generator core (Module B1).
//!
//! These drive the public [`generate_sample`] entrypoint through a stateful
//! [`FakeLlm`] that returns a scripted queue of completion results and records
//! every (system, user) prompt it is asked. They encode the resolve → prompt →
//! complete → merge → validate → repair → fallback contract and must FAIL against
//! the deliberately-wrong skeleton stub (RED), then pass once the real generator
//! is implemented.
//!
//! HTTP / wiremock / real-DeepSeek coverage and `llm_client.rs` changes belong to
//! a SEPARATE unit (B2) and are intentionally NOT written here.

use std::collections::VecDeque;
use std::sync::Mutex;

use glance_mind_api::service::ai_chat::completeness::DraftConfig;
use glance_mind_api::service::ai_chat::task_spec::{Importance, TaskConfigSpec, TaskKind};
use glance_mind_api::service::ai_chat::task_template::generator::{
    generate_sample, GenerateError, LlmFailure, SampleLlm, SampleSource, SampleTemplate,
};
use glance_mind_api::service::ai_chat::task_template::validate::against_spec;
use serde_json::{json, Value};

/// A scripted, stateful fake LLM.
///
/// `responses` is a FIFO of results popped one per `complete_json` call;
/// `recorded` accumulates the (system, user) prompts in call order.
struct FakeLlm {
    responses: Mutex<VecDeque<Result<Value, LlmFailure>>>,
    recorded: Mutex<Vec<(String, String)>>,
}

impl FakeLlm {
    fn new(responses: Vec<Result<Value, LlmFailure>>) -> Self {
        FakeLlm {
            responses: Mutex::new(responses.into_iter().collect()),
            recorded: Mutex::new(Vec::new()),
        }
    }

    /// Number of times `complete_json` was invoked.
    fn call_count(&self) -> usize {
        self.recorded.lock().unwrap().len()
    }

    /// Snapshot of all recorded (system, user) prompts.
    fn prompts(&self) -> Vec<(String, String)> {
        self.recorded.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl SampleLlm for FakeLlm {
    async fn complete_json(
        &self,
        system: &str,
        user: &str,
        _max_tokens: u32,
    ) -> Result<Value, LlmFailure> {
        self.recorded
            .lock()
            .unwrap()
            .push((system.to_string(), user.to_string()));
        // Pop the next scripted response; once exhausted, behave like a generic
        // upstream error so an over-eager implementation cannot silently succeed.
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(LlmFailure::Other("no more scripted responses".into())))
    }
}

/// A campaign draft used as the user-supplied seed across tests.
fn campaign_draft() -> DraftConfig {
    DraftConfig::from_value(json!({
        "name": "User Draft Campaign",
        "platform_id": 1
    }))
}

/// A full, type-correct campaign config the fake LLM can return.
fn valid_campaign_config() -> Value {
    json!({
        "name": "AI Spring Launch",
        "platform_id": 1,
        "region_id": 2,
        "ai_model_id": 3,
        "schedule_type": "immediate",
        "product_prompt": "Promote the new running shoe to runners.",
        "max_scan_count": 100
    })
}

/// An invalid campaign config (region_id missing, platform_id wrong type).
fn invalid_campaign_config() -> Value {
    json!({
        "name": "Broken",
        "platform_id": "not-an-int",
        "ai_model_id": 3,
        "schedule_type": "immediate",
        "product_prompt": "x",
        "max_scan_count": 100
    })
}

/// Flatten a [`SampleTemplate`]'s fields into a config object (expanding dotted
/// keys into nested objects) for re-validation.
fn template_to_config(t: &SampleTemplate) -> Value {
    let mut root = serde_json::Map::new();
    for f in &t.fields {
        match f.key.split_once('.') {
            None => {
                root.insert(f.key.clone(), f.value.clone());
            }
            Some((head, rest)) => {
                let entry = root
                    .entry(head.to_string())
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
                if let Value::Object(obj) = entry {
                    obj.insert(rest.to_string(), f.value.clone());
                }
            }
        }
    }
    Value::Object(root)
}

/// Assert the template fills every Required key for `kind` given `draft`.
fn assert_all_required_filled(t: &SampleTemplate, kind: TaskKind, draft: &DraftConfig) {
    let spec = TaskConfigSpec::for_kind(kind, draft);
    for f in spec.fields.iter() {
        if matches!(f.importance, Importance::Required) {
            let filled = t
                .fields
                .iter()
                .any(|sf| sf.key == f.key && !sf.value.is_null());
            assert!(
                filled,
                "generated template must fill Required key `{}`; fields = {:?}",
                f.key,
                t.fields.iter().map(|sf| &sf.key).collect::<Vec<_>>()
            );
        }
    }
}

/// Assert the rendered template passes against_spec for `kind`.
fn assert_template_valid(t: &SampleTemplate, kind: TaskKind) {
    let config = template_to_config(t);
    assert!(
        against_spec(&config, kind).is_ok(),
        "generated template must pass against_spec; config = {config}"
    );
}

// ---------------------------------------------------------------------------
// Success / merge path
// ---------------------------------------------------------------------------

/// A valid full LLM config yields source AiGenerated|Hybrid, validates, fills all
/// Required keys.
#[tokio::test]
async fn mock_generate_campaign_ai_source() {
    let llm = FakeLlm::new(vec![Ok(valid_campaign_config())]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("a valid LLM config must produce a template");

    assert!(
        matches!(
            template.source,
            SampleSource::AiGenerated | SampleSource::Hybrid
        ),
        "a successful LLM completion must be sourced AiGenerated or Hybrid, got {:?}",
        template.source
    );
    assert_eq!(template.task_kind, TaskKind::Campaign);
    assert_all_required_filled(&template, TaskKind::Campaign, &draft);
    assert_template_valid(&template, TaskKind::Campaign);
    assert_eq!(
        llm.call_count(),
        1,
        "a first-try-valid completion must call the LLM exactly once, got {}",
        llm.call_count()
    );
}

// ---------------------------------------------------------------------------
// Fallback paths (LLM failures) -> Preset
// ---------------------------------------------------------------------------

/// Err(Empty) ⇒ fall back to the preset (source Preset), still validates.
#[tokio::test]
async fn fallback_on_empty() {
    let llm = FakeLlm::new(vec![Err(LlmFailure::Empty)]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("an empty LLM result must fall back to the preset, not error");

    assert_eq!(
        template.source,
        SampleSource::Preset,
        "an empty LLM result must yield a Preset-sourced template"
    );
    assert_all_required_filled(&template, TaskKind::Campaign, &draft);
    assert_template_valid(&template, TaskKind::Campaign);
}

/// Err(Malformed) ⇒ preset fallback.
#[tokio::test]
async fn fallback_on_malformed_json() {
    let llm = FakeLlm::new(vec![Err(LlmFailure::Malformed("not json".into()))]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("malformed LLM JSON must fall back to the preset");

    assert_eq!(
        template.source,
        SampleSource::Preset,
        "malformed LLM JSON must yield a Preset-sourced template"
    );
    assert_template_valid(&template, TaskKind::Campaign);
}

/// Err(Upstream(4xx)) ⇒ preset fallback.
#[tokio::test]
async fn fallback_on_upstream_4xx() {
    let llm = FakeLlm::new(vec![Err(LlmFailure::Upstream(403))]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("an upstream 4xx must fall back to the preset");

    assert_eq!(
        template.source,
        SampleSource::Preset,
        "an upstream 4xx must yield a Preset-sourced template"
    );
    assert_template_valid(&template, TaskKind::Campaign);
}

/// Err(Timeout) ⇒ preset fallback.
#[tokio::test]
async fn fallback_on_timeout() {
    let llm = FakeLlm::new(vec![Err(LlmFailure::Timeout)]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("a timeout must fall back to the preset");

    assert_eq!(
        template.source,
        SampleSource::Preset,
        "a timeout must yield a Preset-sourced template"
    );
    assert_template_valid(&template, TaskKind::Campaign);
}

// ---------------------------------------------------------------------------
// Repair path
// ---------------------------------------------------------------------------

/// First completion invalid, second valid ⇒ exactly 2 calls, source
/// AiGenerated|Hybrid, validates.
#[tokio::test]
async fn repair_then_accept() {
    let llm = FakeLlm::new(vec![
        Ok(invalid_campaign_config()),
        Ok(valid_campaign_config()),
    ]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("a repaired-then-valid completion must produce a template");

    assert_eq!(
        llm.call_count(),
        2,
        "an invalid-then-valid completion must invoke the LLM exactly twice (one repair), got {}",
        llm.call_count()
    );
    assert!(
        matches!(
            template.source,
            SampleSource::AiGenerated | SampleSource::Hybrid
        ),
        "a successful repair must be sourced AiGenerated or Hybrid, got {:?}",
        template.source
    );
    assert_all_required_filled(&template, TaskKind::Campaign, &draft);
    assert_template_valid(&template, TaskKind::Campaign);
}

/// First completion invalid, repair also invalid ⇒ preset fallback (source
/// Preset), exactly 2 calls.
#[tokio::test]
async fn fallback_on_invalid_after_repair() {
    let llm = FakeLlm::new(vec![
        Ok(invalid_campaign_config()),
        Ok(invalid_campaign_config()),
    ]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("an invalid-after-repair completion must fall back to the preset");

    assert_eq!(
        llm.call_count(),
        2,
        "exactly one repair attempt must be made before falling back, got {} calls",
        llm.call_count()
    );
    assert_eq!(
        template.source,
        SampleSource::Preset,
        "an invalid-after-repair completion must yield a Preset-sourced template"
    );
    assert_template_valid(&template, TaskKind::Campaign);
}

// ---------------------------------------------------------------------------
// Rate-limit retry path
// ---------------------------------------------------------------------------

/// Err(RateLimit) twice ⇒ exactly one bounded retry, then preset fallback.
#[tokio::test]
async fn retry_then_fallback_on_429() {
    let llm = FakeLlm::new(vec![Err(LlmFailure::RateLimit), Err(LlmFailure::RateLimit)]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("a persistent 429 must fall back to the preset, not error");

    assert_eq!(
        llm.call_count(),
        2,
        "a 429 must trigger exactly one retry (2 calls total) before fallback, got {}",
        llm.call_count()
    );
    assert_eq!(
        template.source,
        SampleSource::Preset,
        "a persistent 429 must yield a Preset-sourced template"
    );
    assert_template_valid(&template, TaskKind::Campaign);
}

// ---------------------------------------------------------------------------
// Provider drift
// ---------------------------------------------------------------------------

/// Valid JSON carrying unknown extra keys is still parsed + validates (lenient
/// to provider drift).
#[tokio::test]
async fn provider_drift_extra_fields() {
    let mut cfg = valid_campaign_config();
    if let Value::Object(map) = &mut cfg {
        map.insert("__unexpected_provider_field".into(), json!("ignore me"));
        map.insert("nested_garbage".into(), json!({ "a": [1, 2, 3] }));
    }
    let llm = FakeLlm::new(vec![Ok(cfg)]);
    let draft = campaign_draft();

    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("valid JSON with unknown keys must still produce a template");

    assert!(
        matches!(
            template.source,
            SampleSource::AiGenerated | SampleSource::Hybrid
        ),
        "provider-drift JSON that is otherwise valid must be AI-sourced, got {:?}",
        template.source
    );
    assert_all_required_filled(&template, TaskKind::Campaign, &draft);
    assert_template_valid(&template, TaskKind::Campaign);
}

// ---------------------------------------------------------------------------
// Prompt shape
// ---------------------------------------------------------------------------

/// A recorded prompt must contain the literal word "json" (DeepSeek JSON-mode
/// requirement). Checks both system and user across the recorded prompts.
#[tokio::test]
async fn prompt_contains_json_literal() {
    let llm = FakeLlm::new(vec![Ok(valid_campaign_config())]);
    let draft = campaign_draft();

    let _ = generate_sample(TaskKind::Campaign, &draft, &llm).await;

    let prompts = llm.prompts();
    assert!(
        !prompts.is_empty(),
        "generate_sample must call the LLM and record at least one prompt"
    );
    let mentions_json = prompts.iter().any(|(system, user)| {
        system.to_lowercase().contains("json") || user.to_lowercase().contains("json")
    });
    assert!(
        mentions_json,
        "a recorded system/user prompt must contain the literal word \"json\"; prompts = {prompts:?}"
    );
}

// ---------------------------------------------------------------------------
// Secret hygiene (by signature)
// ---------------------------------------------------------------------------

/// The generator/trait API must NOT take an API key parameter — secrets come
/// only from the SampleLlm impl/env. We assert this structurally by binding the
/// exact no-key signatures; if a key parameter were added, this would not
/// compile.
#[tokio::test]
async fn secret_never_from_payload() {
    // generate_sample(kind, &DraftConfig, &dyn SampleLlm) -> Result<..>, no key.
    let _gen: for<'a> fn(
        TaskKind,
        &'a DraftConfig,
        &'a dyn SampleLlm,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SampleTemplate, GenerateError>> + 'a>,
    > = |k, d, l| Box::pin(generate_sample(k, d, l));

    // Compile-time witness that complete_json's only inputs are prompts + a
    // numeric budget — no &str key, no token/credential argument. Calling it
    // positionally with exactly (system, user, max_tokens) would fail to compile
    // if an extra credential parameter were added to the trait method.
    fn assert_complete_json_signature<'a, T: SampleLlm>(
        this: &'a T,
        system: &'a str,
        user: &'a str,
        max_tokens: u32,
    ) -> impl std::future::Future<Output = Result<Value, LlmFailure>> + Send + 'a {
        this.complete_json(system, user, max_tokens)
    }
    {
        let probe = FakeLlm::new(vec![]);
        // Construct (do not poll) the future to bind the signature at compile time.
        let _fut = assert_complete_json_signature(&probe, "sys", "usr", 8u32);
    }

    // Behavioral witness: the FakeLlm is constructed with zero credentials and the
    // generator still produces a valid template.
    let llm = FakeLlm::new(vec![Ok(valid_campaign_config())]);
    let draft = campaign_draft();
    let template = generate_sample(TaskKind::Campaign, &draft, &llm)
        .await
        .expect("generator must work without any API key threaded through it");
    assert_template_valid(&template, TaskKind::Campaign);
    let _ = _gen; // silence unused in case of future refactor
}

// ---------------------------------------------------------------------------
// No preset -> TemplateUnavailable
// ---------------------------------------------------------------------------

/// An unsupported combo (no preset) ⇒ Err(TemplateUnavailable).
#[tokio::test]
async fn no_preset_returns_template_unavailable() {
    // Unknown aipub plan_type ⇒ presets::resolve returns None ⇒ generate errors.
    let draft = DraftConfig::from_value(json!({ "plan_type": "__nonexistent_plan_type__" }));
    let llm = FakeLlm::new(vec![Ok(valid_campaign_config())]);

    let result = generate_sample(TaskKind::PublishPlan, &draft, &llm).await;
    assert!(
        matches!(result, Err(GenerateError::TemplateUnavailable)),
        "an unsupported combo with no preset must return Err(TemplateUnavailable), got {:?}",
        result.map(|t| t.fields.len())
    );
    assert_eq!(
        llm.call_count(),
        0,
        "the generator must short-circuit on a missing preset before calling the LLM, got {} calls",
        llm.call_count()
    );
}

// ---------------------------------------------------------------------------
// Invariant: the returned template ALWAYS validates
// ---------------------------------------------------------------------------

/// Across a spread of LLM outcomes, the returned template always passes
/// against_spec and fills every Required key.
#[tokio::test]
async fn generated_template_always_validates() {
    let outcomes: Vec<Vec<Result<Value, LlmFailure>>> = vec![
        vec![Ok(valid_campaign_config())],                            // ai
        vec![Err(LlmFailure::Empty)],                                 // preset
        vec![Err(LlmFailure::Malformed("x".into()))],                 // preset
        vec![Err(LlmFailure::Upstream(500))],                         // preset
        vec![Err(LlmFailure::Timeout)],                               // preset
        vec![Err(LlmFailure::RateLimit), Err(LlmFailure::RateLimit)], // retry -> preset
        vec![Ok(invalid_campaign_config()), Ok(valid_campaign_config())], // repair -> ai
        vec![Ok(invalid_campaign_config()), Ok(invalid_campaign_config())], // repair fail -> preset
    ];

    for (i, responses) in outcomes.into_iter().enumerate() {
        let llm = FakeLlm::new(responses);
        let draft = campaign_draft();
        let template = generate_sample(TaskKind::Campaign, &draft, &llm)
            .await
            .unwrap_or_else(|e| panic!("outcome #{i} must produce a template, got {e:?}"));
        assert_all_required_filled(&template, TaskKind::Campaign, &draft);
        assert_template_valid(&template, TaskKind::Campaign);
    }
}
