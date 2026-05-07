use crate::config::database::Database;
use crate::dto::ai_dto::AiGenerateRequest;
use crate::dto::template_dto::{
    AssignReusableTemplateDto, ReusableTemplateCreateDto, ReusableTemplateReadDto,
    ReusableTemplateUpdateDto, TemplateCreateDto, TemplateReadDto, TemplateUpdateDto,
};
use crate::error::db_error::DbError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::template_repository::{
    AssignReusableTemplateError, CampaignTemplateInsert, CampaignTemplatePatch,
    CampaignTemplateWriteError, ReusableTemplateInsert, ReusableTemplatePatch, TemplateRepository,
};
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

    fn merge_reusable_template(
        mut template: TemplateReadDto,
        reusable: Option<ReusableTemplateReadDto>,
    ) -> TemplateReadDto {
        if let Some(reusable) = reusable {
            template.name = Some(reusable.name.clone());
            template.dm_prompt = reusable.dm_prompt.clone();
            template.reply_prompt = reusable.reply_prompt.clone();
            template.reply_post_prompt = reusable.reply_post_prompt.clone();
            template.reusable_template = Some(reusable);
        }
        template
    }

    fn normalize_page_request(
        req: crate::dto::common::PageRequest,
    ) -> crate::dto::common::PageRequest {
        crate::dto::common::PageRequest {
            page: req.page.max(1),
            page_size: req.page_size.clamp(1, 100),
            group_id: req.group_id,
        }
    }

    fn map_campaign_template_write_error(error: CampaignTemplateWriteError) -> ApiError {
        match error {
            CampaignTemplateWriteError::ReplyTemplateIdsFull => ApiError::BadRequest(
                "reply_template_ids cannot contain more than 100 unique IDs".to_string(),
            ),
            CampaignTemplateWriteError::Diesel(error) => {
                ApiError::from(DbError::SomethingWentWrong(error.to_string()))
            }
        }
    }

    async fn hydrate_template_for_user(
        &self,
        user_id: i32,
        template: TemplateReadDto,
    ) -> Result<TemplateReadDto, ApiError> {
        let reusable = if let Some(library_id) = template.library_template_id {
            match self
                .template_repo
                .find_reusable_by_id_and_user(library_id, user_id)
                .await
            {
                Ok(template) => Some(ReusableTemplateReadDto::from(template)),
                Err(DieselError::NotFound) => None,
                Err(e) => {
                    return Err(ApiError::from(DbError::SomethingWentWrong(e.to_string())));
                }
            }
        } else {
            None
        };

        Ok(Self::merge_reusable_template(template, reusable))
    }

    async fn hydrate_templates_for_user(
        &self,
        user_id: i32,
        templates: Vec<TemplateReadDto>,
    ) -> Result<Vec<TemplateReadDto>, ApiError> {
        let library_ids: Vec<i32> = templates
            .iter()
            .filter_map(|template| template.library_template_id)
            .collect();
        let reusable_by_id = self
            .template_repo
            .find_reusable_by_ids_and_user(&library_ids, user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(templates
            .into_iter()
            .map(|template| {
                let reusable = template
                    .library_template_id
                    .and_then(|id| reusable_by_id.get(&id).cloned())
                    .map(ReusableTemplateReadDto::from);
                Self::merge_reusable_template(template, reusable)
            })
            .collect())
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

        let library_template_id = dto.library_template_id;
        let reusable = if let Some(library_id) = library_template_id {
            Some(self.get_reusable_template(user_id, library_id).await?)
        } else {
            None
        };

        let template = self
            .template_repo
            .create(CampaignTemplateInsert {
                campaign_id,
                library_template_id,
                name: dto
                    .name
                    .or_else(|| reusable.as_ref().map(|template| template.name.clone())),
                weight: dto.weight,
                dm_prompt: dto.dm_prompt.or_else(|| {
                    reusable
                        .as_ref()
                        .and_then(|template| template.dm_prompt.clone())
                }),
                reply_prompt: dto.reply_prompt.or_else(|| {
                    reusable
                        .as_ref()
                        .and_then(|template| template.reply_prompt.clone())
                }),
                reply_post_prompt: dto.reply_post_prompt.or_else(|| {
                    reusable
                        .as_ref()
                        .and_then(|template| template.reply_post_prompt.clone())
                }),
            })
            .await
            .map_err(Self::map_campaign_template_write_error)?;

        self.hydrate_template_for_user(user_id, TemplateReadDto::from(template))
            .await
    }

    pub async fn get_templates(
        &self,
        user_id: i32,
        campaign_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<TemplateReadDto>, ApiError> {
        let req = Self::normalize_page_request(req);
        self.check_campaign_ownership(campaign_id, user_id).await?;

        let (templates, total) = self
            .template_repo
            .find_all_by_campaign(campaign_id, req.page, req.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = self
            .hydrate_templates_for_user(
                user_id,
                templates.into_iter().map(TemplateReadDto::from).collect(),
            )
            .await?;
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
            .find_resolved_by_id(template_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        // Verify ownership
        self.check_campaign_ownership(template.campaign_id, user_id)
            .await?;

        self.hydrate_template_for_user(user_id, TemplateReadDto::from(template))
            .await
    }

    pub async fn get_all_templates(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<TemplateReadDto>, ApiError> {
        let req = Self::normalize_page_request(req);
        let (templates, total) = self
            .template_repo
            .find_all_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = self
            .hydrate_templates_for_user(
                user_id,
                templates.into_iter().map(TemplateReadDto::from).collect(),
            )
            .await?;
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

        let library_template_id = dto.library_template_id;
        let reusable = if let Some(library_id) = library_template_id.flatten() {
            Some(self.get_reusable_template(user_id, library_id).await?)
        } else {
            None
        };

        let updated = self
            .template_repo
            .update(CampaignTemplatePatch {
                id: template_id,
                library_template_id,
                name: match dto.name {
                    Some(Some(name)) => Some(Some(name)),
                    Some(None) => Some(None),
                    None => library_template_id.flatten().and_then(|_| {
                        reusable
                            .as_ref()
                            .map(|template| Some(template.name.clone()))
                    }),
                },
                weight: dto.weight,
                dm_prompt: match dto.dm_prompt {
                    Some(value) => Some(value),
                    None => library_template_id
                        .flatten()
                        .and_then(|_| reusable.as_ref().map(|template| template.dm_prompt.clone())),
                },
                reply_prompt: match dto.reply_prompt {
                    Some(value) => Some(value),
                    None => library_template_id.flatten().and_then(|_| {
                        reusable
                            .as_ref()
                            .map(|template| template.reply_prompt.clone())
                    }),
                },
                reply_post_prompt: match dto.reply_post_prompt {
                    Some(value) => Some(value),
                    None => library_template_id.flatten().and_then(|_| {
                        reusable
                            .as_ref()
                            .map(|template| template.reply_post_prompt.clone())
                    }),
                },
            })
            .await
            .map_err(Self::map_campaign_template_write_error)?;

        self.hydrate_template_for_user(user_id, TemplateReadDto::from(updated))
            .await
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
            .delete_and_sync(template_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }

    pub async fn create_reusable_template(
        &self,
        user_id: i32,
        dto: ReusableTemplateCreateDto,
    ) -> Result<ReusableTemplateReadDto, ApiError> {
        let template = self
            .template_repo
            .create_reusable(ReusableTemplateInsert {
                user_id,
                name: dto.name,
                description: dto.description,
                weight: dto.weight.unwrap_or(50),
                dm_prompt: dto.dm_prompt,
                reply_prompt: dto.reply_prompt,
                reply_post_prompt: dto.reply_post_prompt,
            })
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(ReusableTemplateReadDto::from(template))
    }

    pub async fn get_reusable_template(
        &self,
        user_id: i32,
        template_id: i32,
    ) -> Result<ReusableTemplateReadDto, ApiError> {
        let template = self
            .template_repo
            .find_reusable_by_id_and_user(template_id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        Ok(ReusableTemplateReadDto::from(template))
    }

    pub async fn list_reusable_templates(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<ReusableTemplateReadDto>, ApiError> {
        let req = Self::normalize_page_request(req);
        let (templates, total) = self
            .template_repo
            .find_all_reusable_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let dtos = templates
            .into_iter()
            .map(ReusableTemplateReadDto::from)
            .collect();
        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn update_reusable_template(
        &self,
        user_id: i32,
        template_id: i32,
        dto: ReusableTemplateUpdateDto,
    ) -> Result<ReusableTemplateReadDto, ApiError> {
        self.template_repo
            .update_reusable(ReusableTemplatePatch {
                id: template_id,
                user_id,
                name: dto.name,
                description: dto.description,
                weight: dto.weight,
                dm_prompt: dto.dm_prompt,
                reply_prompt: dto.reply_prompt,
                reply_post_prompt: dto.reply_post_prompt,
            })
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        self.get_reusable_template(user_id, template_id).await
    }

    pub async fn delete_reusable_template(
        &self,
        user_id: i32,
        template_id: i32,
    ) -> Result<(), ApiError> {
        let deleted = self
            .template_repo
            .delete_reusable(template_id, user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        if deleted == 0 {
            return Err(ApiError::BusinessError(BusinessError::TemplateNotFound));
        }

        Ok(())
    }

    pub async fn assign_reusable_template(
        &self,
        user_id: i32,
        library_template_id: i32,
        dto: AssignReusableTemplateDto,
    ) -> Result<TemplateReadDto, ApiError> {
        let assigned_template = self
            .template_repo
            .assign_reusable_to_campaign(dto.campaign_id, library_template_id, dto.weight, user_id)
            .await;

        let template = assigned_template.map_err(|e| match e {
            AssignReusableTemplateError::ReplyTemplateIdsFull => ApiError::BadRequest(
                "reply_template_ids cannot contain more than 100 unique IDs".to_string(),
            ),
            AssignReusableTemplateError::CampaignNotFound => {
                ApiError::BusinessError(BusinessError::TemplatePermissionDenied)
            }
            AssignReusableTemplateError::TemplateNotFound => {
                ApiError::BusinessError(BusinessError::TemplateNotFound)
            }
            AssignReusableTemplateError::Diesel(DieselError::NotFound) => {
                ApiError::BusinessError(BusinessError::TemplateNotFound)
            }
            AssignReusableTemplateError::Diesel(e) => {
                ApiError::from(DbError::SomethingWentWrong(e.to_string()))
            }
        })?;

        let reusable = self
            .get_reusable_template(user_id, library_template_id)
            .await?;

        Ok(Self::merge_reusable_template(
            TemplateReadDto::from(template),
            Some(reusable),
        ))
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
                            library_template_id: None,
                            reusable_template: None,
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
                    library_template_id: None,
                    reusable_template: None,
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
