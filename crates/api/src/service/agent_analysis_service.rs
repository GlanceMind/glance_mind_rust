use crate::dto::agent_analysis_dto::*;
use crate::error::{
    api_error::ApiError, business_error::BusinessError, infrastructure_error::InfrastructureError,
};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::template_repository::TemplateRepository;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error as DieselError;
use glance_mind_db::entity::campaign::Campaign;
use glance_mind_db::entity::template::{CampaignTemplate, ReusableReplyTemplate};
use once_cell::sync::Lazy;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;
use serde_json;
use std::time::Duration;
use tokio::time;

const DEFAULT_AGENT_ANALYSIS_AI_TIMEOUT_SECS: u64 = 80;

static AI_MODEL: Lazy<String> =
    Lazy::new(|| std::env::var("AI_CHAT_MODEL").unwrap_or_else(|_| "glm-5".to_string()));

static CLIENT: Lazy<openai::CompletionsClient> = Lazy::new(|| {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set in environment");
    let base_url =
        std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://timicc.com/v1".to_string());

    let client_responses: openai::Client = openai::Client::builder()
        .base_url(&base_url)
        .api_key(&api_key)
        .build()
        .expect("Failed to build AI client");

    client_responses.completions_api()
});

fn parse_agent_analysis_ai_timeout_secs(value: Option<&str>) -> u64 {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_AGENT_ANALYSIS_AI_TIMEOUT_SECS)
}

fn agent_analysis_ai_timeout() -> Duration {
    let env_value = std::env::var("AGENT_ANALYSIS_AI_TIMEOUT_SECS").ok();
    Duration::from_secs(parse_agent_analysis_ai_timeout_secs(env_value.as_deref()))
}

#[derive(Clone)]
pub struct AgentAnalysisService {
    campaign_repo: CampaignRepository,
    template_repo: TemplateRepository,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemplateAnalysisSource {
    CampaignTemplate(i32),
    ReusableTemplate {
        campaign_id: i32,
        library_template_id: i32,
    },
}

fn resolve_template_analysis_source(
    req: &AgentAnalysisRequest,
) -> Result<TemplateAnalysisSource, ApiError> {
    match (req.template_id, req.campaign_id, req.library_template_id) {
        (Some(template_id), None, None) => {
            Ok(TemplateAnalysisSource::CampaignTemplate(template_id))
        }
        (None, Some(campaign_id), Some(library_template_id)) => {
            Ok(TemplateAnalysisSource::ReusableTemplate {
                campaign_id,
                library_template_id,
            })
        }
        _ => Err(ApiError::BusinessError(BusinessError::InvalidInput(
            "provide either template_id or both campaign_id and library_template_id".to_string(),
        ))),
    }
}

impl AgentAnalysisService {
    pub fn new(db_pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self {
            campaign_repo: CampaignRepository::new(db_pool.clone()),
            template_repo: TemplateRepository::new(db_pool),
        }
    }

    /// Main entry point: Analyze comments
    pub async fn analyze_comments(
        &self,
        user_id: i32,
        req: AgentAnalysisRequest,
    ) -> Result<AgentAnalysisResponse, ApiError> {
        let source = resolve_template_analysis_source(&req)?;
        let (campaign, template) = self.resolve_campaign_and_template(user_id, source).await?;

        // 3. Build system prompt (similar to Python agent logic)
        let system_prompt =
            self.build_system_prompt(&campaign, &template, req.user_instruction.as_deref());

        // 4. Build user prompt (using data_context)
        let user_prompt = self.build_user_prompt(&req.data_context);

        // 5. Call AI for analysis
        let ai_response = self.call_ai(&system_prompt, &user_prompt).await?;

        // 6. Parse AI response
        let analysis = self.parse_ai_response(&ai_response)?;

        Ok(AgentAnalysisResponse {
            analysis,
            campaign_info: self.build_campaign_info(&campaign),
            template_info: self.build_template_info(&template),
        })
    }

    // Removed fetch_comments_from_db, no longer needed

