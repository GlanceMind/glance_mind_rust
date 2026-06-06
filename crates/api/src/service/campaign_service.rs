use crate::config::database::{DBPool, Database};
use crate::dto::campaign_dto::{
    CampaignCreateDto, CampaignLogDto, CampaignReadDto, CampaignStatus, CampaignUpdateDto,
};
use crate::dto::campaign_lead_metrics_dto::CampaignLeadMetricsDto;
use crate::error::db_error::DbError;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::campaign_repository::CampaignRepository;
use crate::repository::template_repository::TemplateRepository;
use crate::repository::wallet_repository::WalletRepository;
use chrono::Utc;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::campaign::{Campaign, NewCampaign};
use std::collections::HashSet;
use std::sync::Arc;

use bigdecimal::BigDecimal;

#[derive(Clone)]
pub struct CampaignService {
    repo: CampaignRepository,
    pool: DBPool,
    wallet_repo: WalletRepository,
    template_repo: TemplateRepository,
}

impl CampaignService {
    pub fn new(db: &Arc<Database>) -> Self {
        Self {
            repo: CampaignRepository::new(db.pool.clone()),
            pool: db.pool.clone(),
            wallet_repo: WalletRepository::new(db.pool.clone()),
            template_repo: TemplateRepository::new(db.pool.clone()),
        }
    }

