use crate::config::database::Database;
use crate::dto::ai_dto::AiGenerateRequest;
use crate::dto::template_dto::{TemplateCreateDto, TemplateReadDto, TemplateUpdateDto};
use crate::error::db_error::DbError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::template_repository::TemplateRepository;
use crate::service::ai_service::AiService;
use diesel::result::Error as DieselError;
use std::sync::Arc;

#[derive(Clone)]
pub struct TemplateService {
    campaign_repo: CampaignRepository,
    template_repo: TemplateRepository,
}

impl TemplateService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            campaign_repo: CampaignRepository::new(db_conn.pool.clone()),
            template_repo: TemplateRepository::new(db_conn.pool.clone()),
        }
    }

    // Verify ownership helper
    async fn check_campaign_ownership(
        &self,
        campaign_id: i32,
        user_id: i32,
    ) -> Result<(), ApiError> {
        match self
            .campaign_repo
            .find_by_id_and_user(campaign_id, user_id)
            .await
        {
            Ok(_) => Ok(()),
            Err(DieselError::NotFound) => Err(ApiError::BusinessError(
                BusinessError::TemplatePermissionDenied,
            )),
            Err(e) => Err(ApiError::from(DbError::SomethingWentWrong(e.to_string()))),
        }
    }

    pub async fn create_template(
        &self,
        user_id: i32,
        campaign_id: i32,
        dto: TemplateCreateDto,
    ) -> Result<TemplateReadDto, ApiError> {
        self.check_campaign_ownership(campaign_id, user_id).await?;

        let template = self
            .template_repo
            .create(
                campaign_id,
                dto.name,
                dto.weight,
                dto.dm_prompt,
                dto.reply_prompt,
                dto.reply_post_prompt,
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(TemplateReadDto::from(template))
    }

    pub async fn get_templates(
        &self,
        user_id: i32,
        campaign_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<TemplateReadDto>, ApiError> {
        self.check_campaign_ownership(campaign_id, user_id).await?;

        let (templates, total) = self
            .template_repo
            .find_all_by_campaign(campaign_id, req.page, req.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = templates.into_iter().map(TemplateReadDto::from).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn get_template(
        &self,
        user_id: i32,
        template_id: i32,
    ) -> Result<TemplateReadDto, ApiError> {
        let template = self
            .template_repo
            .find_by_id(template_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        // Verify ownership
        self.check_campaign_ownership(template.campaign_id, user_id)
            .await?;

        Ok(TemplateReadDto::from(template))
    }

    pub async fn get_all_templates(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<TemplateReadDto>, ApiError> {
        let (templates, total) = self
            .template_repo
            .find_all_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = templates.into_iter().map(TemplateReadDto::from).collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn update_template(
        &self,
        user_id: i32,
        template_id: i32,
        dto: TemplateUpdateDto,
    ) -> Result<TemplateReadDto, ApiError> {
        // First verify the template exists and belongs to a campaign owned by the user.
        // Optimization: fetch template first, check campaign_id, then check campaign ownership.
        let template = self
            .template_repo
            .find_by_id(template_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        self.check_campaign_ownership(template.campaign_id, user_id)
            .await?;

        let updated = self
            .template_repo
            .update(
                template_id,
                dto.name,
                dto.weight,
                dto.dm_prompt,
                dto.reply_prompt,
                dto.reply_post_prompt,
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(TemplateReadDto::from(updated))
    }

    pub async fn delete_template(&self, user_id: i32, template_id: i32) -> Result<(), ApiError> {
        // Similarity check ownership
        let template = self
            .template_repo
            .find_by_id(template_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        self.check_campaign_ownership(template.campaign_id, user_id)
            .await?;

        self.template_repo
            .delete(template_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }

    pub async fn auto_generate_templates(
        &self,
        _user_id: i32,
        product_description: &str,
        target_audience: &str,
        style_preference: &str,
        count: i32,
    ) -> Result<Vec<TemplateReadDto>, ApiError> {
        let product = if product_description.is_empty() {
            "General product"
        } else {
            product_description
        };
        let audience = if target_audience.is_empty() {
            "General audience"
        } else {
            target_audience
        };

        let style_desc = match style_preference {
            "professional" => "professional, knowledge-driven, data-backed, trust-building",
            "humorous" => "humorous, witty, playful, high-engagement, meme-worthy",
            "concise" => "concise, direct, efficient, action-oriented",
            _ => "friendly, warm, relatable, natural-sounding",
        };

        let ai_req = AiGenerateRequest {
            platform: "social media".to_string(),
            region: "global".to_string(),
            product_description: product.to_string(),
            target_audience: Some(audience.to_string()),
            generation_type: "REPLY".to_string(),
            reply_requirements: Some(format!(
                r#"Generate exactly {count} reply style(s) as a JSON array.
Product/Service: {product}
Target Audience: {audience}
Desired Tone: {style_desc}

Each object must have:
- "name": a short descriptive name
- "dm_prompt": instructions for AI to send personalized DMs (must include {{{{user_name}}}} placeholder, max 200 words)
- "reply_prompt": instructions for AI to reply to comments (max 200 words)
- "reply_post_prompt": instructions for AI to post standalone comments (max 200 words)

All prompts must:
- Use the {style_desc} tone
- Reference the product naturally
- Specify "Match the language of the user's comment/post"
- Include specific rules (max reply count, sentence limits)

Return ONLY valid JSON array. No markdown, no explanation."#
            )),
        };

        match AiService::generate(ai_req).await {
            Ok(response) => {
                let cleaned = response
                    .content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(cleaned) {
                    let templates: Vec<TemplateReadDto> = parsed
                        .into_iter()
                        .enumerate()
                        .take(count as usize)
                        .map(|(i, item)| TemplateReadDto {
                            id: (i + 1) as i32,
                            campaign_id: 0,
                            name: item
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(String::from)
                                .or_else(|| Some(format!("Template #{}", i + 1))),
                            weight: 1,
                            dm_prompt: item
                                .get("dm_prompt")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            reply_prompt: item
                                .get("reply_prompt")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            reply_post_prompt: item
                                .get("reply_post_prompt")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            created_at: chrono::Utc::now(),
                            updated_at: None,
                        })
                        .collect();
                    return Ok(templates);
                }

                tracing::warn!("Failed to parse AI response as JSON array, building single template from raw text");
                Ok(vec![TemplateReadDto {
                    id: 1,
                    campaign_id: 0,
                    name: Some("AI Generated Template".to_string()),
                    weight: 1,
                    dm_prompt: Some(cleaned.to_string()),
                    reply_prompt: None,
                    reply_post_prompt: None,
                    created_at: chrono::Utc::now(),
                    updated_at: None,
                }])
            }
            Err(e) => {
                tracing::error!("AI template generation failed: {}", e);
                Err(ApiError::from(DbError::SomethingWentWrong(format!(
                    "AI generation failed: {}",
                    e
                ))))
            }
        }
    }
}
