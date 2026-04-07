use crate::dto::ai_chat_dto::*;
use crate::dto::common::PageResponse;
use crate::error::api_error::ApiError;
use crate::service::ai_chat::*;
use crate::state::user_state::UserState;
use glance_mind_db::entity::ai_chat::*;
use serde_json::{json, Value};
use tokio::sync::mpsc;

fn display_hint_for_tool(tool_name: &str) -> Option<&'static str> {
    match tool_name {
        "create_questionnaire_proposal" | "search_knowledge" => Some("hidden"),
        _ => None,
    }
}

const PRODUCT_HELP_QUERY_CUES: &[&str] = &[
    "怎么",
    "如何",
    "是什么",
    "什么是",
    "哪些",
    "有哪些",
    "区别",
    "支持",
    "配置",
    "设置",
    "入口",
    "在哪",
    "在哪里",
    "计费",
    "收费",
    "规则",
    "教程",
    "帮助",
    "文档",
    "guide",
    "how to",
    "what is",
    "which",
    "difference",
    "pricing",
    "billing",
    "configure",
    "setup",
    "docs",
    "documentation",
];

const PRODUCT_HELP_TOPICS: &[&str] = &[
    "glancemind",
    "回复模板",
    "template",
    "人设",
    "营销活动",
    "campaign",
    "发布计划",
    "publish plan",
    "计费",
    "钱包",
    "dm",
    "私信群控",
    "收件箱",
    "账号分组",
    "social group",
    "平台",
    "platform",
    "执行器",
    "executor",
    "市场洞察",
    "ai 智能获客",
    "获客",
];

const GENERIC_WRITING_CUES: &[&str] = &[
    "生成一个",
    "写一个",
    "帮我写",
    "给我一个",
    "起草",
    "润色",
    "改写",
    "翻译",
    "write",
    "generate",
    "draft",
    "compose",
    "reply template",
    "caption",
    "文案",
    "笔记",
    "评论回复",
    "示例",
    "例子",
    "网红笔记",
];

const DEFAULT_CHAT_MODEL_KEY: &str = "glm-5";

fn infer_forced_knowledge_query(content: &str) -> Option<String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    if looks_like_generic_writing_request(&lower) && !looks_like_product_help_query(&lower) {
        return None;
    }

    if looks_like_product_help_query(&lower) && contains_product_help_topic(&lower) {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn looks_like_product_help_query(lower: &str) -> bool {
    lower.contains('?')
        || lower.contains('？')
        || PRODUCT_HELP_QUERY_CUES
            .iter()
            .any(|cue| lower.contains(cue))
}

fn contains_product_help_topic(lower: &str) -> bool {
    PRODUCT_HELP_TOPICS
        .iter()
        .any(|topic| lower.contains(topic))
}

fn looks_like_generic_writing_request(lower: &str) -> bool {
    GENERIC_WRITING_CUES.iter().any(|cue| lower.contains(cue))
}

fn default_chat_model_key() -> String {
    std::env::var("AI_CHAT_MODEL").unwrap_or_else(|_| DEFAULT_CHAT_MODEL_KEY.to_string())
}

fn is_retryable_model_provider_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("503")
        || lower.contains("service unavailable")
        || lower.contains("no available accounts")
        || lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("429")
}

fn friendly_ai_chat_error_message(error: &str) -> String {
    let lower = error.to_lowercase();
    if is_retryable_model_provider_error(error) {
        "AI 模型服务暂时不可用，请稍后重试或切换到其他模型。".to_string()
    } else if lower.contains("timed out") || lower.contains("timeout") {
        "AI 模型响应超时，请稍后重试或切换到其他模型。".to_string()
    } else {
        error.to_string()
    }
}

fn build_forced_tool_call(
    tool_name: &str,
    params: &Value,
    unique_suffix: &str,
) -> llm_client::ToolCall {
    llm_client::ToolCall {
        id: format!("forced_{}_{}", tool_name, unique_suffix),
        call_type: "function".into(),
        function: llm_client::FunctionCall {
            name: tool_name.to_string(),
            arguments: serde_json::to_string(params).unwrap_or_else(|_| "{}".into()),
        },
    }
}

