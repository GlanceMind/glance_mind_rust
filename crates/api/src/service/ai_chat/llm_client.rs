use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
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

impl LlmClient {
    pub fn new() -> Self {
        let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set in environment");
        let base_url =
            env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://timicc.com/v1".into());
        let model = env::var("AI_CHAT_MODEL").unwrap_or_else(|_| "gpt-5.2".into());

        Self {
            client: Client::new(),
            api_key,
            base_url,
            model,
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
            .map_err(|e| format!("LLM request failed: {}", e))?;

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
            let chunk = chunk.map_err(|e| format!("Stream read error: {}", e))?;
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
                                                .map(String::from);
                                            let name = tc
                                                .get("function")
                                                .and_then(|f| f.get("name"))
                                                .and_then(|n| n.as_str())
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
            .map(|i| {
                let ptc = &pending[&i];
                ToolCall {
                    id: ptc.id.clone(),
                    call_type: "function".into(),
                    function: FunctionCall {
                        name: ptc.name.clone(),
                        arguments: ptc.arguments.clone(),
                    },
                }
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
            .map_err(|e| format!("LLM request failed: {}", e))?;

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
}
