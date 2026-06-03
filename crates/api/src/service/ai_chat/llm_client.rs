use crate::service::deepseek_config;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    request_timeout_secs: u64,
}

/// The outcome of a JSON-mode chat completion: the raw assistant content string
/// (expected to be a JSON object, but NOT parsed here) plus the `finish_reason`
/// of the first choice (e.g. `"stop"`, `"length"`). Callers decide how to map
/// these to a domain result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonModeResponse {
    pub content: String,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default)]
pub struct LlmUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug)]
pub enum LlmStreamEvent {
    TextDelta(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },
    Usage(LlmUsage),
    Done,
}

#[derive(Debug, Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;

impl LlmClient {
    pub fn new() -> Self {
        let config = deepseek_config::from_env();
        let connect_timeout_secs =
            parse_env_u64("AI_CHAT_CONNECT_TIMEOUT_SECS", DEFAULT_CONNECT_TIMEOUT_SECS);
        let request_timeout_secs =
            parse_env_u64("AI_CHAT_REQUEST_TIMEOUT_SECS", DEFAULT_REQUEST_TIMEOUT_SECS);
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(connect_timeout_secs))
            .timeout(Duration::from_secs(request_timeout_secs))
            .build()
            .expect("Failed to build AI chat HTTP client");

        Self {
            client,
            api_key: config.api_key,
            base_url: config.base_url,
            model: config.model,
            request_timeout_secs,
        }
    }

    /// Construct an `LlmClient` pointed at an explicit `base_url` / `api_key` /
    /// `model`, bypassing the environment. Intended for tests that intercept the
    /// HTTP call (e.g. wiremock). Uses the default request timeout.
    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self::with_base_url_and_timeout(base_url, api_key, model, DEFAULT_REQUEST_TIMEOUT_SECS)
    }

    /// Like [`LlmClient::with_base_url`] but with an explicit request timeout (in
    /// seconds) so tests can force a short budget and exercise the timeout path.
    pub fn with_base_url_and_timeout(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        request_timeout_secs: u64,
    ) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(request_timeout_secs))
            .build()
            .expect("Failed to build AI chat HTTP client");
        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
            model: model.into(),
            request_timeout_secs,
        }
    }

    fn build_chat_body(
        &self,
        messages: &[ChatMessage],
        tools: &[Value],
        stream: bool,
        _ignored_model_override: Option<&str>,
    ) -> Value {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
        });

        if stream {
            body["stream"] = Value::Bool(true);
            body["stream_options"] = serde_json::json!({ "include_usage": true });
        }

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
            body["tool_choice"] = Value::String("auto".into());
        }

        body
    }

    /// Unified streaming call that handles both text and tool_calls in a single stream.
    /// Returns assembled tool calls (if any) so the caller can execute them without a second LLM call.
    pub async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        tools: &[Value],
        tx: mpsc::Sender<LlmStreamEvent>,
        model_override: Option<&str>,
    ) -> Result<Option<Vec<ToolCall>>, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.build_chat_body(messages, tools, true, model_override);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| self.format_request_error("LLM request failed", &e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Self::format_provider_error(status, &body));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut pending_tool_calls: HashMap<usize, PartialToolCall> = HashMap::new();

        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| self.format_request_error("Stream read error", &e))?;
            let raw = String::from_utf8_lossy(&chunk);
            buffer.push_str(&raw.replace("\r\n", "\n"));

            while let Some(pos) = buffer.find("\n\n") {
                let line_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in line_block.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }
                    let data_opt = line
                        .strip_prefix("data: ")
                        .or_else(|| line.strip_prefix("data:"));
                    if let Some(data) = data_opt {
                        if data.trim() == "[DONE]" {
                            let _ = tx.send(LlmStreamEvent::Done).await;
                            return Ok(self.assemble_tool_calls(&pending_tool_calls));
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            // Parse streaming usage if present (final chunk)
                            if let Some(usage) = parsed.get("usage") {
                                let u = LlmUsage {
                                    prompt_tokens: usage["prompt_tokens"].as_i64().unwrap_or(0),
                                    completion_tokens: usage["completion_tokens"]
                                        .as_i64()
                                        .unwrap_or(0),
                                    total_tokens: usage["total_tokens"].as_i64().unwrap_or(0),
                                };
                                if u.total_tokens > 0 {
                                    let _ = tx.send(LlmStreamEvent::Usage(u)).await;
                                }
                            }

                            if let Some(choices) = parsed.get("choices").and_then(|c| c.as_array())
                            {
                                for choice in choices {
                                    let delta = &choice["delta"];

                                    if let Some(content) =
                                        delta.get("content").and_then(|c| c.as_str())
                                    {
                                        if !content.is_empty() {
                                            let _ = tx
                                                .send(LlmStreamEvent::TextDelta(
                                                    content.to_string(),
                                                ))
                                                .await;
                                        }
                                    }

                                    if let Some(tool_calls) =
                                        delta.get("tool_calls").and_then(|t| t.as_array())
                                    {
                                        for tc in tool_calls {
                                            let index = tc
                                                .get("index")
                                                .and_then(|i| i.as_u64())
                                                .unwrap_or(0)
                                                as usize;
                                            let id = tc
                                                .get("id")
                                                .and_then(|i| i.as_str())
                                                .map(str::trim)
                                                .filter(|v| !v.is_empty())
                                                .map(String::from);
                                            let name = tc
                                                .get("function")
                                                .and_then(|f| f.get("name"))
                                                .and_then(|n| n.as_str())
                                                .map(str::trim)
                                                .filter(|v| !v.is_empty())
                                                .map(String::from);
                                            let args = tc
                                                .get("function")
                                                .and_then(|f| f.get("arguments"))
                                                .and_then(|a| a.as_str())
                                                .unwrap_or("");

                                            let entry =
                                                pending_tool_calls.entry(index).or_default();
                                            if let Some(ref id_val) = id {
                                                entry.id = id_val.clone();
                                            }
                                            if let Some(ref name_val) = name {
                                                entry.name = name_val.clone();
                                            }
                                            entry.arguments.push_str(args);

                                            let _ = tx
                                                .send(LlmStreamEvent::ToolCallDelta {
                                                    index,
                                                    id,
                                                    name,
                                                    arguments_delta: args.to_string(),
                                                })
                                                .await;
                                        }
                                    }

                                    let finish =
                                        choice.get("finish_reason").and_then(|f| f.as_str());
                                    if matches!(finish, Some("stop") | Some("tool_calls")) {
                                        let _ = tx.send(LlmStreamEvent::Done).await;
                                        return Ok(self.assemble_tool_calls(&pending_tool_calls));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let _ = tx.send(LlmStreamEvent::Done).await;
        Ok(self.assemble_tool_calls(&pending_tool_calls))
    }

    fn assemble_tool_calls(
        &self,
        pending: &HashMap<usize, PartialToolCall>,
    ) -> Option<Vec<ToolCall>> {
        if pending.is_empty() {
            return None;
        }
        let mut indices: Vec<usize> = pending.keys().copied().collect();
        indices.sort();
        let calls: Vec<ToolCall> = indices
            .into_iter()
            .filter_map(|i| {
                let ptc = &pending[&i];
                let tool_name = ptc.name.trim();
                if tool_name.is_empty() {
                    return None;
                }
                Some(ToolCall {
                    id: if ptc.id.trim().is_empty() {
                        format!("stream_tool_call_{}", i)
                    } else {
                        ptc.id.clone()
                    },
                    call_type: "function".into(),
                    function: FunctionCall {
                        name: tool_name.to_string(),
                        arguments: if ptc.arguments.trim().is_empty() {
                            "{}".into()
                        } else {
                            ptc.arguments.clone()
                        },
                    },
                })
            })
            .collect();
        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    }

    /// Non-streaming call kept for simple cases (e.g., title generation).
    /// Now also returns usage.
    pub async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        tools: &[Value],
        model_override: Option<&str>,
    ) -> Result<(ChatMessage, LlmUsage), String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = self.build_chat_body(messages, tools, false, model_override);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| self.format_request_error("LLM request failed", &e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response.text().await.unwrap_or_default();
            return Err(Self::format_provider_error(status, &body_text));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let usage = result
            .get("usage")
            .map(|u| LlmUsage {
                prompt_tokens: u["prompt_tokens"].as_i64().unwrap_or(0),
                completion_tokens: u["completion_tokens"].as_i64().unwrap_or(0),
                total_tokens: u["total_tokens"].as_i64().unwrap_or(0),
            })
            .unwrap_or_default();

        let choice = result
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .ok_or("No message in LLM response")?;

        let msg = ChatMessage {
            role: "assistant".to_string(),
            content: choice
                .get("content")
                .and_then(|c| c.as_str())
                .map(String::from),
            tool_calls: choice
                .get("tool_calls")
                .and_then(|t| serde_json::from_value(t.clone()).ok()),
            tool_call_id: None,
        };

        Ok((msg, usage))
    }

    /// Build the request body for a JSON-mode (DeepSeek `json_object`) chat
    /// completion: the configured `model`, a `[system, user]` message pair,
    /// `response_format: {"type":"json_object"}`, and the requested `max_tokens`.
    ///
    /// NOTE (B2 skeleton): this stub intentionally OMITS `response_format` and
    /// `max_tokens` so the RED unit test `chat_completion_json_body_sets_json_mode_and_max_tokens`
    /// fails by assertion until the implementer fills in the real body.
    fn build_json_chat_body(&self, system: &str, user: &str, _max_tokens: u32) -> Value {
        serde_json::json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        })
    }

    /// Send a single non-streaming JSON-mode chat completion to
    /// `{base}/chat/completions`, returning the assistant content string plus the
    /// `finish_reason` of the first choice. The request sets
    /// `response_format: {"type":"json_object"}` + `max_tokens` + the configured
    /// `model`. The API key comes only from config/env and is sent as
    /// `Authorization: Bearer`. Provider error bodies are redacted via
    /// [`Self::format_provider_error`].
    ///
    /// NOTE (B2 skeleton): this stub returns a fixed WRONG value and performs NO
    /// network I/O, so the boundary tests fail by assertion until implemented.
    pub async fn chat_completion_json(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<JsonModeResponse, String> {
        // Build the body so the helper is exercised on the production path too,
        // then discard it: this stub performs NO network I/O and returns a fixed
        // WRONG value until the implementer wires the real request + mapping.
        let _body = self.build_json_chat_body(system, user, max_tokens);
        let _ = (&self.client, &self.api_key);
        Ok(JsonModeResponse {
            content: "STUB_NOT_IMPLEMENTED".to_string(),
            finish_reason: Some("stop".to_string()),
        })
    }

    fn format_provider_error(status: reqwest::StatusCode, body: &str) -> String {
        if body.trim().is_empty() {
            return format!("LLM API error {status}");
        }

        format!("LLM API error {status}: provider returned an error")
    }

    fn format_request_error(&self, context: &str, error: &reqwest::Error) -> String {
        if error.is_timeout() {
            format!(
                "{}: request timed out after {}s",
                context, self.request_timeout_secs
            )
        } else {
            format!("{}: {}", context, error)
        }
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_client() -> LlmClient {
        LlmClient {
            client: Client::new(),
            api_key: "test-key".into(),
            base_url: "http://localhost".into(),
            model: "test-model".into(),
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        }
    }

    #[tokio::test]
    async fn chat_completion_posts_to_deepseek_config_with_forced_model() {
        use hyper::service::{make_service_fn, service_fn};
        use hyper::{Body, Request, Response, Server};
        use std::convert::Infallible;
        use std::net::SocketAddr;
        use std::sync::{Arc, Mutex};

        let captured = Arc::new(Mutex::new(None::<(String, Option<String>, Value)>));
        let captured_for_service = captured.clone();
        let make_svc = make_service_fn(move |_| {
            let captured = captured_for_service.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let captured = captured.clone();
                    async move {
                        let path = req.uri().path().to_string();
                        let auth = req
                            .headers()
                            .get("authorization")
                            .and_then(|value| value.to_str().ok())
                            .map(ToOwned::to_owned);
                        let bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
                        let body: Value = serde_json::from_slice(&bytes).unwrap();
                        *captured.lock().unwrap() = Some((path, auth, body));

                        Ok::<_, Infallible>(Response::new(Body::from(
                            r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":1,"completion_tokens":2,"total_tokens":3}}"#,
                        )))
                    }
                }))
            }
        });
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = Server::bind(&addr).serve(make_svc);
        let base_url = format!("http://{}", server.local_addr());
        let server_handle = tokio::spawn(server);

        let config = deepseek_config::from_values(
            Some("deepseek-test-key"),
            Some(&base_url),
            Some("deepseek-forced-model"),
        )
        .expect("test DeepSeek config should resolve");
        let client = LlmClient {
            client: Client::new(),
            api_key: config.api_key,
            base_url: config.base_url,
            model: config.model,
            request_timeout_secs: DEFAULT_REQUEST_TIMEOUT_SECS,
        };

        let (message, usage) = client
            .chat_completion(&[], &[], Some("client-selected-model"))
            .await
            .expect("mock completion should succeed");
        server_handle.abort();

        assert_eq!(message.content.as_deref(), Some("ok"));
        assert_eq!(usage.total_tokens, 3);
        let (path, auth, body) = captured
            .lock()
            .unwrap()
            .clone()
            .expect("request should be captured");
        assert_eq!(path, "/v1/chat/completions");
        assert_eq!(auth.as_deref(), Some("Bearer deepseek-test-key"));
        assert_eq!(body["model"], "deepseek-forced-model");
        assert_ne!(body["model"], "client-selected-model");
    }

    #[test]
    fn builds_chat_body_with_deepseek_model_even_when_override_is_supplied() {
        let client = stub_client();
        let body = client.build_chat_body(&[], &[], true, Some("client-selected-model"));

        assert_eq!(body["model"], "test-model");
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn builds_completion_body_with_deepseek_model_even_when_override_is_supplied() {
        let client = stub_client();
        let body = client.build_chat_body(&[], &[], false, Some("client-selected-model"));

        assert_eq!(body["model"], "test-model");
        assert!(body.get("stream").is_none());
    }

    #[test]
    fn chat_completion_json_body_sets_json_mode_and_max_tokens() {
        let client = stub_client();
        let body = client.build_json_chat_body(
            "You produce json output.",
            "Give me the json config.",
            512,
        );

        // DeepSeek JSON mode: response_format must be {"type":"json_object"}.
        assert_eq!(
            body["response_format"]["type"], "json_object",
            "json-mode body must set response_format.type = json_object; body = {body}"
        );
        // The passed max_tokens must be forwarded verbatim.
        assert_eq!(
            body["max_tokens"], 512,
            "json-mode body must forward max_tokens; body = {body}"
        );
        // Model comes from config (the stub client's configured model).
        assert_eq!(
            body["model"], "test-model",
            "json-mode body must use the configured model; body = {body}"
        );
        // Messages must carry the system + user content and mention json somewhere.
        let messages = body["messages"]
            .as_array()
            .expect("json-mode body must include a messages array");
        assert!(
            !messages.is_empty(),
            "json-mode body must include at least one message; body = {body}"
        );
        let any_mentions_json = messages.iter().any(|m| {
            m["content"]
                .as_str()
                .map(|c| c.to_lowercase().contains("json"))
                .unwrap_or(false)
        });
        assert!(
            any_mentions_json,
            "a message must contain the literal word \"json\" (DeepSeek json-mode requirement); body = {body}"
        );
    }

    #[test]
    fn provider_error_redacts_response_body() {
        let formatted = LlmClient::format_provider_error(
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            r#"{"error":{"message":"No available accounts for sk-secret-token"}}"#,
        );

        assert_eq!(
            formatted,
            "LLM API error 503 Service Unavailable: provider returned an error"
        );
        assert!(!formatted.contains("sk-secret-token"));
        assert!(!formatted.contains("No available accounts"));
    }

    #[test]
    fn assemble_tool_calls_filters_blank_names() {
        let client = stub_client();
        let pending = HashMap::from([
            (
                0usize,
                PartialToolCall {
                    id: "call_valid".into(),
                    name: "create_social_group".into(),
                    arguments: "{\"group_name\":\"A\"}".into(),
                },
            ),
            (
                1usize,
                PartialToolCall {
                    id: "call_blank".into(),
                    name: "   ".into(),
                    arguments: "{}".into(),
                },
            ),
        ]);

        let calls = client
            .assemble_tool_calls(&pending)
            .expect("valid call should remain");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_valid");
        assert_eq!(calls[0].function.name, "create_social_group");
    }

    #[test]
    fn assemble_tool_calls_defaults_missing_id_and_arguments() {
        let client = stub_client();
        let pending = HashMap::from([(
            2usize,
            PartialToolCall {
                id: "".into(),
                name: "create_plan_proposal".into(),
                arguments: "   ".into(),
            },
        )]);

        let calls = client
            .assemble_tool_calls(&pending)
            .expect("call should be assembled");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "stream_tool_call_2");
        assert_eq!(calls[0].function.name, "create_plan_proposal");
        assert_eq!(calls[0].function.arguments, "{}");
    }

    #[test]
    fn format_request_error_mentions_timeout_budget() {
        let client = stub_client();
        let error = client
            .client
            .get("::invalid-url")
            .build()
            .expect_err("request should fail");

        let formatted = client.format_request_error("LLM request failed", &error);
        assert!(
            formatted.starts_with("LLM request failed:"),
            "unexpected error format: {formatted}"
        );
    }
}