    pub async fn create_campaign(
        &self,
        user_id: i32,
        dto: CampaignCreateDto,
    ) -> Result<CampaignReadDto, ApiError> {
        // Validate schedule_type
        validate_schedule_type(&dto.schedule_type)?;
        validate_schedule_config(&dto.schedule_type, &dto.schedule_config)?;

        // Validate max_scan_count
        let scan_count = dto.max_scan_count.unwrap_or(0);
        if scan_count <= 0 {
            return Err(ApiError::BadRequest(
                "max_scan_count must be greater than 0 / 最大扫描数量必须大于 0".to_string(),
            ));
        }

        // Validate budget_cap against minimum cost using DB stored procedure.
        // Always compute min_cost so we can also check wallet balance up-front.
        let min_cost = self
            .calculate_min_cost(dto.platform_id, scan_count, dto.ai_model_id)
            .await?;

        if let Some(ref cap) = dto.budget_cap {
            if cap < &min_cost {
                return Err(ApiError::BadRequest(format!(
                    "Budget too low: minimum {} points required / 预算不足：最低需要 {} 积分",
                    min_cost, min_cost
                )));
            }
        }

        // Balance gate: even though budget freeze happens on activation, require the
        // user's available wallet balance to cover the greater of min_cost or
        // budget_cap at creation time. This keeps UX consistent with other apps and
        // lets the front-end show the unified insufficient-balance popup before the
        // user invests time in drafting a campaign they can't activate.
        let required = match &dto.budget_cap {
            Some(cap) if cap > &min_cost => cap.clone(),
            _ => min_cost.clone(),
        };
        let has_balance = self
            .wallet_repo
            .validate_available_balance(user_id, &required)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check wallet balance for campaign: {:?}", e);
                ApiError::InternalServerError("Failed to check balance".to_string())
            })?;
        if !has_balance {
            return Err(ApiError::InsufficientBalance);
        }

        let reply_template_ids = normalize_reply_template_ids(dto.reply_template_ids)?;
        self.validate_reply_template_ownership(user_id, &reply_template_ids)
            .await?;

        // Create campaign in DRAFT status (budget not frozen yet)
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
            reply_template_ids: reply_template_ids.clone(),
            // Module D3: link back to the assistant task-template draft this
            // campaign was confirmed from (when present).
            source_draft_id: dto.source_draft_id,
        };

        let campaign = self
            .repo
            .create_with_reply_template_ids(new_campaign, &reply_template_ids)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::TemplateNotFound),
                e => {
                    tracing::error!("Failed to create campaign: {:?}", e);
                    ApiError::InternalServerError("Failed to create campaign".to_string())
                }
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

    /// Aggregate engagement feedback (lead metrics) for a campaign.
    ///
    /// Window:
    /// - start = `campaign.created_at`
    /// - end   = `min(now, campaign.end_date)` (end_date is TIMESTAMPTZ — see Task 0.7.4)
    ///
    /// Semantics: **associated engagement**, NOT caused engagement. See DTO docs.
    pub async fn get_lead_metrics(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<CampaignLeadMetricsDto, ApiError> {
        let campaign = self
            .repo
            .find_by_id_and_user(id, user_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::CampaignNotFound),
                e => {
                    tracing::error!("Failed to fetch campaign for lead metrics: {:?}", e);
                    ApiError::InternalServerError("Failed to fetch campaign".to_string())
                }
            })?;

        let window_start = campaign.created_at;
        let now = Utc::now();

        // end_date is TIMESTAMPTZ per Task 0.7.4 — no +1 day adjustment needed.
        let window_end = match campaign.end_date {
            Some(end) if end < now => end,
            _ => now,
        };

        tracing::debug!(
            campaign_id = id,
            window_start = %window_start,
            window_end = %window_end,
            "aggregating campaign lead metrics"
        );

        let agg = self
            .repo
            .aggregate_lead_metrics(id, window_start, window_end)
            .await
            .map_err(|e| {
                tracing::error!("Failed to aggregate lead metrics: {:?}", e);
                ApiError::InternalServerError("Failed to aggregate lead metrics".to_string())
            })?;

        Ok(CampaignLeadMetricsDto {
            campaign_id: id,
            window_start_at: window_start,
            window_end_at: window_end,
            account_count: agg.account_count,
            tracked_account_count: agg.tracked_account_count,
            new_followers: agg.new_followers,
            dms: agg.dms,
            friend_requests: agg.friend_requests,
            mentions: agg.mentions,
            received_likes: agg.received_likes,
            received_comments: agg.received_comments,
            last_updated_at: agg.last_updated_at,
        })
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

        // Validate schedule_type if provided
        if let Some(ref st) = dto.schedule_type {
            validate_schedule_type(st)?;
            let config = dto
                .schedule_config
                .as_ref()
                .or(existing.schedule_config.as_ref());
            validate_schedule_config(st, &config.cloned())?;
        }

        // Build changeset
        let should_replace_reply_template_ids = dto.reply_template_ids.is_some();
        let reply_template_ids = if let Some(raw_ids) = dto.reply_template_ids {
            let ids = normalize_reply_template_ids(Some(raw_ids))?;
            self.validate_reply_template_ownership(user_id, &ids)
                .await?;
            ids
        } else {
            existing.reply_template_ids.clone()
        };

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
            reply_template_ids: reply_template_ids.clone(),
            // Preserve the original draft link on update (never clobbered by an
            // edit; Module D3).
            source_draft_id: existing.source_draft_id,
        };

        let updated = if should_replace_reply_template_ids {
            self.repo
                .update_with_reply_template_ids(id, user_id, &changeset, &reply_template_ids)
                .await
        } else {
            self.repo.update(id, user_id, &changeset).await
        }
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
        new_status: CampaignStatus,
    ) -> Result<CampaignReadDto, ApiError> {
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

        let status_str = new_status.to_string();

        match new_status {
            // Handle STOPPED/COMPLETED - use stored procedure for graceful stop
            CampaignStatus::Stopped | CampaignStatus::Completed => {
                let result = self.repo.stop_gracefully(id).await.map_err(|e| {
                    tracing::error!("Failed to stop campaign gracefully: {:?}", e);
                    ApiError::InternalServerError("Failed to stop campaign".to_string())
                })?;

                if !result.success {
                    return Err(ApiError::InternalServerError(
                        "Failed to stop campaign".to_string(),
                    ));
                }

                tracing::info!(
                    "Campaign {} stopped gracefully. Immediate: {}, Refunded: {}",
                    id,
                    result.immediate_stopped,
                    result.refunded_amount
                );

                // Fetch updated campaign
                let updated = self
                    .repo
                    .find_by_id_and_user(id, user_id)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to fetch updated campaign: {:?}", e);
                        ApiError::InternalServerError("Failed to fetch campaign".to_string())
                    })?;

                Ok(self.to_dto(updated))
            }

            // Handle PAUSED - just update status
            CampaignStatus::Paused => {
                let updated = self
                    .repo
                    .update_status(id, user_id, &status_str)
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

            // Handle ACTIVE - use stored procedure for first activation (freezes budget)
            CampaignStatus::Active => {
                // Check if this is first activation (not frozen yet)
                if !campaign.is_frozen {
                    // First activation - use stored procedure to freeze budget
                    let result = self.repo.activate_campaign(id).await.map_err(|e| {
                        tracing::error!("Failed to activate campaign: {:?}", e);
                        ApiError::InternalServerError("Failed to activate campaign".to_string())
                    })?;

                    if !result.success {
                        // Check if it's insufficient balance
                        if result.message.contains("Insufficient balance") {
                            return Err(ApiError::BusinessError(
                                BusinessError::InsufficientBalanceForCampaign,
                            ));
                        }
                        return Err(ApiError::InternalServerError(result.message));
                    }

                    tracing::info!("Campaign {} activated with budget frozen", id);

                    // Fetch updated campaign
                    let updated =
                        self.repo
                            .find_by_id_and_user(id, user_id)
                            .await
                            .map_err(|e| {
                                tracing::error!("Failed to fetch updated campaign: {:?}", e);
                                ApiError::InternalServerError(
                                    "Failed to fetch campaign".to_string(),
                                )
                            })?;

                    Ok(self.to_dto(updated))
                } else {
                    // Resume from PAUSED - just update status (budget already frozen)
                    let updated = self
                        .repo
                        .update_status(id, user_id, &status_str)
                        .await
                        .map_err(|e| {
                            tracing::error!("Failed to update status: {:?}", e);
                            ApiError::InternalServerError("Failed to update status".to_string())
                        })?;

                    Ok(self.to_dto(updated))
                }
            }

            // Handle ARCHIVED - just update status
            CampaignStatus::Archived => {
                let updated = self
                    .repo
                    .update_status(id, user_id, &status_str)
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

            // DRAFT and STOPPING are not valid target statuses for manual update
            CampaignStatus::Draft | CampaignStatus::Stopping => {
                Err(ApiError::BusinessError(BusinessError::InvalidStatus))
            }
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
            reply_template_ids: campaign.reply_template_ids,
        }
    }

    async fn validate_reply_template_ownership(
        &self,
        user_id: i32,
        reply_template_ids: &[i32],
    ) -> Result<(), ApiError> {
        if reply_template_ids.is_empty() {
            return Ok(());
        }

        let found = self
            .template_repo
            .find_reusable_by_ids_and_user(reply_template_ids, user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;
        if found.len() != reply_template_ids.len() {
            return Err(ApiError::BusinessError(BusinessError::TemplateNotFound));
        }

        Ok(())
    }

    async fn calculate_min_cost(
        &self,
        platform_id: i32,
        scan_count: i32,
        ai_model_id: i32,
    ) -> Result<BigDecimal, ApiError> {
        use diesel::prelude::*;

        let model_id: Option<i32> = if ai_model_id > 0 {
            Some(ai_model_id)
        } else {
            None
        };

        let mut conn = self
            .pool
            .get()
            .map_err(|e| ApiError::InternalServerError(format!("DB pool error: {}", e)))?;

        match diesel::sql_query(
            "SELECT calculate_min_campaign_cost($1, $2, $3) AS calculate_min_campaign_cost",
        )
        .bind::<diesel::sql_types::Integer, _>(platform_id)
        .bind::<diesel::sql_types::Integer, _>(scan_count)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(model_id)
        .get_result::<MinCostRow>(&mut conn)
        {
            Ok(row) => Ok(row.calculate_min_campaign_cost),
            Err(e) => {
                tracing::warn!(
                    "calculate_min_campaign_cost DB function unavailable: {}, using fallback formula",
                    e
                );
                let scan_cost = BigDecimal::from(5);
                let ai_cost = BigDecimal::from(2);
                let effective_scans = BigDecimal::from(std::cmp::max(scan_count, 1));
                Ok((scan_cost + ai_cost) * effective_scans)
            }
        }
    }
}

