use crate::schema::{
    gm_ai_conversations, gm_ai_messages, gm_ai_plan_steps, gm_ai_plans, gm_ai_tool_audit_logs,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ---------------------------------------------------------------------------
// Conversation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_ai_conversations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AiConversation {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_conversations)]
pub struct NewAiConversation {
    pub user_id: i32,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = gm_ai_conversations)]
pub struct UpdateAiConversation {
    pub title: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_ai_messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AiMessage {
    pub id: i32,
    pub conversation_id: i32,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<JsonValue>,
    pub tool_call_id: Option<String>,
    pub plan_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_messages)]
pub struct NewAiMessage {
    pub conversation_id: i32,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<JsonValue>,
    pub tool_call_id: Option<String>,
    pub plan_id: Option<i32>,
}

// ---------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_ai_plans)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AiPlan {
    pub id: i32,
    pub conversation_id: i32,
    pub message_id: Option<i32>,
    pub user_id: i32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_plans)]
pub struct NewAiPlan {
    pub conversation_id: i32,
    pub message_id: Option<i32>,
    pub user_id: i32,
    pub title: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = gm_ai_plans)]
pub struct UpdateAiPlan {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Plan Step
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_ai_plan_steps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AiPlanStep {
    pub id: i32,
    pub plan_id: i32,
    pub step_order: i32,
    pub tool_name: String,
    pub tool_params: JsonValue,
    pub description: String,
    pub status: String,
    pub result: Option<JsonValue>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_plan_steps)]
pub struct NewAiPlanStep {
    pub plan_id: i32,
    pub step_order: i32,
    pub tool_name: String,
    pub tool_params: JsonValue,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone, AsChangeset, Default)]
#[diesel(table_name = gm_ai_plan_steps)]
pub struct UpdateAiPlanStep {
    pub tool_params: Option<JsonValue>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub result: Option<JsonValue>,
    pub error_message: Option<Option<String>>,
}

// ---------------------------------------------------------------------------
// Tool Audit Log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = gm_ai_tool_audit_logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AiToolAuditLog {
    pub id: i32,
    pub user_id: i32,
    pub conversation_id: Option<i32>,
    pub tool_name: String,
    pub safety_level: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = gm_ai_tool_audit_logs)]
pub struct NewAiToolAuditLog {
    pub user_id: i32,
    pub conversation_id: Option<i32>,
    pub tool_name: String,
    pub safety_level: String,
    pub success: bool,
    pub error_message: Option<String>,
}
