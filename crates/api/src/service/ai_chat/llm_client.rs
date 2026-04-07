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
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set in environment");
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://timicc.com/v1".into());
        let model = env::var("AI_CHAT_MODEL").unwrap_or_else(|_| "glm-5".into());
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
            api_key,
            base_url,
            model,
            request_timeout_secs,
        }
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
        let model = model_override.unwrap_or(&self.model);

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
            body["tool_choice"] = Value::String("auto".into());
        }

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
            return Err(format!("LLM API error {}: {}", status, body));
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
        let model = model_override.unwrap_or(&self.model);

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
        });

        if !tools.is_empty() {
            body["tools"] = Value::Array(tools.to_vec());
            body["tool_choice"] = Value::String("auto".into());
        }

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
            return Err(format!("LLM API error {}: {}", status, body_text));
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
