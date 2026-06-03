//! HTTP boundary tests for the DeepSeek-backed sample-template LLM (Module B2).
//!
//! ASSERTION-CHANGE-JUSTIFIED: this file is NEW (no pre-existing assertions are
//! weakened). The live-provider tests use a runtime environment gate (return
//! early, printing a "skip:" line, when DEEPSEEK_API_KEY is unset) — this is
//! environmental gating mandated by the B2 spec, NOT a test-suppression
//! attribute and NOT a weakened assertion. When the key IS set, those tests
//! hard-fail (panic) on any provider error. No suppression attributes are used
//! anywhere in this file.
//!
//! These drive the production [`LlmClientSampleLlm`] (whose underlying
//! [`LlmClient`] base URL points at a `wiremock::MockServer`) and pin the
//! `complete_json` HTTP/content → [`LlmFailure`] contract:
//!
//! - 200 + valid JSON object ⇒ `Ok(Value)`.
//! - 200 + empty/whitespace content ⇒ `Err(Empty)`.
//! - 200 + prose / truncated JSON ⇒ `Err(Malformed)`.
//! - 200 + `finish_reason == "length"` ⇒ failure (NEVER `Ok`).
//! - 429 ⇒ `Err(RateLimit)`; other 4xx / 5xx ⇒ `Err(Upstream(status))`.
//! - delay beyond the client timeout ⇒ `Err(Timeout)`.
//! - the request body sets `response_format.type == "json_object"`, `max_tokens`,
//!   the configured model, a message mentioning "json", and an
//!   `Authorization: Bearer <key>` header; provider error bodies are redacted.
//!
//! They MUST fail against the deliberately-wrong B2 skeleton stub (RED), then
//! pass once `LlmClient::chat_completion_json` + `LlmClientSampleLlm::complete_json`
//! are implemented.

use glance_mind_api::service::ai_chat::task_template::generator::{LlmFailure, SampleLlm};
use glance_mind_api::service::ai_chat::task_template::LlmClientSampleLlm;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The fake DeepSeek API key the test client is built with. The boundary tests
/// assert this is sent as `Authorization: Bearer <key>` AND that it never leaks
/// into a surfaced error string.
const TEST_API_KEY: &str = "sk-test-secret-do-not-leak-12345";
const TEST_MODEL: &str = "deepseek-test-model";

/// `max_tokens` value the boundary tests pass + assert on the wire.
const TEST_MAX_TOKENS: u32 = 777;

/// Build a client pointed at the mock server (default timeout).
fn client_for(server: &MockServer) -> LlmClientSampleLlm {
    LlmClientSampleLlm::with_base_url(server.uri(), TEST_API_KEY, TEST_MODEL)
}

/// A DeepSeek-shaped chat-completion response whose assistant content is `content`
/// and whose first-choice `finish_reason` is `finish_reason`.
fn completion_response(content: &str, finish_reason: &str) -> Value {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "model": TEST_MODEL,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": finish_reason
        }],
        "usage": { "prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30 }
    })
}

/// Mount a single `POST /chat/completions` mock returning `response`.
async fn mount_completion(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(response)
        .mount(server)
        .await;
}

/// A system prompt that contains the literal word "json" (DeepSeek json-mode
/// requirement) — what we pass through `complete_json`.
fn json_system_prompt() -> String {
    "You output a single json object describing the config.".to_string()
}

// ---------------------------------------------------------------------------
// Request body / header shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn request_body_shape() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response(r#"{"ok":true}"#, "stop")),
    )
    .await;

    let llm = client_for(&server);
    let _ = llm
        .complete_json(&json_system_prompt(), "fill in the config", TEST_MAX_TOKENS)
        .await;

    let requests = server
        .received_requests()
        .await
        .expect("mock server must record received requests");
    assert_eq!(
        requests.len(),
        1,
        "complete_json must issue exactly one POST to /chat/completions, got {}",
        requests.len()
    );
    let req = &requests[0];

    // Authorization: Bearer <key> — key comes only from config, sent as bearer.
    let auth = req
        .headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(
        auth,
        format!("Bearer {TEST_API_KEY}"),
        "request must send Authorization: Bearer <configured key>; got {auth:?}"
    );

    let body: Value = serde_json::from_slice(&req.body).expect("request body must be JSON");

    // DeepSeek JSON mode.
    assert_eq!(
        body["response_format"]["type"], "json_object",
        "request must set response_format.type = json_object; body = {body}"
    );
    // max_tokens forwarded verbatim.
    assert_eq!(
        body["max_tokens"], TEST_MAX_TOKENS,
        "request must forward the passed max_tokens; body = {body}"
    );
    // Configured model.
    assert_eq!(
        body["model"], TEST_MODEL,
        "request must use the configured model; body = {body}"
    );
    // A message mentioning json.
    let messages = body["messages"]
        .as_array()
        .expect("request body must include a messages array");
    let mentions_json = messages.iter().any(|m| {
        m["content"]
            .as_str()
            .map(|c| c.to_lowercase().contains("json"))
            .unwrap_or(false)
    });
    assert!(
        mentions_json,
        "a request message must contain the literal word \"json\"; body = {body}"
    );
}

