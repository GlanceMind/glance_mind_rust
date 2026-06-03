//! Production [`SampleLlm`] backed by the real DeepSeek [`LlmClient`] (Module B2).
//!
//! [`LlmClientSampleLlm`] adapts the B1 [`SampleLlm`] trait onto a concrete
//! [`LlmClient`] call in DeepSeek JSON mode. It sends
//! `response_format: {"type":"json_object"}` + `max_tokens` + the configured
//! model, then maps the HTTP/content outcome onto a
//! `Result<serde_json::Value, LlmFailure>`:
//!
//! - HTTP 200 + valid JSON object content ⇒ `Ok(Value)`.
//! - HTTP 200 + empty / whitespace content ⇒ `Err(LlmFailure::Empty)`.
//! - HTTP 200 + non-JSON / truncated content ⇒ `Err(LlmFailure::Malformed(_))`.
//! - HTTP 200 + `finish_reason == "length"` ⇒ truncation ⇒
//!   `Err(LlmFailure::Malformed(_))` (NEVER `Ok`).
//! - HTTP 429 ⇒ `Err(LlmFailure::RateLimit)`.
//! - HTTP 4xx (non-429) / 5xx ⇒ `Err(LlmFailure::Upstream(status))`.
//! - timeout ⇒ `Err(LlmFailure::Timeout)`.
//!
//! The API key is supplied ONLY via the underlying [`LlmClient`] (config/env);
//! it is never threaded through the [`SampleLlm`] API or a method/payload arg.

use serde_json::Value;

use super::super::llm_client::LlmClient;
use super::generator::{LlmFailure, SampleLlm};

/// A [`SampleLlm`] implementation that calls the real DeepSeek [`LlmClient`] in
/// JSON mode.
#[derive(Debug, Clone)]
pub struct LlmClientSampleLlm {
    client: LlmClient,
}

impl LlmClientSampleLlm {
    /// Wrap an already-configured [`LlmClient`].
    pub fn new(client: LlmClient) -> Self {
        Self { client }
    }

    /// Build from the environment (real DeepSeek config). Used by the live
    /// provider tests / production wiring.
    pub fn from_env() -> Self {
        Self::new(LlmClient::new())
    }

    /// Build pointed at an explicit base URL (for tests intercepting the HTTP
    /// call, e.g. wiremock). Default request timeout.
    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self::new(LlmClient::with_base_url(base_url, api_key, model))
    }

    /// Like [`LlmClientSampleLlm::with_base_url`] but with an explicit request
    /// timeout (seconds) so tests can force a short budget and exercise the
    /// timeout path.
    pub fn with_base_url_and_timeout(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        request_timeout_secs: u64,
    ) -> Self {
        Self::new(LlmClient::with_base_url_and_timeout(
            base_url,
            api_key,
            model,
            request_timeout_secs,
        ))
    }
}

#[async_trait::async_trait]
impl SampleLlm for LlmClientSampleLlm {
    /// NOTE (B2 skeleton): this stub returns a fixed WRONG value and performs NO
    /// network I/O / outcome mapping, so the boundary tests fail by assertion
    /// until the implementer wires it to [`LlmClient::chat_completion_json`] and
    /// applies the documented HTTP/content → [`LlmFailure`] mapping.
    async fn complete_json(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<Value, LlmFailure> {
        let _ = (system, user, max_tokens, &self.client);
        Ok(serde_json::json!({ "__stub_not_implemented": true }))
    }
}