fn normalize_reply_template_ids(input: Option<Vec<i32>>) -> Result<Vec<i32>, ApiError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for id in input.unwrap_or_default() {
        if id <= 0 {
            return Err(ApiError::BadRequest(
                "reply_template_ids must contain only positive IDs".to_string(),
            ));
        }
        if seen.insert(id) {
            normalized.push(id);
            if normalized.len() > 100 {
                return Err(ApiError::BadRequest(
                    "reply_template_ids cannot contain more than 100 unique IDs".to_string(),
                ));
            }
        }
    }

    Ok(normalized)
}

#[derive(diesel::QueryableByName)]
struct MinCostRow {
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    calculate_min_campaign_cost: BigDecimal,
}

const VALID_SCHEDULE_TYPES: &[&str] = &["CONTINUOUS", "ONCE", "SCHEDULED", "INTERVAL", "CRON"];

fn validate_schedule_type(schedule_type: &str) -> Result<(), ApiError> {
    if !VALID_SCHEDULE_TYPES.contains(&schedule_type) {
        return Err(ApiError::BadRequest(format!(
            "Invalid schedule_type '{}'. Valid values: {} / 无效投放策略 '{}'，可选：{}",
            schedule_type,
            VALID_SCHEDULE_TYPES.join(", "),
            schedule_type,
            VALID_SCHEDULE_TYPES.join(", ")
        )));
    }
    Ok(())
}

fn validate_schedule_config(
    schedule_type: &str,
    config: &Option<serde_json::Value>,
) -> Result<(), ApiError> {
    match schedule_type {
        "CONTINUOUS" | "INTERVAL" => {
            if let Some(cfg) = config {
                if let Some(secs) = cfg.get("interval_seconds").and_then(|v| v.as_i64()) {
                    if !(60..=86400).contains(&secs) {
                        return Err(ApiError::BadRequest(format!(
                            "interval_seconds must be between 60 and 86400 (got {}) / 执行间隔必须在 60-86400 秒之间（当前 {}）",
                            secs, secs
                        )));
                    }
                }
            }
        }
        "SCHEDULED" | "CRON" => {
            if let Some(cfg) = config {
                if let Some(expr) = cfg.get("cron_expression").and_then(|v| v.as_str()) {
                    if expr.is_empty() {
                        return Err(ApiError::BadRequest(
                            "cron_expression cannot be empty / Cron 表达式不能为空".to_string(),
                        ));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
