use serde::{Deserialize, Serialize};
use validator::Validate;

/// Agent analysis request
#[derive(Debug, Deserialize, Validate)]
pub struct AgentAnalysisRequest {
    /// Template ID (used to get prompts and associated campaign)
    #[validate(range(min = 1))]
    pub template_id: i32,

    /// Formatted data context (contains post and comment information)
    /// Similar to Instagram workflow format
    #[validate(length(min = 1))]
    pub data_context: String,

    /// User's additional instructions (optional)
    pub user_instruction: Option<String>,
}

// Removed CommentInput, using data_context string instead

/// Agent analysis response
#[derive(Debug, Serialize)]
pub struct AgentAnalysisResponse {
    /// AI analysis result
    pub analysis: AgentAnalysisResult,

    /// Campaign info used
    pub campaign_info: CampaignInfo,

    /// Template info used
    pub template_info: TemplateInfo,
}

/// AI analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentAnalysisResult {
    pub suggestions: Vec<CommentSuggestion>,
}

/// Suggestion for a single comment
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentSuggestion {
    pub comment_id: String,
    pub reason: String,
    pub suggested_reply: Option<String>,
    pub suggested_dm: Option<String>,
    pub suggested_reply_post: Option<String>,
}

/// Campaign info (for response)
#[derive(Debug, Serialize)]
pub struct CampaignInfo {
    pub id: i32,
    pub name: String,
    pub platform_id: i32,
    pub product_description: Option<String>,
}

/// Template info (for response)
#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub id: i32,
    pub name: String,
    pub persona_prompt: Option<String>,
    pub target_audience: Option<String>,
    pub reply_strategy_prompt: Option<String>,
    pub dm_prompt: Option<String>,
    pub reply_post_prompt: Option<String>,
}