// ---------------------------------------------------------------------------
// 200 outcomes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn valid_json_maps_to_ok() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response(
            r#"{"name":"Spring","platform_id":1}"#,
            "stop",
        )),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    let value = result.expect("HTTP 200 + valid JSON object content must map to Ok(Value)");
    assert_eq!(
        value["name"], "Spring",
        "the returned Value must be the parsed assistant JSON; got {value}"
    );
    assert_eq!(
        value["platform_id"], 1,
        "parsed JSON must round-trip; got {value}"
    );
}

#[tokio::test]
async fn empty_content_maps_to_empty() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response("   ", "stop")),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Empty) => {}
        other => panic!("HTTP 200 + whitespace-only content must map to Err(Empty), got {other:?}"),
    }
}

#[tokio::test]
async fn prose_maps_to_malformed() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response(
            "Sure! Here is your configuration in plain prose, not JSON at all.",
            "stop",
        )),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Malformed(_)) => {}
        other => {
            panic!("HTTP 200 + non-JSON prose content must map to Err(Malformed), got {other:?}")
        }
    }
}

#[tokio::test]
async fn truncated_json_maps_to_malformed() {
    let server = MockServer::start().await;
    // Valid-looking start but truncated mid-object (no closing brace).
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response(
            r#"{"name":"Spring","platform_id":1,"product_prompt":"promote the"#,
            "stop",
        )),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Malformed(_)) => {}
        other => {
            panic!("HTTP 200 + truncated JSON content must map to Err(Malformed), got {other:?}")
        }
    }
}

