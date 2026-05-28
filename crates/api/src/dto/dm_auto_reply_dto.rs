use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmConfigResponse {
    pub social_account_id: i32,
    pub enabled: bool,
    pub campaign_id: Option<i32>,
    pub dm_prompt: Option<String>,
    pub product_info: Option<String>,
    pub knowledge_id: Option<String>,
    pub brand_name: String,
    pub blacklist_keywords: Vec<String>,
    pub rate_limit_per_day: i32,
    pub rate_limit_count_today: i32,
    pub confidence_threshold: f32,
    pub rag_threshold: f32,
    pub fallback_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyLogUpsertRequest {
    pub inbound_msg_id: String,
    pub conv_id: String,
    pub social_account_id: i32,
    pub campaign_id: Option<i32>,
    pub user_ref: String,
    pub user_handle: Option<String>,
    pub platform: String,
    pub inbound_text: String,
    pub inbound_received_at: DateTime<Utc>,
    pub status: String,
    #[serde(default)]
    pub skip_reason: Option<String>,
    #[serde(default)]
    pub rag_score: Option<f32>,
    #[serde(default)]
    pub llm_confidence: Option<f32>,
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub latency_ms: i32,
    #[serde(default)]
    pub reply_text: Option<String>,
    #[serde(default)]
    pub escalate_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplyLogUpsertResponse {
    pub action: String,
    pub existed_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalReplyRequest {
    pub conv_id: String,
    pub content: String,
    pub inbound_msg_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalReplyResponse {
    pub cmd_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalateRequest {
    pub conv_id: String,
    pub reason: String,
    pub inbound_msg_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalateResponse {
    pub conv_id: String,
    pub needs_human_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResponse {
    pub campaign_id: i32,
    pub user_ref: String,
    pub count: i32,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearReviewResponse {
    pub conv_id: String,
    pub resolved_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_reply_log_id: Option<i64>,
}