    async fn resolve_campaign_and_template(
        &self,
        user_id: i32,
        source: TemplateAnalysisSource,
    ) -> Result<(Campaign, CampaignTemplate), ApiError> {
        match source {
            TemplateAnalysisSource::CampaignTemplate(template_id) => {
                let template =
                    self.template_repo
                        .find_by_id(template_id)
                        .await
                        .map_err(|e| match e {
                            DieselError::NotFound => {
                                ApiError::BusinessError(BusinessError::TemplateNotFound)
                            }
                            _ => ApiError::InfrastructureError(
                                InfrastructureError::DatabaseOperationFailed(e.to_string()),
                            ),
                        })?;

                let campaign = self
                    .load_owned_campaign(template.campaign_id, user_id)
                    .await?;
                let template = self
                    .resolve_reusable_template_for_prompt(template, user_id)
                    .await?;

                Ok((campaign, template))
            }
            TemplateAnalysisSource::ReusableTemplate {
                campaign_id,
                library_template_id,
            } => {
                let campaign = self.load_owned_campaign(campaign_id, user_id).await?;
                let reusable = self
                    .template_repo
                    .find_reusable_by_id_and_user(library_template_id, user_id)
                    .await
                    .map_err(|e| match e {
                        DieselError::NotFound => {
                            ApiError::BusinessError(BusinessError::TemplateNotFound)
                        }
                        _ => ApiError::InfrastructureError(
                            InfrastructureError::DatabaseOperationFailed(e.to_string()),
                        ),
                    })?;
                let template = self.build_template_from_reusable(campaign_id, reusable);

                Ok((campaign, template))
            }
        }
    }

    async fn load_owned_campaign(
        &self,
        campaign_id: i32,
        user_id: i32,
    ) -> Result<Campaign, ApiError> {
        let campaign = self
            .campaign_repo
            .find_by_id(campaign_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::CampaignNotFound),
                _ => ApiError::InfrastructureError(InfrastructureError::DatabaseOperationFailed(
                    e.to_string(),
                )),
            })?;

