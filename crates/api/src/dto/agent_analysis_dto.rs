use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use validator::{Validate, ValidationError};

/// Agent analysis request
#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_analysis_source"))]
pub struct AgentAnalysisRequest {
    /// Campaign template ID (used to get prompts and associated campaign)
    #[validate(range(min = 1))]
    pub template_id: Option<i32>,

    /// Campaign ID used as context when testing a reusable library template
    #[validate(range(min = 1))]
    pub campaign_id: Option<i32>,

    /// Reusable library template ID to test with the selected campaign context
    #[validate(range(min = 1))]
    pub library_template_id: Option<i32>,

    /// Formatted data context (contains post and comment information)
    /// Similar to Instagram workflow format
    #[validate(length(min = 1))]
    pub data_context: String,

    /// User's additional instructions (optional)
    pub user_instruction: Option<String>,
}

fn validate_analysis_source(req: &AgentAnalysisRequest) -> Result<(), ValidationError> {
    let has_campaign_template = req.template_id.is_some();
    let has_reusable_pair = req.campaign_id.is_some() && req.library_template_id.is_some();
    let has_partial_reusable = req.campaign_id.is_some() || req.library_template_id.is_some();

    if has_campaign_template && !has_partial_reusable {
        return Ok(());
    }

    if !has_campaign_template && has_reusable_pair {
        return Ok(());
    }

    let mut error = ValidationError::new("invalid_analysis_source");
    error.message = Some(Cow::Borrowed(
        "provide either template_id or both campaign_id and library_template_id",
    ));
    Err(error)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        template_id: Option<i32>,
        campaign_id: Option<i32>,
        library_template_id: Option<i32>,
    ) -> AgentAnalysisRequest {
        AgentAnalysisRequest {
            template_id,
            campaign_id,
            library_template_id,
            data_context: "Post: demo\nComment: interested".to_string(),
            user_instruction: Some("测试模板效果".to_string()),
        }
    }

    #[test]
    fn campaign_template_source_remains_valid() {
        let req = request(Some(12), None, None);

        assert!(req.validate().is_ok());
    }

    #[test]
    fn reusable_template_source_requires_campaign_context() {
        let req = request(None, Some(34), Some(56));

        assert!(req.validate().is_ok());
    }

    #[test]
    fn library_template_without_campaign_is_invalid() {
        let req = request(None, None, Some(56));

        let err = req
            .validate()
            .expect_err("partial reusable source should fail");
        assert!(err
            .to_string()
            .contains("provide either template_id or both campaign_id and library_template_id"));
    }

    #[test]
    fn campaign_without_any_template_source_is_invalid() {
        let req = request(None, Some(34), None);

        let err = req
            .validate()
            .expect_err("missing template source should fail");
        assert!(err
            .to_string()
            .contains("provide either template_id or both campaign_id and library_template_id"));
    }
}
