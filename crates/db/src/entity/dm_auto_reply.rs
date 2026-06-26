use crate::schema::{
    gm_auto_reply_config, gm_dm_conversation_review, gm_dm_reply_log, gm_product_faq,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_auto_reply_config)]
pub struct AutoReplyConfig {
    pub id: i32,
    pub social_account_id: i32,
    pub enabled: bool,
    pub blacklist_keywords: Vec<Option<String>>,
    pub rate_limit_per_day: i32,
    pub confidence_threshold: f32,
    pub rag_threshold: f32,
    pub fallback_strategy: String,
    pub brand_name: String,
    pub last_modified_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_modified_by_admin_user_id: Option<i32>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_auto_reply_config)]
pub struct NewAutoReplyConfig {
    pub social_account_id: i32,
    pub enabled: bool,
    pub blacklist_keywords: Vec<Option<String>>,
    pub rate_limit_per_day: i32,
    pub confidence_threshold: f32,
    pub rag_threshold: f32,
    pub fallback_strategy: String,
    pub brand_name: String,
    pub last_modified_by: Option<i32>,
    pub last_modified_by_admin_user_id: Option<i32>,
}

#[derive(AsChangeset, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_auto_reply_config)]
pub struct UpdateAutoReplyConfig {
    pub enabled: Option<bool>,
    pub blacklist_keywords: Option<Vec<Option<String>>>,
    pub rate_limit_per_day: Option<i32>,
    pub confidence_threshold: Option<f32>,
    pub rag_threshold: Option<f32>,
    pub fallback_strategy: Option<String>,
    pub brand_name: Option<String>,
    pub last_modified_by: Option<i32>,
    pub last_modified_by_admin_user_id: Option<i32>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_dm_reply_log)]
pub struct DmReplyLog {
    pub id: i64,
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
    pub skip_reason: Option<String>,
    pub rag_score: Option<f32>,
    pub llm_confidence: Option<f32>,
    pub llm_model: Option<String>,
    pub latency_ms: i32,
    pub reply_text: Option<String>,
    pub escalate_reason: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_dm_reply_log)]
pub struct NewDmReplyLog {
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
    pub skip_reason: Option<String>,
    pub rag_score: Option<f32>,
    pub llm_confidence: Option<f32>,
    pub llm_model: Option<String>,
    pub latency_ms: i32,
    pub reply_text: Option<String>,
    pub escalate_reason: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_product_faq)]
pub struct ProductFaq {
    pub id: i32,
    pub campaign_id: i32,
    pub question: String,
    pub answer: String,
    pub chunk_type: String,
    pub dify_kb_id: Option<String>,
    pub dify_doc_id: Option<String>,
    pub sync_status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_product_faq)]
pub struct NewProductFaq {
    pub campaign_id: i32,
    pub question: String,
    pub answer: String,
    pub chunk_type: String,
    pub dify_kb_id: Option<String>,
    pub dify_doc_id: Option<String>,
    pub sync_status: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_dm_conversation_review, primary_key(conv_id))]
pub struct DmConversationReview {
    pub conv_id: String,
    pub needs_human_review: bool,
    pub human_review_reason: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<i32>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = gm_dm_conversation_review)]
pub struct NewDmConversationReview {
    pub conv_id: String,
    pub needs_human_review: bool,
    pub human_review_reason: Option<String>,
}