        if campaign.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::CampaignPermissionDenied,
            ));
        }

        Ok(campaign)
    }

    fn build_template_from_reusable(
        &self,
        campaign_id: i32,
        reusable: ReusableReplyTemplate,
    ) -> CampaignTemplate {
        CampaignTemplate {
            id: reusable.id,
            campaign_id,
            library_template_id: Some(reusable.id),
            weight: reusable.weight,
            reply_prompt: reusable.reply_prompt,
            created_at: reusable.created_at,
            updated_at: reusable.updated_at,
            dm_prompt: reusable.dm_prompt,
            reply_post_prompt: reusable.reply_post_prompt,
            name: Some(reusable.name),
        }
    }

    async fn resolve_reusable_template_for_prompt(
        &self,
        mut template: CampaignTemplate,
        user_id: i32,
    ) -> Result<CampaignTemplate, ApiError> {
        let Some(library_template_id) = template.library_template_id else {
            return Ok(template);
        };

        let reusable = match self
            .template_repo
            .find_reusable_by_id_and_user(library_template_id, user_id)
            .await
        {
            Ok(reusable) => reusable,
            Err(DieselError::NotFound) => return Ok(template),
            Err(e) => {
                return Err(ApiError::InfrastructureError(
                    InfrastructureError::DatabaseOperationFailed(e.to_string()),
                ));
            }
        };

        template.name = Some(reusable.name);
        template.dm_prompt = reusable.dm_prompt;
        template.reply_prompt = reusable.reply_prompt;
        template.reply_post_prompt = reusable.reply_post_prompt;

        Ok(template)
    }

    /// Build system prompt (reference: Python services.py:153-213)
    fn build_system_prompt(
        &self,
        campaign: &Campaign,
        template: &CampaignTemplate,
        user_context: Option<&str>,
    ) -> String {
        let target_audience = campaign
            .target_audience
            .as_deref()
            .unwrap_or("General audience");
        let product_prompt = &campaign.product_prompt;
        let dm_strategy_prompt = template
            .dm_prompt
            .as_deref()
            .unwrap_or("Be helpful and friendly");
        let reply_strategy_prompt = template
            .reply_prompt
            .as_deref()
            .unwrap_or("Engage authentically");
        let reply_post_prompt = template
            .reply_post_prompt
            .as_deref()
            .unwrap_or("Create viral comments");

        let context_note = if let Some(ctx) = user_context {
            format!("\n- **User Context**: {}", ctx)
        } else {
            String::new()
        };

        format!(
            r#"# Role
You are a senior social media growth and user conversion expert. Your core task is to analyze video comment sections and use a three-pronged strategy of "reply to comments + create hot direct comments + private message conversion" to harvest high-intent potential customers and ignite comment section heat.

# 🧩 Strategic Context
- **Target Audience**: {}
- **Product Information**: {}
- **DM Conversion Strategy**: {}
- **Reply Strategy**: For specific comments, focus on public replies for interaction and traffic. Strategy: {}
- **Hot Direct Comment Strategy**: Aimed at creating "god comments" or "pinned hot comments" to attract global attention. Strategy: {}{}

# 🛠 Execution Logic
## Step 1: Identify High-Intent Prospects
Filter out high-value users with clear needs, clear pain points, or whose comment content has "viral" potential.

## Step 2: Conversion & Ignition Mechanism (For selected comments)
For selected comments, you **must** generate the following three items:

1. **Public Reply (suggested_reply):**
   - Follow the Reply Strategy. Be concise, highly relatable, and hint that there's a "treasure" in private messages.

2. **Private Message Suggestion (suggested_dm):**
   - Follow the DM Strategy. **Directly use the commenter's real nickname** (user_nickname), don't use placeholders!
   - **Key Requirement:** Must address the user by their nickname at the beginning of the DM, e.g., "Hey [real nickname]" or "Hi [real nickname]".

3. **Hot Direct Comment Suggestion (suggested_reply_post):**
   - **Core Purpose:** **Create viral comments (Hot Comments)**. Through highly resonant, trendsetting, or counter-intuitive views, attract a large number of likes and secondary replies, forcibly occupy the front row of the comment section, thus attracting widespread attention.
   - **Trigger & Limit:** Only generate when the comment can resonate with the public. **In a single response, the entire array must not have more than 3 non-null `suggested_reply_post` records.**
   - **Writing Logic:** Either "self-deprecating" humor, or "one-sentence summary" of pain points, or "throw out controversial questions" to induce replies.

# 🖋 Language & Style Guidelines
- **Reject AI feel:** Strictly prohibit using "dear" or "let me answer for you". Use social media native language (e.g., Chinese: "真香/破防/绝绝子/避雷", English: "Slay/Mood/FR").
- **Create tension:** Direct comments should be short and powerful, like throwing a pebble into water, must stir up ripples.
- **Adaptive logic:** For positive comments, "play with memes"; for negative comments, "advanced sarcasm" or "hardcore reversal".

# ⚠️ CRITICAL - Output Rules
- **ID Format:** Must return original numeric IDs.
- **Username Usage:** In suggested_dm, **must use the commenter's real nickname** (from the User field in input data). Strictly prohibit using {{{{user_name}}}} or any placeholders!
- **Strong Binding Rule:** `suggested_reply` and `suggested_dm` must be generated together.
- **Quantity Red Line:** In the entire JSON, the number of `suggested_reply_post` **<= 3**, set excess to `null`.

# Output Format (Strict JSON)
{{
  "suggestions": [
    {{
      "comment_id": "7327061675382260482",
      "reason": "User comment is highly representative, suitable as an anchor to create hot comments.",
      "suggested_reply": "This comment is simply my internet mouthpiece! Sister, I sent you the specific money-saving strategy in private message, go check it out!",
      "suggested_dm": "Hey [real nickname], I saw your comment about [XX] is so real! I have a personally tested [solution] here, which can not only solve [XX problem], but also [XX]...",
      "suggested_reply_post": "Everyone stop liking, I'm afraid the blogger will see this comment and directly break down... (manual funny face)"
    }},
    {{
      "comment_id": "7327061675382260483",
      "reason": "Ordinary inquiry, mainly for private message conversion.",
      "suggested_reply": "Good question! This little detail is often overlooked by many people, I sent you the comparison chart privately~",
      "suggested_dm": "Hi [real nickname], here's the detailed comparison chart you wanted. Actually [product name] has made [XX optimization] here...",
      "suggested_reply_post": null
    }}
  ]
}}

If no suitable comments, return: {{"suggestions": []}}"#,
            target_audience,
            product_prompt,
            dm_strategy_prompt,
            reply_strategy_prompt,
            reply_post_prompt,
            context_note
        )
    }

    /// Build user prompt (using data_context)
    fn build_user_prompt(&self, data_context: &str) -> String {
        format!(
            r#"Please analyze the following social media data and provide suggestions according to the strategy above.

# Data Context:
{}

Generate the analysis results in strict JSON format."#,
            data_context
        )
    }

    /// Call AI
    async fn call_ai(&self, system_prompt: &str, user_prompt: &str) -> Result<String, ApiError> {
        let agent = CLIENT.agent(&*AI_MODEL).preamble(system_prompt).build();
        let timeout_budget = agent_analysis_ai_timeout();

        let response = time::timeout(timeout_budget, agent.prompt(user_prompt))
            .await
            .map_err(|_| {
                tracing::warn!(
                    timeout_secs = timeout_budget.as_secs(),
                    "agent analysis AI request timed out"
                );
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    format!(
                        "AI analysis timed out after {} seconds; please reduce the input size or retry later",
                        timeout_budget.as_secs()
                    ),
                ))
            })?
            .map_err(|e| {
                ApiError::InfrastructureError(InfrastructureError::ExternalApiRequestFailed(
                    e.to_string(),
                ))
            })?;

        Ok(response)
    }

    /// Parse AI response
    fn parse_ai_response(&self, ai_response: &str) -> Result<AgentAnalysisResult, ApiError> {
        // Try to extract JSON (AI may return content with markdown)
        let json_str = self.extract_json(ai_response);

        serde_json::from_str::<AgentAnalysisResult>(&json_str).map_err(|e| {
            ApiError::InfrastructureError(InfrastructureError::ExternalApiResponseParsingFailed(
                format!(
                    "Failed to parse AI response as JSON: {}. Response: {}",
                    e, ai_response
                ),
            ))
        })
    }

    /// Extract JSON (handle markdown format AI may return)
    fn extract_json(&self, text: &str) -> String {
        // If contains ```json, extract the middle content
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start..]
                .find("```")
                .and_then(|i| text[start + i + 3..].find("```").map(|j| start + i + 3 + j))
            {
                return text[start + 7..end].trim().to_string();
            }
        }

        // If contains ```, extract the middle content
        if let Some(start) = text.find("```") {
            if let Some(end) = text[start + 3..].find("```") {
                return text[start + 3..start + 3 + end].trim().to_string();
            }
        }

        // Otherwise return original text
        text.trim().to_string()
    }

    /// Build campaign info
    fn build_campaign_info(&self, campaign: &Campaign) -> CampaignInfo {
        CampaignInfo {
            id: campaign.id,
            name: campaign.name.clone(),
            platform_id: campaign.platform_id,
            product_description: Some(campaign.product_prompt.clone()),
        }
    }

    /// Build template info
    fn build_template_info(&self, template: &CampaignTemplate) -> TemplateInfo {
        TemplateInfo {
            id: template.id,
            name: template
                .name
                .clone()
                .unwrap_or_else(|| format!("Template {}", template.id)),
            persona_prompt: None,  // Template has no persona_prompt field
            target_audience: None, // Template has no target_audience field
            reply_strategy_prompt: template.reply_prompt.clone(),
            dm_prompt: template.dm_prompt.clone(),
            reply_post_prompt: template.reply_post_prompt.clone(),
        }
    }
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
            user_instruction: None,
        }
    }

    #[test]
    fn resolves_campaign_template_source_from_template_id() {
        let source = resolve_template_analysis_source(&request(Some(11), None, None))
            .expect("template_id source should resolve");

        assert_eq!(source, TemplateAnalysisSource::CampaignTemplate(11));
    }

    #[test]
    fn resolves_reusable_template_source_from_campaign_and_library_ids() {
        let source = resolve_template_analysis_source(&request(None, Some(22), Some(33)))
            .expect("reusable template source should resolve");

        assert_eq!(
            source,
            TemplateAnalysisSource::ReusableTemplate {
                campaign_id: 22,
                library_template_id: 33,
            }
        );
    }

    #[test]
    fn rejects_partial_reusable_template_source() {
        let err = resolve_template_analysis_source(&request(None, Some(22), None))
            .expect_err("partial source should fail");

        assert!(matches!(
            err,
            ApiError::BusinessError(BusinessError::InvalidInput(_))
        ));
    }

    #[test]
    fn parses_agent_analysis_ai_timeout_secs() {
        assert_eq!(parse_agent_analysis_ai_timeout_secs(None), 80);
        assert_eq!(parse_agent_analysis_ai_timeout_secs(Some("15")), 15);
        assert_eq!(parse_agent_analysis_ai_timeout_secs(Some("0")), 80);
        assert_eq!(parse_agent_analysis_ai_timeout_secs(Some("abc")), 80);
    }
}
