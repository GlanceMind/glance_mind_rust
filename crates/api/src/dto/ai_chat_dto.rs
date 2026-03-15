use chrono::{DateTime, Utc};
use glance_mind_db::entity::ai_chat::*;
use crate::service::ai_chat::QuestionnaireSubmission;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Request DTOs ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub model_id: Option<i32>,
    pub ui_capabilities: Option<AiChatUiCapabilities>,
    pub questionnaire_submission: Option<QuestionnaireSubmission>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AiChatUiCapabilities {
    #[serde(default)]
    pub questionnaire: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanStepRequest {
    pub tool_params: Option<Value>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListConversationsQuery {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

// ── Response DTOs ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDto {
    pub id: i32,
    pub title: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<AiConversation> for ConversationDto {
    fn from(c: AiConversation) -> Self {
        Self {
            id: c.id,
            title: c.title,
            status: c.status,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDto {
    pub id: i32,
    pub conversation_id: i32,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub tool_call_id: Option<String>,
    pub plan_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl From<AiMessage> for MessageDto {
    fn from(m: AiMessage) -> Self {
        Self {
            id: m.id,
            conversation_id: m.conversation_id,
            role: m.role,
            content: m.content,
            tool_calls: m.tool_calls,
            tool_call_id: m.tool_call_id,
            plan_id: m.plan_id,
            created_at: m.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanDetailDto {
    pub id: i32,
    pub conversation_id: i32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub steps: Vec<PlanStepDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl PlanDetailDto {
    pub fn from_plan_and_steps(plan: AiPlan, steps: Vec<AiPlanStep>) -> Self {
        Self {
            id: plan.id,
            conversation_id: plan.conversation_id,
            title: plan.title,
            description: plan.description,
            status: plan.status,
            steps: steps.into_iter().map(PlanStepDto::from).collect(),
            created_at: plan.created_at,
            updated_at: plan.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStepDto {
    pub id: i32,
    pub step_order: i32,
    pub tool_name: String,
    pub tool_params: Value,
    pub description: String,
    pub status: String,
    pub result: Option<Value>,
    pub error_message: Option<String>,
}

impl From<AiPlanStep> for PlanStepDto {
    fn from(s: AiPlanStep) -> Self {
        Self {
            id: s.id,
            step_order: s.step_order,
            tool_name: s.tool_name,
            tool_params: s.tool_params,
            description: s.description,
            status: s.status,
            result: s.result,
            error_message: s.error_message,
        }
    }
}
