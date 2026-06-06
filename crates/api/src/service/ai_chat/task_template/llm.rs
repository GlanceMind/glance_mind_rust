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

use super::super::llm_client::{JsonChatError, LlmClient};
use super::generator::{LlmFailure, SampleLlm};

/// HTTP 429 (Too Many Requests) — mapped to [`LlmFailure::RateLimit`].
const HTTP_TOO_MANY_REQUESTS: u16 = 429;

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
    /// Run one DeepSeek JSON-mode completion and map the HTTP/content outcome
    /// onto a [`LlmFailure`] per the module contract:
    ///
    /// - non-success status: 429 ⇒ [`LlmFailure::RateLimit`], everything else ⇒
    ///   [`LlmFailure::Upstream`] (status code only — never the provider body);
    /// - timeout ⇒ [`LlmFailure::Timeout`]; other transport failure ⇒
    ///   [`LlmFailure::Other`];
    /// - HTTP 200 with `finish_reason == "length"` ⇒ [`LlmFailure::Malformed`]
    ///   (truncated, never `Ok`);
    /// - HTTP 200 with empty/whitespace content ⇒ [`LlmFailure::Empty`];
    /// - HTTP 200 with content that parses to a JSON object ⇒ `Ok(Value)`;
    /// - HTTP 200 with content that is not valid JSON ⇒ [`LlmFailure::Malformed`].
    async fn complete_json(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<Value, LlmFailure> {
        let response = match self
            .client
            .chat_completion_json(system, user, max_tokens)
            .await
        {
            Ok(response) => response,
            Err(JsonChatError::Status(HTTP_TOO_MANY_REQUESTS)) => {
                return Err(LlmFailure::RateLimit)
            }
            Err(JsonChatError::Status(status)) => return Err(LlmFailure::Upstream(status)),
            Err(JsonChatError::Timeout) => return Err(LlmFailure::Timeout),
            Err(JsonChatError::Transport(message)) => return Err(LlmFailure::Other(message)),
        };

        // A `finish_reason` of "length" signals the model was cut off mid-output;
        // treat it as truncation (Malformed) even if the partial body parses.
        if response.finish_reason.as_deref() == Some("length") {
            return Err(LlmFailure::Malformed(
                "completion truncated (finish_reason = length)".to_string(),
            ));
        }

        let content = response.content.trim();
        if content.is_empty() {
            return Err(LlmFailure::Empty);
        }

        match serde_json::from_str::<Value>(content) {
            Ok(value) if value.is_object() => Ok(value),
            // Parsed, but not a JSON object (e.g. a bare string/number/array):
            // the contract requires a JSON object, so anything else is malformed.
            Ok(_) => Err(LlmFailure::Malformed("expected a JSON object".to_string())),
            Err(err) => Err(LlmFailure::Malformed(format!("invalid JSON: {err}"))),
        }
    }
}
