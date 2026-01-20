use crate::config::database::Database;
use crate::dto::campaign_dto::{
    CampaignCreateDto, CampaignLogDto, CampaignReadDto, CampaignUpdateDto,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::wallet_repository::WalletRepository;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::campaign::{Campaign, NewCampaign};
use std::sync::Arc;

#[derive(Clone)]
pub struct CampaignService {
    repo: CampaignRepository,
    wallet_repo: WalletRepository,
}

impl CampaignService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: CampaignRepository::new(db.pool.clone()),
            wallet_repo: WalletRepository::new(db.pool.clone()),
        }
    }

    pub async fn create_campaign(
        &self,
        user_id: i32,
        dto: CampaignCreateDto,
    ) -> Result<CampaignReadDto, ApiError> {
        let new_campaign = NewCampaign {
            user_id,
            name: dto.name,
            status: Some("DRAFT".to_string()),
            target_audience: dto.target_audience,
            schedule_config: dto.schedule_config,
            platform_id: dto.platform_id,
            region_id: dto.region_id,
            enable_ai_refactor: dto.enable_ai_refactor,
            ai_model_id: dto.ai_model_id,
            persona_id: dto.persona_id,
            max_scan_count: dto.max_scan_count,
            budget_cap: dto.budget_cap,
            end_date: dto.end_date,
            schedule_type: dto.schedule_type,
            product_prompt: dto.product_prompt,
            keyword: dto.keyword,
            call_to_action: dto.call_to_action,
            tone_of_voice: dto.tone_of_voice,
            additional_info: dto.additional_info,
            social_group_id: dto.social_group_id,
            auto_like: dto.auto_like,
            auto_follow: dto.auto_follow,
            auto_dm: dto.auto_dm,
            auto_reply_comments: dto.auto_reply_comments,
            auto_reply_post: dto.auto_reply_post,
            search_options: dto.search_options,
        };

        let campaign = self.repo.create(new_campaign).await.map_err(|e| {
            tracing::error!("Failed to create campaign: {:?}", e);
            ApiError::InternalServerError("Failed to create campaign".to_string())
        })?;

        Ok(self.to_dto(campaign))
    }

    pub async fn get_campaign(&self, id: i32, user_id: i32) -> Result<CampaignReadDto, ApiError> {
        let campaign = self
            .repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::CampaignNotFound),
                e => {
                    tracing::error!("Failed to fetch campaign: {:?}", e);
                    ApiError::InternalServerError("Failed to fetch campaign".to_string())
                }
            })?;

        // Fetch statistics
        let total_scans = self.repo.get_total_scans(id).await.unwrap_or(0);
        let ai_replies = self.repo.get_ai_replies_count(id).await.unwrap_or(0);

        Ok(self.to_dto_with_stats(campaign, total_scans, ai_replies))
    }

    pub async fn list_campaigns(
        &self,
        user_id: i32,
        req: crate::dto::common::PageRequest,
    ) -> Result<crate::dto::common::PageResponse<CampaignReadDto>, ApiError> {
        let (campaigns, total) = self
            .repo
            .find_by_user(user_id, req.page, req.page_size)
            .await
            .map_err(|e| {
                tracing::error!("Failed to list campaigns: {:?}", e);
                ApiError::InternalServerError("Failed to list campaigns".to_string())
            })?;

        // Fetch stats for each campaign
        let mut dtos = Vec::new();
        for campaign in campaigns {
            let campaign_id = campaign.id;
            let total_scans = self.repo.get_total_scans(campaign_id).await.unwrap_or(0);
            let ai_replies = self
                .repo
                .get_ai_replies_count(campaign_id)
                .await
                .unwrap_or(0);
            dtos.push(self.to_dto_with_stats(campaign, total_scans, ai_replies));
        }

        Ok(crate::dto::common::PageResponse::new(
            dtos,
            total,
            req.page,
            req.page_size,
        ))
    }

    pub async fn update_campaign(
        &self,
        id: i32,
        user_id: i32,
        dto: CampaignUpdateDto,
    ) -> Result<CampaignReadDto, ApiError> {
        // First verify ownership
        let existing = self
            .repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|_| ApiError::BusinessError(BusinessError::CampaignNotFound))?;

        // Build changeset
        let changeset = NewCampaign {
            user_id: existing.user_id,
            name: dto.name.unwrap_or(existing.name),
            status: Some(existing.status),
            target_audience: dto.target_audience.or(existing.target_audience),
            schedule_config: dto.schedule_config.or(existing.schedule_config),
            platform_id: dto.platform_id.unwrap_or(existing.platform_id),
            region_id: dto.region_id.unwrap_or(existing.region_id),
            enable_ai_refactor: dto.enable_ai_refactor.or(existing.enable_ai_refactor),
            ai_model_id: dto.ai_model_id.unwrap_or(existing.ai_model_id),
            persona_id: existing.persona_id,
            max_scan_count: dto.max_scan_count.or(existing.max_scan_count),
            budget_cap: dto.budget_cap.or(existing.budget_cap),
            end_date: dto.end_date.or(existing.end_date),
            schedule_type: dto.schedule_type.unwrap_or(existing.schedule_type),
            product_prompt: dto.product_prompt.unwrap_or(existing.product_prompt),
            keyword: dto.keyword.or(existing.keyword),
            call_to_action: dto.call_to_action.or(existing.call_to_action),
            tone_of_voice: dto.tone_of_voice.or(existing.tone_of_voice),
            additional_info: dto.additional_info.or(existing.additional_info),
            social_group_id: dto.social_group_id.or(existing.social_group_id),
            auto_like: dto.auto_like.or(Some(existing.auto_like)),
            auto_follow: dto.auto_follow.or(Some(existing.auto_follow)),
            auto_dm: dto.auto_dm.or(Some(existing.auto_dm)),
            auto_reply_comments: dto
                .auto_reply_comments
                .or(Some(existing.auto_reply_comments)),
            auto_reply_post: dto.auto_reply_post.or(Some(existing.auto_reply_post)),
            search_options: dto.search_options.or(existing.search_options),
        };

        let updated = self
            .repo
            .update(id, user_id, &changeset)
            .await
            .map_err(|e| {
                tracing::error!("Failed to update campaign: {:?}", e);
                ApiError::InternalServerError("Failed to update campaign".to_string())
            })?;

        Ok(self.to_dto(updated))
    }

    pub async fn update_status(
        &self,
        id: i32,
        user_id: i32,
        new_status: &str,
    ) -> Result<CampaignReadDto, ApiError> {
        // Validate status
        match new_status {
            "ACTIVE" | "PAUSED" | "COMPLETED" | "ARCHIVED" => {}
            _ => return Err(ApiError::BusinessError(BusinessError::InvalidStatus)),
        }

        // Get current campaign
        let campaign = self
            .repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::CampaignNotFound),
                e => {
                    tracing::error!("Failed to fetch campaign: {:?}", e);
                    ApiError::InternalServerError("Failed to fetch campaign".to_string())
                }
            })?;

        // Budget freezing logic: only freeze on first activation
        if new_status == "ACTIVE" && !campaign.is_frozen {
            // First activation - need to freeze budget
            if let Some(budget_cap) = &campaign.budget_cap {
                // Validate available balance
                let has_balance = self
                    .wallet_repo
                    .validate_available_balance(user_id, budget_cap)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to validate balance: {:?}", e);
                        ApiError::InternalServerError("Failed to validate balance".to_string())
                    })?;

                if !has_balance {
                    return Err(ApiError::BusinessError(
                        BusinessError::InsufficientBalanceForCampaign,
                    ));
                }

                // Freeze budget in wallet
                self.wallet_repo
                    .freeze_campaign_budget(user_id, budget_cap)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to freeze budget: {:?}", e);
                        ApiError::InternalServerError("Failed to freeze budget".to_string())
                    })?;

                tracing::info!(
                    "Froze budget {} for campaign {} (user {})",
                    budget_cap,
                    id,
                    user_id
                );
            }

            // Update campaign status and set is_frozen = true
            let updated = self
                .repo
                .update_status_and_freeze(id, user_id, new_status, true)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to update status: {:?}", e);
                    ApiError::InternalServerError("Failed to update status".to_string())
                })?;

            Ok(self.to_dto(updated))
        } else {
            // Resume from PAUSED or other status change - just update status
            let updated = self
                .repo
                .update_status(id, user_id, new_status)
                .await
                .map_err(|e| match e {
                    DieselError::NotFound => {
                        ApiError::BusinessError(BusinessError::CampaignNotFound)
                    }
                    e => {
                        tracing::error!("Failed to update status: {:?}", e);
                        ApiError::InternalServerError("Failed to update status".to_string())
                    }
                })?;

            Ok(self.to_dto(updated))
        }
    }

    pub async fn get_campaign_logs(
        &self,
        _id: i32,
        _user_id: i32,
    ) -> Result<Vec<CampaignLogDto>, ApiError> {
        // TODO: Implement actual log retrieval from database
        // For now, return empty logs as placeholder
        Ok(vec![])
    }

    fn to_dto(&self, campaign: Campaign) -> CampaignReadDto {
        self.to_dto_with_stats(campaign, 0, 0)
    }

    fn to_dto_with_stats(
        &self,
        campaign: Campaign,
        total_scans: i64,
        ai_replies: i64,
    ) -> CampaignReadDto {
        CampaignReadDto {
            id: campaign.id,
            user_id: campaign.user_id,
            name: campaign.name,
            status: campaign.status,
            platform_id: campaign.platform_id,
            region_id: campaign.region_id,
            ai_model_id: campaign.ai_model_id,
            target_audience: campaign.target_audience,
            enable_ai_refactor: campaign.enable_ai_refactor,
            persona_id: campaign.persona_id,
            max_scan_count: campaign.max_scan_count,
            budget_cap: campaign.budget_cap,
            actual_consumption: campaign.actual_consumption,
            end_date: campaign.end_date,
            schedule_type: campaign.schedule_type,
            schedule_config: campaign.schedule_config,
            created_at: campaign.created_at,
            updated_at: campaign.updated_at,
            product_prompt: campaign.product_prompt,
            keyword: campaign.keyword,
            call_to_action: campaign.call_to_action,
            tone_of_voice: campaign.tone_of_voice,
            additional_info: campaign.additional_info,
            social_group_id: campaign.social_group_id,
            stats: crate::dto::campaign_dto::CampaignStatsDto {
                scans: total_scans as i32,
                replies: ai_replies as i32,
                conversions: 0,
            },
            auto_like: campaign.auto_like,
            auto_follow: campaign.auto_follow,
            auto_dm: campaign.auto_dm,
            auto_reply_comments: campaign.auto_reply_comments,
            auto_reply_post: campaign.auto_reply_post,
            search_options: campaign.search_options,
        }
    }
}