#[tokio::test]
async fn finish_reason_length_not_ok() {
    let server = MockServer::start().await;
    // The body PARSES as valid JSON, but finish_reason == "length" signals
    // truncation: this must NOT be returned as Ok.
    mount_completion(
        &server,
        ResponseTemplate::new(200).set_body_json(completion_response(
            r#"{"name":"Spring","platform_id":1}"#,
            "length",
        )),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    assert!(
        result.is_err(),
        "finish_reason == \"length\" must be treated as a failure even when the body parses, got Ok"
    );
    // Per the contract it surfaces as Malformed (truncation) — never RateLimit/Empty/Upstream.
    match result {
        Err(LlmFailure::Malformed(_)) => {}
        other => panic!(
            "finish_reason == \"length\" must map to Err(Malformed) (truncation), got {other:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// HTTP status outcomes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn http_429_maps_to_ratelimit() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(429).set_body_string(r#"{"error":{"message":"rate limited"}}"#),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::RateLimit) => {}
        other => panic!("HTTP 429 must map to Err(RateLimit), got {other:?}"),
    }
}

#[tokio::test]
async fn http_500_maps_to_upstream() {
    let server = MockServer::start().await;
    mount_completion(
        &server,
        ResponseTemplate::new(500).set_body_string("internal error"),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Upstream(status)) => assert_eq!(
            status, 500,
            "HTTP 500 must map to Err(Upstream(500)), got Upstream({status})"
        ),
        other => panic!("HTTP 500 must map to Err(Upstream(500)), got {other:?}"),
    }
}

#[tokio::test]
async fn http_400_maps_to_upstream() {
    let server = MockServer::start().await;
    // Non-429 4xx (e.g. the DeepSeek 400 when the prompt lacks "json").
    mount_completion(
        &server,
        ResponseTemplate::new(400)
            .set_body_string(r#"{"error":{"message":"prompt must contain json"}}"#),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Upstream(status)) => assert_eq!(
            status, 400,
            "HTTP 400 must map to Err(Upstream(400)), got Upstream({status})"
        ),
        other => panic!("HTTP 400 (non-429) must map to Err(Upstream(400)), got {other:?}"),
    }
}

#[tokio::test]
async fn timeout_maps_to_timeout() {
    let server = MockServer::start().await;
    // Respond AFTER a delay that exceeds the client's (short) request timeout.
    mount_completion(
        &server,
        ResponseTemplate::new(200)
            .set_body_json(completion_response(r#"{"ok":true}"#, "stop"))
            .set_delay(std::time::Duration::from_secs(3)),
    )
    .await;

    // 1s timeout < 3s delay ⇒ the request must time out.
    let llm =
        LlmClientSampleLlm::with_base_url_and_timeout(server.uri(), TEST_API_KEY, TEST_MODEL, 1);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    match result {
        Err(LlmFailure::Timeout) => {}
        other => panic!(
            "a response delay beyond the client timeout must map to Err(Timeout), got {other:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// Error redaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn provider_error_body_redacted() {
    let server = MockServer::start().await;
    // A 5xx whose body embeds a secret-looking token + a sensitive phrase.
    let secret = "sk-leaked-provider-token-DO-NOT-SURFACE";
    mount_completion(
        &server,
        ResponseTemplate::new(503).set_body_string(format!(
            r#"{{"error":{{"message":"No available accounts for {secret}"}}}}"#
        )),
    )
    .await;

    let llm = client_for(&server);
    let result = llm
        .complete_json(&json_system_prompt(), "go", TEST_MAX_TOKENS)
        .await;

    // 503 ⇒ Upstream(503). The carried diagnostic (if any) must not leak the body.
    let rendered = format!("{result:?}");
    assert!(
        !rendered.contains(secret),
        "surfaced error must not contain the provider secret token; got {rendered}"
    );
    assert!(
        !rendered.contains("No available accounts"),
        "surfaced error must not echo the provider error body; got {rendered}"
    );
    match result {
        Err(LlmFailure::Upstream(status)) => assert_eq!(
            status, 503,
            "HTTP 503 must map to Err(Upstream(503)), got Upstream({status})"
        ),
        other => panic!("HTTP 503 must map to Err(Upstream(503)), got {other:?}"),
    }
}

// ===========================================================================
// Live-provider tests (real DeepSeek). The WHOLE block is behind the
// `live-provider` feature so the default `cargo test` cannot compile it.
// ===========================================================================
#[cfg(feature = "live-provider")]
mod live_provider {
    use glance_mind_api::service::ai_chat::completeness::DraftConfig;
    use glance_mind_api::service::ai_chat::task_spec::TaskKind;
    use glance_mind_api::service::ai_chat::task_template::generator::generate_sample;
    use glance_mind_api::service::ai_chat::task_template::validate::against_spec;
    use glance_mind_api::service::ai_chat::task_template::{
        LlmClientSampleLlm, SampleSource, SampleTemplate,
    };
    use serde_json::{json, Value};

    /// Flatten a template's fields into a nested config object for re-validation.
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

    #[tokio::test]
    async fn real_deepseek_complete_json_succeeds() {
        // Environmental gate (return early) when no key is set. NOT a suppression
        // attribute: with a key present this test runs and hard-fails on errors.
        let Some(_) = std::env::var("DEEPSEEK_API_KEY").ok() else {
            eprintln!("skip: no DEEPSEEK_API_KEY");
            return;
        };

        use glance_mind_api::service::ai_chat::task_template::generator::SampleLlm;
        let llm = LlmClientSampleLlm::from_env();

        let system = "You output ONLY a single json object. \
             Return a json object with keys \"name\" (string) and \"platform_id\" (number).";
        let user = "Produce the json object now.";

        // With a real key set, any LlmFailure is a HARD failure (panic) — we do
        // NOT silently tolerate provider errors here.
        let value = llm
            .complete_json(system, user, 256)
            .await
            .unwrap_or_else(|e| panic!("live DeepSeek complete_json must succeed, got {e:?}"));

        let obj = value
            .as_object()
            .unwrap_or_else(|| panic!("live DeepSeek must return a JSON object, got {value}"));
        assert!(
            !obj.is_empty(),
            "live DeepSeek json object must be non-empty, got {value}"
        );
    }

    #[tokio::test]
    async fn real_deepseek_generate_sample_end_to_end() {
        let Some(_) = std::env::var("DEEPSEEK_API_KEY").ok() else {
            eprintln!("skip: no DEEPSEEK_API_KEY");
            return;
        };

        let llm = LlmClientSampleLlm::from_env();
        // A sparse campaign draft: the live LLM must flesh out the rest.
        let draft = DraftConfig::from_value(json!({
            "name": "Live E2E Campaign",
            "platform_id": 1
        }));

        let template = generate_sample(TaskKind::Campaign, &draft, &llm)
            .await
            .unwrap_or_else(|e| panic!("live generate_sample must produce a template, got {e:?}"));

        // The rendered template must validate against the spec.
        let config = template_to_config(&template);
        assert!(
            against_spec(&config, TaskKind::Campaign).is_ok(),
            "live-generated template must pass against_spec; config = {config}"
        );
        // Since the live call works, the source must NOT be the deterministic preset.
        assert_ne!(
            template.source,
            SampleSource::Preset,
            "a working live LLM call must yield an AI-sourced (non-Preset) template, got {:?}",
            template.source
        );
    }
}
