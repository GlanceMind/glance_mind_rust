use crate::config::database::Database;
use crate::dto::template_dto::{TemplateCreateDto, TemplateReadDto, TemplateUpdateDto};
use crate::error::db_error::DbError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::template_repository::TemplateRepository;
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
        product_info: serde_json::Value,
        _ai_model_id: i32,
        count: i32,
    ) -> Result<Vec<TemplateReadDto>, ApiError> {
        // TODO: Integrate with real AI model
        // For now, generate mock templates
        let mut templates = Vec::new();

        for i in 0..count {
            templates.push(TemplateReadDto {
                id: i + 1,      // Mock i32 ID
                campaign_id: 0, // Mock campaign ID
                weight: 1,
                dm_prompt: Some(format!(
                    "Auto-generated DM #{}: Check out this amazing product! {}",
                    i + 1,
                    product_info
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Product")
                )),
                reply_prompt: Some("Engaging and friendly reply".to_string()),
                reply_post_prompt: Some("Thoughtful post reply".to_string()),
                created_at: chrono::Utc::now(),
                updated_at: None,
            });
        }

        Ok(templates)
    }
}
