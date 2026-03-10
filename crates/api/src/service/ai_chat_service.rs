use crate::dto::ai_chat_dto::*;
use crate::dto::common::PageResponse;
use crate::error::api_error::ApiError;
use crate::service::ai_chat::*;
use crate::state::user_state::UserState;
use glance_mind_db::entity::ai_chat::*;
use serde_json::Value;
use tokio::sync::mpsc;

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

    pub fn create_conversation(&self, user_id: i32, title: Option<String>) -> Result<ConversationDto, ApiError> {
        let conv = self.repo.create_conversation(user_id, &title.unwrap_or_else(|| "New Chat".into()))?;
        Ok(ConversationDto::from(conv))
    }

    pub fn list_conversations(&self, user_id: i32, page: i32, page_size: i32) -> Result<PageResponse<ConversationDto>, ApiError> {
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

    pub fn update_conversation(&self, id: i32, user_id: i32, req: UpdateConversationRequest) -> Result<ConversationDto, ApiError> {
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
        self.repo.get_conversation(conv_id, user_id)?
            .ok_or(ApiError::NotFound("Conversation not found".into()))?;

        let messages = self.repo.list_messages(conv_id, MAX_CONTEXT_MESSAGES as i64)?;
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
        let mut assistant_tool_ids: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();
        let mut tool_result_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for (idx, msg) in history.iter().enumerate() {
            if msg.role == "assistant" {
                if let Some(tc_json) = &msg.tool_calls {
                    if let Ok(tcs) = serde_json::from_value::<Vec<llm_client::ToolCall>>(tc_json.clone()) {
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
                let tool_indices: Vec<usize> = tc_ids.iter()
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
                        let result: Value = serde_json::from_str(&tmsg.content).unwrap_or(Value::Null);
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
                if has_tc || msg.role == "assistant" { Some(String::new()) } else { None }
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
                if let Ok(tcs) = serde_json::from_value::<Vec<llm_client::ToolCall>>(tc_json.clone()) {
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

    pub async fn send_message(
        &self,
        conv_id: i32,
        user_id: i32,
        content: &str,
        model_id: Option<i32>,
        state: &UserState,
        tx: mpsc::Sender<SseEvent>,
    ) -> Result<(), ApiError> {
        if content.len() > MAX_MESSAGE_LENGTH {
            return Err(ApiError::BadRequest(format!(
                "Message too long. Max {} characters.",
                MAX_MESSAGE_LENGTH
            )));
        }

        let model_key: Option<String> = if let Some(mid) = model_id {
            let model = state.config_service.get_ai_model_by_id(mid).await
                .map_err(|e| ApiError::DatabaseError(e.to_string()))?
                .ok_or_else(|| ApiError::BadRequest(format!("AI model id={} not found", mid)))?;
            if !model.is_active {
                return Err(ApiError::BadRequest(format!("AI model '{}' is not active", model.name)));
            }
            if model.model_type != "chat" {
                return Err(ApiError::BadRequest(format!("AI model '{}' is not a chat model", model.name)));
            }
            tracing::info!("AI Chat using model override: {} (id={})", model.model_key, mid);
            Some(model.model_key)
        } else {
            tracing::info!("AI Chat using default model (no override)");
            None
        };

        self.repo.get_conversation(conv_id, user_id)?
            .ok_or(ApiError::NotFound("Conversation not found".into()))?;

        let _user_msg = self.repo.create_message(&NewAiMessage {
            conversation_id: conv_id,
            role: "user".into(),
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            plan_id: None,
        })?;

        let history = self.repo.list_messages(conv_id, MAX_CONTEXT_MESSAGES as i64)?;
        let mut messages = self.build_context(&history);

        let tools = ToolRegistry::openai_tools();
        let mut tool_call_count = 0;
        let mut total_usage = llm_client::LlmUsage::default();

        loop {
            let (stream_tx, mut stream_rx) = mpsc::channel::<llm_client::LlmStreamEvent>(64);
            let llm = self.llm.clone();
            let msgs_clone = messages.clone();
            let tools_clone = tools.clone();
            let model_override = model_key.clone();

            let stream_handle = tokio::spawn(async move {
                llm.chat_stream(&msgs_clone, &tools_clone, stream_tx, model_override.as_deref()).await
            });

            let mut text_content = String::new();

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

            let assembled_tool_calls = stream_handle.await
                .map_err(|e| ApiError::AiServiceError(format!("Stream task failed: {}", e)))?
                .map_err(|e| ApiError::AiServiceError(e))?;

            if let Some(ref tool_calls) = assembled_tool_calls {
                if !tool_calls.is_empty() {
                    // Persist assistant message with tool_calls
                    let tc_json: Value = serde_json::to_value(tool_calls).unwrap_or_default();
                    self.repo.create_message(&NewAiMessage {
                        conversation_id: conv_id,
                        role: "assistant".into(),
                        content: text_content.clone(),
                        tool_calls: Some(tc_json),
                        tool_call_id: None,
                        plan_id: None,
                    })?;

                    messages.push(llm_client::ChatMessage {
                        role: "assistant".into(),
                        content: if text_content.is_empty() { Some(String::new()) } else { Some(text_content.clone()) },
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    // Execute tools - parallel for ReadOnly, sequential for mutations
                    let (readonly_calls, mutation_calls): (Vec<_>, Vec<_>) = tool_calls.iter().partition(|tc| {
                        ToolRegistry::get_safety_level(&tc.function.name) == SafetyLevel::ReadOnly
                    });

                    // Parallel execution of ReadOnly tools
                    if !readonly_calls.is_empty() {
                        let mut handles = Vec::new();
                        for tc in &readonly_calls {
                            tool_call_count += 1;
                            if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                                let _ = tx.send(SseEvent::Error {
                                    message: "Too many tool calls in this turn".into(),
                                }).await;
                                break;
                            }
                            let _ = tx.send(SseEvent::ToolCallStart {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.function.name.clone(),
                            }).await;

                            let params: Value = serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                            let name = tc.function.name.clone();
                            let tc_id = tc.id.clone();
                            let st = state.clone();
                            handles.push(tokio::spawn(async move {
                                let result = ToolRegistry::execute(&name, params, user_id, &st).await;
                                (tc_id, name, result)
                            }));
                        }
                        let results = futures::future::join_all(handles).await;
                        for join_result in results {
                            if let Ok((tc_id, name, tool_result)) = join_result {
                                self.process_tool_result(conv_id, user_id, &tc_id, &name, tool_result, &tx, &mut messages).await?;
                            }
                        }
                    }

                    // Sequential execution of mutation tools
                    for tc in &mutation_calls {
                        tool_call_count += 1;
                        if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                            let _ = tx.send(SseEvent::Error {
                                message: "Too many tool calls in this turn".into(),
                            }).await;
                            break;
                        }
                        let _ = tx.send(SseEvent::ToolCallStart {
                            tool_call_id: tc.id.clone(),
                            tool_name: tc.function.name.clone(),
                        }).await;

                        let params: Value = serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        let tool_result = ToolRegistry::execute(&tc.function.name, params, user_id, state).await;
                        self.process_tool_result(conv_id, user_id, &tc.id, &tc.function.name, tool_result, &tx, &mut messages).await?;
                    }

                    if tool_call_count > MAX_TOOL_CALLS_PER_TURN {
                        break;
                    }
                    continue;
                }
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

            let _ = tx.send(SseEvent::MessageEnd {
                message_id: assistant_msg.id,
                finish_reason: "stop".into(),
            }).await;

            break;
        }

        self.repo.touch_conversation(conv_id)?;
        Ok(())
    }

    /// Process a single tool result: audit, SSE events, plan creation, message persistence
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
            error_message: if success { None } else { Some(result_value["error"].as_str().unwrap_or("").to_string()) },
        });

        let _ = tx.send(SseEvent::ToolCallResult {
            tool_call_id: tc_id.to_string(),
            result: result_value.clone(),
            success,
        }).await;

        if tool_name == "create_plan_proposal" && success {
            if let Ok(proposal) = serde_json::from_value::<PlanProposal>(result_value.clone()) {
                let plan = self.create_plan_from_proposal(conv_id, user_id, &proposal)?;
                let steps = self.repo.get_plan_steps(plan.id)?;
                let _ = tx.send(SseEvent::PlanCreated {
                    plan_id: plan.id,
                    title: proposal.title.clone(),
                    steps: steps.iter().map(|s| PlanStepSse {
                        step_id: s.id,
                        step_order: s.step_order,
                        tool_name: s.tool_name.clone(),
                        description: s.description.clone(),
                        tool_params: s.tool_params.clone(),
                    }).collect(),
                }).await;
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

    fn create_plan_from_proposal(&self, conv_id: i32, user_id: i32, proposal: &PlanProposal) -> Result<AiPlan, ApiError> {
        let plan = self.repo.create_plan(&NewAiPlan {
            conversation_id: conv_id,
            message_id: None,
            user_id,
            title: proposal.title.clone(),
            description: proposal.description.clone(),
            status: "draft".into(),
        })?;

        let steps: Vec<NewAiPlanStep> = proposal.steps.iter().enumerate().map(|(i, s)| {
            NewAiPlanStep {
                plan_id: plan.id,
                step_order: i as i32 + 1,
                tool_name: s.tool_name.clone(),
                tool_params: s.tool_params.clone(),
                description: s.description.clone(),
                status: "pending".into(),
            }
        }).collect();

        self.repo.create_plan_steps(&steps)?;
        Ok(plan)
    }

    pub fn get_plan(&self, plan_id: i32, user_id: i32) -> Result<PlanDetailDto, ApiError> {
        let plan = self.repo.get_plan(plan_id, user_id)?
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
        let plan = self.repo.get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest(format!("Plan status is '{}', expected 'draft'", plan.status)));
        }

        self.repo.update_plan(plan_id, user_id, &UpdateAiPlan {
            status: Some("executing".into()),
            ..Default::default()
        })?;

        let steps = self.repo.get_plan_steps(plan_id)?;
        let mut all_success = true;

        for step in &steps {
            let _ = tx.send(SseEvent::StepStart {
                step_id: step.id,
                step_order: step.step_order,
                description: step.description.clone(),
            }).await;

            self.repo.update_plan_step(step.id, &UpdateAiPlanStep {
                status: Some("executing".into()),
                ..Default::default()
            })?;

            let result = ToolRegistry::execute(&step.tool_name, step.tool_params.clone(), user_id, state).await;

            let _ = self.repo.log_tool_call(&NewAiToolAuditLog {
                user_id,
                conversation_id: Some(plan.conversation_id),
                tool_name: step.tool_name.clone(),
                safety_level: ToolRegistry::get_safety_level(&step.tool_name).as_str().to_string(),
                success: result.is_ok(),
                error_message: result.as_ref().err().map(|e| e.to_string()),
            });

            match result {
                Ok(val) => {
                    self.repo.update_plan_step(step.id, &UpdateAiPlanStep {
                        status: Some("completed".into()),
                        result: Some(val.clone()),
                        ..Default::default()
                    })?;
                    let _ = tx.send(SseEvent::StepCompleted {
                        step_id: step.id,
                        result: val,
                    }).await;
                }
                Err(e) => {
                    all_success = false;
                    self.repo.update_plan_step(step.id, &UpdateAiPlanStep {
                        status: Some("failed".into()),
                        error_message: Some(Some(e.to_string())),
                        ..Default::default()
                    })?;
                    let _ = tx.send(SseEvent::StepFailed {
                        step_id: step.id,
                        error: e.to_string(),
                    }).await;
                    break;
                }
            }
        }

        let final_status = if all_success { "completed" } else { "failed" };
        self.repo.update_plan(plan_id, user_id, &UpdateAiPlan {
            status: Some(final_status.into()),
            ..Default::default()
        })?;

        let _ = tx.send(SseEvent::PlanCompleted {
            plan_id,
            summary: if all_success { "所有步骤执行成功".into() } else { "部分步骤执行失败".into() },
        }).await;

        Ok(())
    }

    pub fn cancel_plan(&self, plan_id: i32, user_id: i32) -> Result<PlanDetailDto, ApiError> {
        let plan = self.repo.get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest("Can only cancel draft plans".into()));
        }

        let plan = self.repo.update_plan(plan_id, user_id, &UpdateAiPlan {
            status: Some("cancelled".into()),
            ..Default::default()
        })?;

        let steps = self.repo.get_plan_steps(plan_id)?;
        Ok(PlanDetailDto::from_plan_and_steps(plan, steps))
    }

    pub fn update_plan_step(&self, plan_id: i32, step_id: i32, user_id: i32, req: UpdatePlanStepRequest) -> Result<PlanStepDto, ApiError> {
        let plan = self.repo.get_plan(plan_id, user_id)?
            .ok_or(ApiError::NotFound("Plan not found".into()))?;

        if plan.status != "draft" {
            return Err(ApiError::BadRequest("Can only modify draft plan steps".into()));
        }

        let update = UpdateAiPlanStep {
            tool_params: req.tool_params,
            description: req.description,
            ..Default::default()
        };

        let step = self.repo.update_plan_step(step_id, &update)?;
        if step.plan_id != plan_id {
            return Err(ApiError::NotFound("Step does not belong to this plan".into()));
        }

        Ok(PlanStepDto::from(step))
    }
}