fn client_safe_tool_result(tool_name: &str, display_hint: Option<&str>, result: &Value) -> Value {
    if display_hint != Some("hidden") {
        return result.clone();
    }

    match tool_name {
        "search_knowledge" => {
            let sources = result
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .take(3)
                        .map(|entry| {
                            json!({
                                "title": entry.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown"),
                                "topic": entry.get("topic").and_then(|v| v.as_str()).unwrap_or("unknown"),
                                "url": entry.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            json!({
                "hidden": true,
                "count": result.as_array().map(|entries| entries.len()).unwrap_or(0),
                "sources": sources,
            })
        }
        "create_questionnaire_proposal" => json!({
            "hidden": true,
            "intent": result.get("intent").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "field_count": result.get("fields").and_then(|v| v.as_array()).map(|v| v.len()).unwrap_or(0),
        }),
        _ => json!({ "hidden": true }),
    }
}

#[derive(Clone)]
pub struct AiChatService {
    pub repo: AiChatRepository,
    pub llm: LlmClient,
}

impl AiChatService {
    pub fn new(repo: AiChatRepository) -> Self {
        Self {
            repo,
            llm: LlmClient::new(),
        }
    }

    pub fn create_conversation(
        &self,
        user_id: i32,
        title: Option<String>,
    ) -> Result<ConversationDto, ApiError> {
        let conv = self
            .repo
            .create_conversation(user_id, &title.unwrap_or_else(|| "New Chat".into()))?;
        Ok(ConversationDto::from(conv))
    }

    pub fn list_conversations(
        &self,
        user_id: i32,
        page: i32,
        page_size: i32,
    ) -> Result<PageResponse<ConversationDto>, ApiError> {
        let (items, total) = self.repo.list_conversations(user_id, page, page_size)?;
        let total_pages = (total + page_size as i64 - 1) / page_size as i64;
        Ok(PageResponse {
            list: items.into_iter().map(ConversationDto::from).collect(),
            total,
            page: page as i64,
            page_size: page_size as i64,
            total_pages,
        })
    }

    pub fn get_conversation(&self, id: i32, user_id: i32) -> Result<ConversationDto, ApiError> {
        self.repo
            .get_conversation(id, user_id)?
            .map(ConversationDto::from)
            .ok_or(ApiError::NotFound("Conversation not found".into()))
    }

    pub fn update_conversation(
        &self,
        id: i32,
        user_id: i32,
        req: UpdateConversationRequest,
    ) -> Result<ConversationDto, ApiError> {
        let update = UpdateAiConversation {
            title: req.title,
            status: None,
        };
        let conv = self.repo.update_conversation(id, user_id, &update)?;
        Ok(ConversationDto::from(conv))
    }

    pub fn delete_conversation(&self, id: i32, user_id: i32) -> Result<(), ApiError> {
        let deleted = self.repo.delete_conversation(id, user_id)?;
        if deleted == 0 {
            return Err(ApiError::NotFound("Conversation not found".into()));
        }
        Ok(())
    }

    pub fn get_messages(&self, conv_id: i32, user_id: i32) -> Result<Vec<MessageDto>, ApiError> {
        self.repo
            .get_conversation(conv_id, user_id)?
            .ok_or(ApiError::NotFound("Conversation not found".into()))?;

        let messages = self
            .repo
            .list_messages(conv_id, MAX_CONTEXT_MESSAGES as i64)?;
        Ok(messages.into_iter().map(MessageDto::from).collect())
    }

    /// Build LLM context with token budget, ensuring tool_call pair completeness.
    ///
    /// OpenAI requires: every assistant message with `tool_calls` MUST be followed by
    /// a `tool` message for EACH `tool_call_id`. Missing any causes a 400 error.
    fn build_context(&self, history: &[AiMessage]) -> Vec<llm_client::ChatMessage> {
        let system_prompt = SYSTEM_PROMPT.to_string();
        let system_tokens = estimate_tokens(&system_prompt);
        let tool_tokens = 6000;
        let mut budget = MAX_CONTEXT_TOKENS.saturating_sub(system_tokens + tool_tokens);

        let mut messages: Vec<llm_client::ChatMessage> = vec![llm_client::ChatMessage {
            role: "system".into(),
            content: Some(system_prompt),
            tool_calls: None,
            tool_call_id: None,
        }];

        // Pre-index: for each assistant message with tool_calls, find all required tool_call_ids
        // and map them to the indices of their corresponding tool result messages.
        let mut assistant_tool_ids: std::collections::HashMap<usize, Vec<String>> =
            std::collections::HashMap::new();
        let mut tool_result_index: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for (idx, msg) in history.iter().enumerate() {
            if msg.role == "assistant" {
                if let Some(tc_json) = &msg.tool_calls {
                    if let Ok(tcs) =
                        serde_json::from_value::<Vec<llm_client::ToolCall>>(tc_json.clone())
                    {
                        let ids: Vec<String> = tcs.iter().map(|tc| tc.id.clone()).collect();
                        assistant_tool_ids.insert(idx, ids);
                    }
                }
            } else if msg.role == "tool" {
                if let Some(tc_id) = &msg.tool_call_id {
                    tool_result_index.insert(tc_id.clone(), idx);
                }
            }
        }

        // Scan from newest to oldest, collecting messages while respecting budget.
        let mut selected: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut i = history.len();

        while i > 0 {
            i -= 1;
            if selected.contains(&i) {
                continue;
            }

            let msg = &history[i];

            // For assistant messages with tool_calls, include as a complete group or skip entirely.
            if msg.role == "assistant" && assistant_tool_ids.contains_key(&i) {
                let tc_ids = &assistant_tool_ids[&i];
                let tool_indices: Vec<usize> = tc_ids
                    .iter()
                    .filter_map(|id| tool_result_index.get(id).copied())
                    .collect();

                // If any tool result is missing from history, skip this entire group (orphaned)
                if tool_indices.len() != tc_ids.len() {
                    continue;
                }

                // Estimate total cost of assistant + all tool results
                let assistant_tokens = estimate_tokens(&msg.content) + 50;
                let mut group_tokens = assistant_tokens;
                for &tidx in &tool_indices {
                    let tmsg = &history[tidx];
                    let content = if let Some(tc_id) = &tmsg.tool_call_id {
                        let tool_name = self.find_tool_name_for_call(history, tc_id);
                        let result: Value =
                            serde_json::from_str(&tmsg.content).unwrap_or(Value::Null);
                        compress_tool_result(&tool_name, &result)
                    } else {
                        tmsg.content.clone()
                    };
                    group_tokens += estimate_tokens(&content);
                }

                if group_tokens > budget {
                    break;
                }
                budget = budget.saturating_sub(group_tokens);
                selected.insert(i);
                for tidx in tool_indices {
                    selected.insert(tidx);
                }
                continue;
            }

            // For tool messages: they'll be pulled in by their assistant group above, skip standalone
            if msg.role == "tool" {
                continue;
            }

            // Regular user/assistant messages (no tool_calls)
            let msg_tokens = estimate_tokens(&msg.content);
            if msg_tokens > budget {
                break;
            }
            budget = budget.saturating_sub(msg_tokens);
            selected.insert(i);
        }

        // Build final message list in chronological order
        let mut sorted: Vec<usize> = selected.into_iter().collect();
        sorted.sort();

        for &idx in &sorted {
            let msg = &history[idx];
            let content = if msg.role == "tool" {
                if let Some(tc_id) = &msg.tool_call_id {
                    let tool_name = self.find_tool_name_for_call(history, tc_id);
                    let result: Value = serde_json::from_str(&msg.content).unwrap_or(Value::Null);
                    Some(compress_tool_result(&tool_name, &result))
                } else {
                    Some(msg.content.clone())
                }
            } else if msg.content.is_empty() {
                let has_tc = msg.tool_calls.is_some();
                if has_tc || msg.role == "assistant" {
                    Some(String::new())
                } else {
                    None
                }
            } else {
                Some(msg.content.clone())
            };

            let mut cm = llm_client::ChatMessage {
                role: msg.role.clone(),
                content,
                tool_calls: None,
                tool_call_id: msg.tool_call_id.clone(),
            };
            if let Some(tc) = &msg.tool_calls {
                cm.tool_calls = serde_json::from_value(tc.clone()).ok();
            }
            messages.push(cm);
        }

        messages
    }

    fn find_tool_name_for_call(&self, history: &[AiMessage], tool_call_id: &str) -> String {
        for msg in history.iter().rev() {
            if let Some(tc_json) = &msg.tool_calls {
                if let Ok(tcs) =
                    serde_json::from_value::<Vec<llm_client::ToolCall>>(tc_json.clone())
                {
                    for tc in &tcs {
                        if tc.id == tool_call_id {
                            return tc.function.name.clone();
                        }
                    }
                }
            }
        }
        String::new()
    }

    fn persist_assistant_tool_calls(
        &self,
        conv_id: i32,
        text_content: &str,
        tool_calls: &[llm_client::ToolCall],
        messages: &mut Vec<llm_client::ChatMessage>,
    ) -> Result<(), ApiError> {
        let tc_json: Value = serde_json::to_value(tool_calls).unwrap_or_default();
        self.repo.create_message(&NewAiMessage {
            conversation_id: conv_id,
            role: "assistant".into(),
            content: text_content.to_string(),
            tool_calls: Some(tc_json),
            tool_call_id: None,
            plan_id: None,
        })?;

        messages.push(llm_client::ChatMessage {
            role: "assistant".into(),
            content: Some(text_content.to_string()),
            tool_calls: Some(tool_calls.to_vec()),
            tool_call_id: None,
        });

        Ok(())
    }

    async fn build_chat_model_attempts(
        &self,
        preferred_model_key: Option<&str>,
        state: &UserState,
    ) -> Result<Vec<String>, ApiError> {
        let mut attempts = Vec::new();
        let preferred = preferred_model_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_chat_model_key);
        attempts.push(preferred.clone());

        let fallback_models = state
            .config_service
            .get_ai_models_by_type("chat")
            .await
            .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        for model in fallback_models {
            let key = model.model_key.trim();
            if key.is_empty() {
                continue;
            }
            if attempts.iter().any(|existing| existing == key) {
                continue;
            }
            attempts.push(key.to_string());
        }

        Ok(attempts)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_message(
        &self,
        conv_id: i32,
        user_id: i32,
        content: &str,
        model_id: Option<i32>,
        ui_capabilities: AiChatUiCapabilities,
        questionnaire_submission: Option<QuestionnaireSubmission>,
        state: &UserState,
        tx: mpsc::Sender<SseEvent>,
    ) -> Result<(), ApiError> {
        if content.len() > MAX_MESSAGE_LENGTH {
            return Err(ApiError::BadRequest(format!(
                "Message too long. Max {} characters.",
                MAX_MESSAGE_LENGTH
            )));
        }

        let preferred_model_key: Option<String> = if let Some(mid) = model_id {
            let model = state
                .config_service
                .get_ai_model_by_id(mid)
                .await
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                .ok_or_else(|| ApiError::BadRequest(format!("AI model id={} not found", mid)))?;
            if !model.is_active {
                return Err(ApiError::BadRequest(format!(
                    "AI model '{}' is not active",
                    model.name
                )));
            }
            if model.model_type != "chat" {
                return Err(ApiError::BadRequest(format!(
                    "AI model '{}' is not a chat model",
                    model.name
                )));
            }
            tracing::info!(
                "AI Chat using model override: {} (id={})",
                model.model_key,
                mid
            );
            Some(model.model_key)
        } else {
            tracing::info!("AI Chat using default model (no override)");
            None
        };

        self.repo
            .get_conversation(conv_id, user_id)?
            .ok_or(ApiError::NotFound("Conversation not found".into()))?;

        let _user_msg = self.repo.create_message(&NewAiMessage {
            conversation_id: conv_id,
            role: "user".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            plan_id: None,
        })?;

        let history = self
            .repo
            .list_messages(conv_id, MAX_CONTEXT_MESSAGES as i64)?;
        let mut messages = self.build_context(&history);

        if let Some(system_message) = messages.first_mut() {
            let mut directives = String::new();
            if ui_capabilities.questionnaire {
                directives.push_str(
                    "\n\n当前客户端支持结构化问卷：\n\
                    - 当用户要创建 AI 视频或创建 AI 发布计划且缺少参数时，必须调用 create_questionnaire_proposal，绝对不要输出 Q1/Q2/Q3 文本。\n\
                    - AI 视频使用 create_questionnaire_proposal(intent=generate_video)。\n\
                    - 发布计划使用 create_questionnaire_proposal(intent=create_publish_plan)，从用户输入中提取已知的 platform_id、content_type、group_id、content_prompt 作为参数传入。\n\
                    - create_questionnaire_proposal 返回后，正文只需用 1 句话提示用户在下方完成选择。\n\
                    - 当用户提交 questionnaire_submission 后，不要再次追问已填写字段，直接使用这些结构化参数调用 create_plan_proposal 生成待确认计划。\n\
                    - 严禁使用 Q1/Q2/Q3 + A/B/C 文本格式，必须使用 create_questionnaire_proposal 工具。\n",
                );
            } else {
                directives.push_str(
                    "\n\n当前客户端不支持结构化问卷：\n\
                    - 不要调用 create_questionnaire_proposal。\n\
                    - 继续使用 Q1/Q2/Q3 + A/B/C 的文本方式收集缺失参数。\n",
                );
            }

            directives.push_str(
                "\n\n产品帮助问答判定规则：\n\
                - 只有当用户明确在问 GlanceMind 平台功能、配置、规则、入口、计费、支持范围、功能区别时，才调用 search_knowledge。\n\
                - 如果用户是在让你直接写模板、文案、评论回复、DM 话术、营销示例或其他创作内容，这是通用写作任务，不要调用 search_knowledge。\n\
                - 如果系统已提供 search_knowledge 的工具结果，优先基于现有结果回答，不要重复检索同一主题。\n",
            );

            if questionnaire_submission.is_some() {
                directives.push_str(
                    "\n用户本轮消息包含 questionnaire_submission，结构化字段值比展示文本更可靠，必须以结构化字段为准。\n",
                );
            }

            if let Some(existing) = &mut system_message.content {
                existing.push_str(&directives);
            } else {
                system_message.content = Some(directives);
            }
        }

        if let Some(submission) = &questionnaire_submission {
            let normalized =
                ToolRegistry::normalize_questionnaire_submission(submission, state).await?;
            if let Some(latest_user_message) =
                messages.iter_mut().rev().find(|msg| msg.role == "user")
            {
                latest_user_message.content = Some(normalized);
            }
        }

        let tools = ToolRegistry::openai_tools();
        let mut tool_call_count = 0;
        let mut total_usage = llm_client::LlmUsage::default();
        let mut empty_response_retry_count = 0usize;
        let mut active_model_key = preferred_model_key.clone();

        if questionnaire_submission.is_none() {
            if let Some(query) = infer_forced_knowledge_query(content) {
                let params = json!({ "query": query });
                let forced_call = build_forced_tool_call(
                    "search_knowledge",
                    &params,
                    &format!("{}_{}", conv_id, history.len()),
                );
                let forced_calls = vec![forced_call.clone()];
                self.persist_assistant_tool_calls(conv_id, "", &forced_calls, &mut messages)?;

                tool_call_count += 1;
                let _ = tx
                    .send(SseEvent::ToolCallStart {
                        tool_call_id: forced_call.id.clone(),
                        tool_name: forced_call.function.name.clone(),
                    })
                    .await;
                let forced_result =
                    ToolRegistry::execute("search_knowledge", params, user_id, state).await;
                self.process_tool_result(
                    conv_id,
                    user_id,
                    &forced_call.id,
                    "search_knowledge",
                    forced_result,
                    &tx,
                    &mut messages,
                )
                .await?;
            }
        }

        loop {
            let model_attempts = self
                .build_chat_model_attempts(active_model_key.as_deref(), state)
                .await?;

            let mut text_content = String::new();
            let mut assembled_tool_calls: Option<Vec<llm_client::ToolCall>> = None;
            let mut successful_model_key: Option<String> = None;
            let mut last_retryable_error: Option<String> = None;

            for attempt_model_key in model_attempts {
                let (stream_tx, mut stream_rx) = mpsc::channel::<llm_client::LlmStreamEvent>(64);
                let llm = self.llm.clone();
                let msgs_clone = messages.clone();
                let tools_clone = tools.clone();
                let model_override = Some(attempt_model_key.clone());

                let stream_handle = tokio::spawn(async move {
                    llm.chat_stream(
                        &msgs_clone,
                        &tools_clone,
                        stream_tx,
                        model_override.as_deref(),
                    )
                    .await
                });

                text_content.clear();

                while let Some(evt) = stream_rx.recv().await {
                    match evt {
                        llm_client::LlmStreamEvent::TextDelta(delta) => {
                            text_content.push_str(&delta);
                            let _ = tx.send(SseEvent::TextDelta { delta }).await;
                        }
                        llm_client::LlmStreamEvent::Usage(u) => {
                            total_usage.prompt_tokens += u.prompt_tokens;
                            total_usage.completion_tokens += u.completion_tokens;
                            total_usage.total_tokens += u.total_tokens;
                        }
                        llm_client::LlmStreamEvent::ToolCallDelta { .. } => {
                            // Deltas are accumulated inside chat_stream; we just wait for Done
                        }
                        llm_client::LlmStreamEvent::Done => break,
                    }
                }

                let stream_result = stream_handle
                    .await
                    .map_err(|e| ApiError::AiServiceError(format!("Stream task failed: {}", e)))?;

                match stream_result {
                    Ok(tool_calls) => {
                        assembled_tool_calls = tool_calls;
                        successful_model_key = Some(attempt_model_key);
                        break;
                    }
                    Err(err)
                        if text_content.trim().is_empty()
                            && is_retryable_model_provider_error(&err) =>
                    {
                        tracing::warn!(
                            conv_id = conv_id,
                            user_id = user_id,
                            model = %attempt_model_key,
                            error = %err,
                            "AI Chat model unavailable, trying fallback model"
                        );
                        last_retryable_error = Some(err);
                        continue;
                    }
                    Err(err) => {
                        return Err(ApiError::AiServiceError(friendly_ai_chat_error_message(
                            &err,
                        )));
                    }
                }
            }

            if let Some(model_key) = successful_model_key {
                active_model_key = Some(model_key);
            } else if let Some(err) = last_retryable_error {
                return Err(ApiError::AiServiceError(friendly_ai_chat_error_message(
                    &err,
                )));
            }

            if let Some(ref tool_calls) = assembled_tool_calls {
                if !tool_calls.is_empty() {
                    self.persist_assistant_tool_calls(
                        conv_id,
                        &text_content,
                        tool_calls,
                        &mut messages,
                    )?;

                    // Execute tools - parallel for ReadOnly, sequential for mutations
                    let (readonly_calls, mutation_calls): (Vec<_>, Vec<_>) =
                        tool_calls.iter().partition(|tc| {
                            ToolRegistry::get_safety_level(&tc.function.name)
                                == SafetyLevel::ReadOnly
                        });

                    // Parallel execution of ReadOnly tools
                    if !readonly_calls.is_empty() {
                        let mut handles = Vec::new();
                        for tc in &readonly_calls {
                            tool_call_count += 1;
                            if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                                let _ = tx
                                    .send(SseEvent::Error {
                                        message: "Too many tool calls in this turn".into(),
                                    })
                                    .await;
                                break;
                            }
                            let _ = tx
                                .send(SseEvent::ToolCallStart {
                                    tool_call_id: tc.id.clone(),
                                    tool_name: tc.function.name.clone(),
                                })
                                .await;

                            let params: Value =
                                serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                            let name = tc.function.name.clone();
                            let tc_id = tc.id.clone();
                            let st = state.clone();
                            handles.push(tokio::spawn(async move {
                                let result =
                                    ToolRegistry::execute(&name, params, user_id, &st).await;
                                (tc_id, name, result)
                            }));
                        }
                        let results = futures::future::join_all(handles).await;
                        for (tc_id, name, tool_result) in results.into_iter().flatten() {
                            self.process_tool_result(
                                conv_id,
                                user_id,
                                &tc_id,
                                &name,
                                tool_result,
                                &tx,
                                &mut messages,
                            )
                            .await?;
                        }
                    }

                    // Sequential execution of mutation tools
                    for tc in &mutation_calls {
                        tool_call_count += 1;
                        if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                            let _ = tx
                                .send(SseEvent::Error {
                                    message: "Too many tool calls in this turn".into(),
                                })
                                .await;
                            break;
                        }
                        let _ = tx
                            .send(SseEvent::ToolCallStart {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.function.name.clone(),
                            })
                            .await;

                        let params: Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        let tool_result =
                            ToolRegistry::execute(&tc.function.name, params, user_id, state).await;
                        self.process_tool_result(
                            conv_id,
                            user_id,
                            &tc.id,
                            &tc.function.name,
                            tool_result,
                            &tx,
                            &mut messages,
                        )
                        .await?;
                    }

                    if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                        break;
                    }
                    continue;
                }
            }

            if text_content.trim().is_empty() {
                empty_response_retry_count += 1;
                if empty_response_retry_count <= 2 {
                    tracing::warn!(
                        conv_id = conv_id,
                        user_id = user_id,
                        retry = empty_response_retry_count,
                        "LLM returned empty response without valid tool calls, retrying"
                    );
                    continue;
                }

                let _ = tx
                    .send(SseEvent::Error {
                        message: "AI service returned an empty response".into(),
                    })
                    .await;
                return Err(ApiError::AiServiceError(
                    "LLM returned an empty response without text or valid tool calls".into(),
                ));
            }

            // No tool calls - text was already streamed. Persist and finish.
            let assistant_msg = self.repo.create_message(&NewAiMessage {
                conversation_id: conv_id,
                role: "assistant".into(),
                content: text_content,
                tool_calls: None,
                tool_call_id: None,
                plan_id: None,
            })?;

            // Log token usage
            if total_usage.total_tokens > 0 {
                tracing::info!(
                    conv_id = conv_id,
                    user_id = user_id,
                    prompt_tokens = total_usage.prompt_tokens,
                    completion_tokens = total_usage.completion_tokens,
                    total_tokens = total_usage.total_tokens,
                    "AI Chat token usage"
                );
            }

            let _ = tx
                .send(SseEvent::MessageEnd {
                    message_id: assistant_msg.id,
                    finish_reason: "stop".into(),
                })
                .await;

            break;
        }

        self.repo.touch_conversation(conv_id)?;
        Ok(())
    }

    /// Process a single tool result: audit, SSE events, plan creation, message persistence
    #[allow(clippy::too_many_arguments)]
    async fn process_tool_result(
        &self,
        conv_id: i32,
        user_id: i32,
        tc_id: &str,
        tool_name: &str,
        tool_result: Result<Value, ApiError>,
        tx: &mpsc::Sender<SseEvent>,
        messages: &mut Vec<llm_client::ChatMessage>,
    ) -> Result<(), ApiError> {
        let safety = ToolRegistry::get_safety_level(tool_name);
        let (result_value, success) = match &tool_result {
            Ok(v) => (v.clone(), true),
            Err(e) => (serde_json::json!({ "error": e.to_string() }), false),
        };

        let _ = self.repo.log_tool_call(&NewAiToolAuditLog {
            user_id,
            conversation_id: Some(conv_id),
            tool_name: tool_name.to_string(),
            safety_level: safety.as_str().to_string(),
            success,
            error_message: if success {
                None
            } else {
                Some(result_value["error"].as_str().unwrap_or("").to_string())
            },
        });

        let display_hint = display_hint_for_tool(tool_name).map(str::to_string);
        let client_result =
            client_safe_tool_result(tool_name, display_hint.as_deref(), &result_value);

        let _ = tx
            .send(SseEvent::ToolCallResult {
                tool_call_id: tc_id.to_string(),
                result: client_result,
                success,
                display_hint: display_hint.clone(),
            })
            .await;

        if tool_name == "create_questionnaire_proposal" && success {
            if let Ok(questionnaire) =
                serde_json::from_value::<QuestionnairePayload>(result_value.clone())
            {
                let _ = tx.send(SseEvent::Questionnaire { questionnaire }).await;
            }
        }

        if tool_name == "create_plan_proposal" && success {
            if let Ok(proposal) = serde_json::from_value::<PlanProposal>(result_value.clone()) {
                let plan = self.create_plan_from_proposal(conv_id, user_id, &proposal)?;
                let steps = self.repo.get_plan_steps(plan.id)?;
                let _ = tx
                    .send(SseEvent::PlanCreated {
                        plan_id: plan.id,
                        title: proposal.title.clone(),
                        steps: steps
                            .iter()
                            .map(|s| PlanStepSse {
                                step_id: s.id,
                                step_order: s.step_order,
                                tool_name: s.tool_name.clone(),
                                description: s.description.clone(),
                                tool_params: s.tool_params.clone(),
                            })
                            .collect(),
                    })
                    .await;
            }
        }

        // Store full result in DB, but send compressed version to LLM context
        let full_result_str = serde_json::to_string(&result_value).unwrap_or_default();
        self.repo.create_message(&NewAiMessage {
            conversation_id: conv_id,
            role: "tool".into(),
            content: full_result_str,
            tool_calls: None,
            tool_call_id: Some(tc_id.to_string()),
            plan_id: None,
        })?;

        // Add compressed version to in-memory context for next LLM round
        let compressed = compress_tool_result(tool_name, &result_value);
        messages.push(llm_client::ChatMessage {
            role: "tool".into(),
            content: Some(compressed),
            tool_calls: None,
            tool_call_id: Some(tc_id.to_string()),
        });

        Ok(())
    }

    fn create_plan_from_proposal(
        &self,
        conv_id: i32,
        user_id: i32,
        proposal: &PlanProposal,
    ) -> Result<AiPlan, ApiError> {
        let plan = self.repo.create_plan(&NewAiPlan {
            conversation_id: conv_id,
            message_id: None,
            user_id,
            title: proposal.title.clone(),
            description: proposal.description.clone(),
            status: "draft".into(),
        })?;

        let steps: Vec<NewAiPlanStep> = proposal
            .steps
            .iter()
            .enumerate()
            .map(|(i, s)| NewAiPlanStep {
                plan_id: plan.id,
                step_order: i as i32 + 1,
                tool_name: s.tool_name.clone(),
                tool_params: s.tool_params.clone(),
                description: s.description.clone(),
                status: "pending".into(),
            })
            .collect();

        self.repo.create_plan_steps(&steps)?;
        Ok(plan)
    }

    pub fn get_plan(&self, plan_id: i32, user_id: i32) -> Result<PlanDetailDto, ApiError> {
        let plan = self
            .repo
            .get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;
        let steps = self.repo.get_plan_steps(plan_id)?;
        Ok(PlanDetailDto::from_plan_and_steps(plan, steps))
    }

    pub async fn confirm_plan(
        &self,
        plan_id: i32,
        user_id: i32,
        state: &UserState,
        tx: mpsc::Sender<SseEvent>,
    ) -> Result<(), ApiError> {
        let plan = self
            .repo
            .get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest(format!(
                "Plan status is '{}', expected 'draft'",
                plan.status
            )));
        }

        self.repo.update_plan(
            plan_id,
            user_id,
            &UpdateAiPlan {
                status: Some("executing".into()),
                ..Default::default()
            },
        )?;

        let steps = self.repo.get_plan_steps(plan_id)?;
        let mut all_success = true;

        for step in &steps {
            let _ = tx
                .send(SseEvent::StepStart {
                    step_id: step.id,
                    step_order: step.step_order,
                    description: step.description.clone(),
                })
                .await;

            self.repo.update_plan_step(
                step.id,
                &UpdateAiPlanStep {
                    status: Some("executing".into()),
                    ..Default::default()
                },
            )?;

            let result =
                ToolRegistry::execute(&step.tool_name, step.tool_params.clone(), user_id, state)
                    .await;

            let _ = self.repo.log_tool_call(&NewAiToolAuditLog {
                user_id,
                conversation_id: Some(plan.conversation_id),
                tool_name: step.tool_name.clone(),
                safety_level: ToolRegistry::get_safety_level(&step.tool_name)
                    .as_str()
                    .to_string(),
                success: result.is_ok(),
                error_message: result.as_ref().err().map(|e| e.to_string()),
            });

            match result {
                Ok(val) => {
                    self.repo.update_plan_step(
                        step.id,
                        &UpdateAiPlanStep {
                            status: Some("completed".into()),
                            result: Some(val.clone()),
                            ..Default::default()
                        },
                    )?;
                    let _ = tx
                        .send(SseEvent::StepCompleted {
                            step_id: step.id,
                            result: val,
                        })
                        .await;
                }
                Err(e) => {
                    all_success = false;
                    self.repo.update_plan_step(
                        step.id,
                        &UpdateAiPlanStep {
                            status: Some("failed".into()),
                            error_message: Some(Some(e.to_string())),
                            ..Default::default()
                        },
                    )?;
                    let _ = tx
                        .send(SseEvent::StepFailed {
                            step_id: step.id,
                            error: e.to_string(),
                        })
                        .await;
                    break;
                }
            }
        }

        let final_status = if all_success { "completed" } else { "failed" };
        self.repo.update_plan(
            plan_id,
            user_id,
            &UpdateAiPlan {
                status: Some(final_status.into()),
                ..Default::default()
            },
        )?;

        let _ = tx
            .send(SseEvent::PlanCompleted {
                plan_id,
                summary: if all_success {
                    "所有步骤执行成功".into()
                } else {
                    "部分步骤执行失败".into()
                },
            })
            .await;

        Ok(())
    }

    pub fn cancel_plan(&self, plan_id: i32, user_id: i32) -> Result<PlanDetailDto, ApiError> {
        let plan = self
            .repo
            .get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest("Can only cancel draft plans".into()));
        }

        let plan = self.repo.update_plan(
            plan_id,
            user_id,
            &UpdateAiPlan {
                status: Some("cancelled".into()),
                ..Default::default()
            },
        )?;

        let steps = self.repo.get_plan_steps(plan_id)?;
        Ok(PlanDetailDto::from_plan_and_steps(plan, steps))
    }

    pub fn update_plan_step(
        &self,
        plan_id: i32,
        step_id: i32,
        user_id: i32,
        req: UpdatePlanStepRequest,
    ) -> Result<PlanStepDto, ApiError> {
        let plan = self
            .repo
            .get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest(
                "Can only modify draft plan steps".into(),
            ));
        }

        let update = UpdateAiPlanStep {
            tool_params: req.tool_params,
            description: req.description,
            ..Default::default()
        };

        let step = self.repo.update_plan_step(step_id, &update)?;
        if step.plan_id != plan_id {
            return Err(ApiError::NotFound(
                "Step does not belong to this plan".into(),
            ));
        }

        Ok(PlanStepDto::from(step))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        client_safe_tool_result, display_hint_for_tool, friendly_ai_chat_error_message,
        infer_forced_knowledge_query, is_retryable_model_provider_error,
    };
    use serde_json::json;

    #[test]
    fn search_knowledge_tool_results_are_hidden() {
        assert_eq!(display_hint_for_tool("search_knowledge"), Some("hidden"));
        assert_eq!(
            display_hint_for_tool("create_questionnaire_proposal"),
            Some("hidden")
        );
        assert_eq!(display_hint_for_tool("list_platforms"), None);
    }

    #[test]
    fn infer_forced_knowledge_query_matches_product_help_only() {
        assert_eq!(
            infer_forced_knowledge_query("GlanceMind 里的回复模板怎么配置？"),
            Some("GlanceMind 里的回复模板怎么配置？".to_string())
        );
        assert_eq!(
            infer_forced_knowledge_query("DM 私信群控是什么"),
            Some("DM 私信群控是什么".to_string())
        );
        assert_eq!(
            infer_forced_knowledge_query("生成一个网红笔记的回复模板"),
            None
        );
        assert_eq!(infer_forced_knowledge_query("帮我写一段评论回复文案"), None);
    }

    #[test]
    fn hidden_tool_result_for_search_knowledge_is_client_safe() {
        let safe = client_safe_tool_result(
            "search_knowledge",
            Some("hidden"),
            &json!([
                {
                    "title": "配置回复模板",
                    "topic": "template_guide",
                    "url": "https://docs.glancemind.org/guide/create-template.html",
                    "content": "very long body that should never be streamed to hidden clients"
                }
            ]),
        );

        assert_eq!(safe["hidden"], json!(true));
        assert_eq!(safe["count"], json!(1));
        assert_eq!(safe["sources"][0]["title"], json!("配置回复模板"));
        assert!(safe["sources"][0].get("content").is_none());
    }

    #[test]
    fn provider_unavailable_errors_are_retryable() {
        assert!(is_retryable_model_provider_error(
            "LLM API error 503 Service Unavailable: No available accounts"
        ));
        assert!(is_retryable_model_provider_error(
            "LLM API error 429 Too Many Requests"
        ));
        assert!(!is_retryable_model_provider_error(
            "LLM API error 400 Invalid request"
        ));
    }

    #[test]
    fn friendly_ai_error_message_hides_provider_details() {
        let friendly = friendly_ai_chat_error_message(
            "LLM API error 503 Service Unavailable: {\"error\":{\"message\":\"No available accounts\"}}",
        );
        assert_eq!(
            friendly,
            "AI 模型服务暂时不可用，请稍后重试或切换到其他模型。"
        );
    }
}
