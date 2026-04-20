//! AI Publish Module Service
//! 自动发布模块业务逻辑层

use crate::config::database::Database;
use crate::dto::aipub_dto::*;
use crate::dto::common::PageResponse;
use crate::error::api_error::ApiError;
use crate::error::business_error::BusinessError;
use crate::error::db_error::DbError;
use crate::repository::aipub_repository::AipubRepository;
use crate::service::{
    behavior_validation, image_generation_validation, reddit_validation, seedance_validation,
};
use chrono::Utc;
use diesel::result::Error as DieselError;
use glance_mind_db::entity::aipub::{
    AiTaskStatus, AiTaskType, NewAipubPlan, NewAipubTask, PlanStatus, PlanType, PublishTaskStatus,
    UpdateAipubAiTask, UpdateAipubPlan, UpdateAipubTask,
};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AipubService {
    pub repo: AipubRepository,
}

impl AipubService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            repo: AipubRepository::new(db_conn.pool.clone()),
        }
    }

    // =========================================================================
    // Plan Service Methods
    // =========================================================================

    /// Create a new publish plan
    pub async fn create_plan(
        &self,
        user_id: i32,
        dto: CreatePlanDto,
    ) -> Result<PlanResponseDto, ApiError> {
        // Determine plan_type: default to "batch_text" if not specified
        let plan_type = dto
            .plan_type
            .clone()
            .unwrap_or_else(|| PlanType::BatchText.as_str().to_string());

        // Validate plan_type
        if PlanType::parse(&plan_type).is_none() {
            return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                format!("Invalid plan_type: {}. Supported: batch_text, single_video, account_grooming, reddit_text, reddit_image, reddit_link, direct_publish", plan_type),
            )));
        }

        if let Err(err) = self.repo.get_platform_name(dto.platform_id) {
            return match err {
                DieselError::NotFound => Err(ApiError::BusinessError(BusinessError::InvalidInput(
                    format!("Invalid platform_id: {}", dto.platform_id),
                ))),
                _ => Err(ApiError::from(DbError::SomethingWentWrong(err.to_string()))),
            };
        }

        // Validate ai_input.image_generations[] (V2 ImageGenerationSpec).
        // No-op when the field is absent (legacy V1 plans).
        if let Some(ai_input) = dto.ai_input.as_ref() {
            image_generation_validation::validate(ai_input)?;
        }

        // Phase 4 R3 Task 7a — validate plan-level PublishBehavior shape
        // before persisting. No-op when behavior is absent.
        behavior_validation::validate(dto.behavior.as_ref())?;

        // Validate target based on plan_type
        match PlanType::parse(plan_type.as_str()) {
            Some(PlanType::BatchText) => {
                // batch_text requires group_id (multiple accounts)
                if dto.group_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "batch_text plan requires group_id".to_string(),
                    )));
                }
                if dto.social_account_id.is_some() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "batch_text plan cannot have social_account_id, use group_id instead"
                            .to_string(),
                    )));
                }
            }
            Some(PlanType::SingleVideo) => {
                // single_video requires social_account_id (single account)
                if dto.social_account_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "single_video plan requires social_account_id".to_string(),
                    )));
                }
                if dto.group_id.is_some() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "single_video plan cannot have group_id, use social_account_id instead"
                            .to_string(),
                    )));
                }
                // single_video requires video_ai_model_id
                if dto.video_ai_model_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "single_video plan requires video_ai_model_id".to_string(),
                    )));
                }
                // Seedance-specific validation when seedance_config is present
                if seedance_validation::is_seedance_plan(&dto.ai_input) {
                    if let Some(ref ai_input) = dto.ai_input {
                        seedance_validation::validate_seedance_config(ai_input)?;
                    }
                }
            }
            Some(PlanType::AccountGrooming) => {
                // account_grooming requires group_id (generate profiles for all accounts in group)
                if dto.group_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "account_grooming plan requires group_id".to_string(),
                    )));
                }
                if dto.social_account_id.is_some() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "account_grooming plan cannot have social_account_id, use group_id instead"
                            .to_string(),
                    )));
                }
            }
            Some(pt) if pt.is_reddit() => {
                // All Reddit types require group_id (batch posting to multiple accounts)
                if dto.group_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        format!("{} plan requires group_id", plan_type),
                    )));
                }
                // Typed Reddit validation (moved to dedicated module to
                // drop inline string-key JSON indexing). Handles subreddit /
                // link_url / image_prompt / uploaded_image_urls + content_prompt.
                reddit_validation::validate(pt, dto.ai_input.as_ref())?;
            }
            Some(PlanType::DirectPublish) => {
                if dto.social_account_id.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "direct_publish plan requires social_account_id".to_string(),
                    )));
                }
                if dto.content.is_none() {
                    return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                        "direct_publish plan requires content with video_url".to_string(),
                    )));
                }
            }
            _ => {}
        }

        // Determine initial status based on whether AI tasks are needed
        let initial_status = if dto.ai_task_types.is_some() && dto.content.is_none() {
            PlanStatus::Pending.as_str() // Will transition to ai_processing when AI tasks are created
        } else if dto.content.is_some() {
            PlanStatus::Ready.as_str() // Direct content provided, skip AI
        } else {
            PlanStatus::Pending.as_str()
        };

        // Infer ai_task_types based on plan_type if not explicitly set
        let ai_task_types = dto.ai_task_types.clone().or_else(|| {
            match PlanType::parse(plan_type.as_str()) {
                Some(PlanType::BatchText) => {
                    Some(vec![AiTaskType::ContentGen.as_str().to_string()])
                }
                Some(PlanType::SingleVideo) => {
                    if seedance_validation::is_seedance_plan(&dto.ai_input) {
                        if seedance_validation::has_content_prompt(&dto.ai_input) {
                            Some(vec![
                                AiTaskType::ContentGen.as_str().to_string(),
                                AiTaskType::SeedanceVideo.as_str().to_string(),
                            ])
                        } else {
                            Some(vec![AiTaskType::SeedanceVideo.as_str().to_string()])
                        }
                    } else {
                        Some(vec![
                            AiTaskType::ContentGen.as_str().to_string(),
                            AiTaskType::VideoGen.as_str().to_string(),
                        ])
                    }
                }
                Some(PlanType::AccountGrooming) => {
                    Some(vec![AiTaskType::AccountGrooming.as_str().to_string()])
                }
                Some(PlanType::RedditText) | Some(PlanType::RedditLink) => {
                    Some(vec![AiTaskType::ContentGen.as_str().to_string()])
                }
                Some(PlanType::RedditImage) => {
                    // If image_prompt is provided, need image_gen; otherwise just content_gen
                    if reddit_validation::has_ai_image_prompt(dto.ai_input.as_ref()) {
                        Some(vec![
                            AiTaskType::ContentGen.as_str().to_string(),
                            AiTaskType::ImageGen.as_str().to_string(),
                        ])
                    } else {
                        Some(vec![AiTaskType::ContentGen.as_str().to_string()])
                    }
                }
                Some(PlanType::DirectPublish) => None,
                None => None,
            }
        });

        let new_plan = NewAipubPlan {
            user_id,
            name: dto.name.clone(),
            group_id: dto.group_id,
            social_account_id: dto.social_account_id,
            platform_id: dto.platform_id,
            content_type: dto.content_type.clone(),
            plan_type,
            ai_task_types: ai_task_types.map(|v| v.into_iter().map(Some).collect()),
            ai_service_config: dto.ai_service_config.clone(),
            ai_input: dto.ai_input.clone(),
            content: dto.content.clone(),
            status: initial_status.to_string(),
            chat_ai_model_id: dto.chat_ai_model_id,
            video_ai_model_id: dto.video_ai_model_id,
            image_ai_model_id: dto.image_ai_model_id,
            behavior: dto.behavior.clone(),
        };

        let plan = self
            .repo
            .create_plan(new_plan)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let mut response = PlanResponseDto::from(plan.clone());

        // Enrich with names
        if let Some(gid) = plan.group_id {
            if let Ok(name) = self.repo.get_group_name(gid) {
                response.group_name = Some(name);
            }
        }
        if let Some(aid) = plan.social_account_id {
            if let Ok(name) = self.repo.get_account_username(aid) {
                response.account_username = Some(name);
            }
        }
        if let Ok(name) = self.repo.get_platform_name(plan.platform_id) {
            response.platform_name = Some(name);
        }
        if let Some(mid) = plan.chat_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.chat_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = plan.video_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.video_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = plan.image_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.image_ai_model_name = Some(name);
            }
        }

        // =====================================================================
        // Billing: Freeze budget for plans that require AI generation
        // =====================================================================
        // Only freeze if the plan needs AI processing (not direct content).
        // The stored procedure handles idempotency and balance validation.
        if dto.content.is_none() {
            let plan_type_enum = PlanType::parse(&plan.plan_type);
            let freeze_result = match plan_type_enum {
                Some(PlanType::AccountGrooming) => {
                    // 1 chat (batch name+bio) + N images (one avatar per account)
                    let account_ids = self
                        .repo
                        .get_group_account_ids(plan.group_id.unwrap_or(0))
                        .await
                        .unwrap_or_default();
                    let n = account_ids.len() as i32;
                    if n > 0 {
                        Some(
                            self.repo
                                .freeze_budget(
                                    user_id,
                                    1,
                                    n,
                                    0,
                                    plan.chat_ai_model_id,
                                    plan.image_ai_model_id,
                                    None,
                                    "aipub_plan",
                                    plan.id,
                                )
                                .await,
                        )
                    } else {
                        None // No accounts, no billing
                    }
                }
                Some(PlanType::BatchText) => {
                    // N chats (one content variation per account)
                    let account_ids = self
                        .repo
                        .get_group_account_ids(plan.group_id.unwrap_or(0))
                        .await
                        .unwrap_or_default();
                    let n = account_ids.len() as i32;
                    if n > 0 {
                        Some(
                            self.repo
                                .freeze_budget(
                                    user_id,
                                    n,
                                    0,
                                    0,
                                    plan.chat_ai_model_id,
                                    None,
                                    None,
                                    "aipub_plan",
                                    plan.id,
                                )
                                .await,
                        )
                    } else {
                        None
                    }
                }
                Some(PlanType::SingleVideo) => {
                    if seedance_validation::is_seedance_plan(&dto.ai_input) {
                        let ai_input_ref = dto.ai_input.as_ref().unwrap();
                        let validated = seedance_validation::validate_seedance_config(ai_input_ref)
                            .expect("seedance_config already validated");

                        let model_base_cost = self
                            .repo
                            .get_model_cost_multiplier(plan.video_ai_model_id.unwrap())
                            .map_err(|e| {
                                ApiError::from(DbError::SomethingWentWrong(e.to_string()))
                            })?;

                        use bigdecimal::BigDecimal;
                        use std::str::FromStr;

                        let duration_factor =
                            BigDecimal::from(validated.duration) / BigDecimal::from(4i64);
                        let quality_factor = if validated.quality == "720p" {
                            BigDecimal::from(2)
                        } else {
                            BigDecimal::from(1)
                        };
                        let audio_factor = if validated.generate_audio {
                            BigDecimal::from_str("1.5").unwrap()
                        } else {
                            BigDecimal::from(1)
                        };

                        let mut total =
                            &model_base_cost * &duration_factor * &quality_factor * &audio_factor;

                        if seedance_validation::has_content_prompt(&dto.ai_input) {
                            if let Some(chat_model_id) = plan.chat_ai_model_id {
                                if let Ok(chat_cost) =
                                    self.repo.get_model_cost_multiplier(chat_model_id)
                                {
                                    total += chat_cost;
                                }
                            }
                        }

                        Some(
                            self.repo
                                .freeze_budget_direct(user_id, total, "aipub_plan", plan.id)
                                .await,
                        )
                    } else {
                        // Existing LaoZhang/Vidu: 1 chat + 1 video
                        Some(
                            self.repo
                                .freeze_budget(
                                    user_id,
                                    1,
                                    0,
                                    1,
                                    plan.chat_ai_model_id,
                                    None,
                                    plan.video_ai_model_id,
                                    "aipub_plan",
                                    plan.id,
                                )
                                .await,
                        )
                    }
                }
                Some(PlanType::RedditText) | Some(PlanType::RedditLink) => {
                    // 1 chat call generates N variations (one per account)
                    Some(
                        self.repo
                            .freeze_budget(
                                user_id,
                                1,
                                0,
                                0,
                                plan.chat_ai_model_id,
                                None,
                                None,
                                "aipub_plan",
                                plan.id,
                            )
                            .await,
                    )
                }
                Some(PlanType::RedditImage) => {
                    // 1 chat + N images (one per account, if AI-generated).
                    //
                    // V2 path: when ai_input.image_generations[] is set, the
                    // total image count = sum(spec.count) per account so
                    // multi-image carousels pre-bill correctly. Otherwise
                    // fall back to legacy "1 image per account" rule.
                    let account_ids = self
                        .repo
                        .get_group_account_ids(plan.group_id.unwrap_or(0))
                        .await
                        .unwrap_or_default();
                    let n = account_ids.len() as i32;

                    let v2_per_account = plan
                        .ai_input
                        .as_ref()
                        .and_then(|ai| ai.get("image_generations"))
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|spec| {
                                    let c = spec.get("count").and_then(|v| v.as_u64()).unwrap_or(1)
                                        as i32;
                                    c.max(1)
                                })
                                .sum::<i32>()
                                .max(1)
                        });

                    let has_ai_image =
                        reddit_validation::has_ai_image_prompt(plan.ai_input.as_ref());
                    let image_count = match (v2_per_account, has_ai_image) {
                        (Some(per_acct), _) => per_acct * n,
                        (None, true) => n,
                        (None, false) => 0,
                    };
                    if n > 0 {
                        Some(
                            self.repo
                                .freeze_budget(
                                    user_id,
                                    1,
                                    image_count,
                                    0,
                                    plan.chat_ai_model_id,
                                    plan.image_ai_model_id,
                                    None,
                                    "aipub_plan",
                                    plan.id,
                                )
                                .await,
                        )
                    } else {
                        None
                    }
                }
                Some(PlanType::DirectPublish) => None,
                None => None,
            };

            // If freeze failed, rollback (delete plan) and return error
            if let Some(Err(e)) = freeze_result {
                tracing::warn!("Budget freeze failed for plan {}: {:?}", plan.id, e);
                self.repo.delete_plan(plan.id).await.ok();
                return Err(ApiError::InsufficientBalance);
            }

            // Reload plan to get updated billing fields
            if freeze_result.is_some() {
                if let Ok(updated_plan) = self.repo.find_plan_by_id(plan.id).await {
                    response.billing_status = updated_plan.billing_status;
                    response.frozen_cost = updated_plan.frozen_cost;
                    response.consumed_cost = updated_plan.consumed_cost;
                }
            }
        }

        // NOTE: AI tasks are now created by the Scheduler, not by the API.
        // The Scheduler will pick up pending plans and create ai_tasks.
        //
        // If direct content is provided (no AI generation needed), create publish tasks immediately.
        if dto.content.is_some() && dto.ai_task_types.is_none() {
            // Direct content - create publish tasks immediately
            self.expand_plan_to_tasks(plan.id, dto.content.as_ref().unwrap().clone())
                .await?;

            // Update plan status to ready
            let update = UpdateAipubPlan {
                status: Some(PlanStatus::Ready.as_str().to_string()),
                updated_at: Some(Utc::now()),
                ..Default::default()
            };
            self.repo.update_plan(plan.id, update).await.ok();
            response.status = PlanStatus::Ready.as_str().to_string();
        }
        // Otherwise, plan stays in "pending" status and Scheduler will pick it up

        Ok(response)
    }

    /// Get plan by ID
    pub async fn get_plan(
        &self,
        user_id: i32,
        plan_id: i32,
        include: Option<String>,
    ) -> Result<PlanDetailResponseDto, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        // Check ownership
        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        let mut response = PlanResponseDto::from(plan.clone());

        // Enrich with names
        if let Some(gid) = plan.group_id {
            if let Ok(name) = self.repo.get_group_name(gid) {
                response.group_name = Some(name);
            }
        }
        if let Some(aid) = plan.social_account_id {
            if let Ok(name) = self.repo.get_account_username(aid) {
                response.account_username = Some(name);
            }
        }
        if let Ok(name) = self.repo.get_platform_name(plan.platform_id) {
            response.platform_name = Some(name);
        }
        if let Some(mid) = plan.chat_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.chat_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = plan.video_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.video_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = plan.image_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.image_ai_model_name = Some(name);
            }
        }

        // Add stats
        let ai_stats = self
            .repo
            .count_ai_tasks_by_plan_and_status(plan_id)
            .await
            .unwrap_or_default();
        let publish_stats = self
            .repo
            .count_publish_tasks_by_plan_and_status(plan_id)
            .await
            .unwrap_or_default();

        response.ai_tasks_count = Some(ai_stats.iter().map(|(_, c)| c).sum());
        response.publish_tasks_count = Some(publish_stats.iter().map(|(_, c)| c).sum());
        response.completed_count = Some(
            publish_stats
                .iter()
                .filter(|(s, _)| s == PublishTaskStatus::Completed.as_str())
                .map(|(_, c)| c)
                .sum(),
        );
        response.failed_count = Some(
            publish_stats
                .iter()
                .filter(|(s, _)| s == PublishTaskStatus::Failed.as_str())
                .map(|(_, c)| c)
                .sum(),
        );

        // Include tasks if requested
        let ai_tasks =
            if include.as_deref() == Some("ai_tasks") || include.as_deref() == Some("all") {
                let tasks = self
                    .repo
                    .find_ai_tasks_by_plan(plan_id)
                    .await
                    .unwrap_or_default();
                Some(tasks.into_iter().map(AiTaskResponseDto::from).collect())
            } else {
                None
            };

        let publish_tasks = if include.as_deref() == Some("publish_tasks")
            || include.as_deref() == Some("all")
        {
            let tasks = self
                .repo
                .find_publish_tasks_by_plan(plan_id)
                .await
                .unwrap_or_default();
            Some(
                tasks
                    .into_iter()
                    .map(|t| {
                        let mut dto = PublishTaskResponseDto::from(t.clone());
                        // Try to enrich with account info
                        if let Ok(username) = self.repo.get_account_username(t.social_account_id) {
                            dto.account_username = Some(username);
                        }
                        dto
                    })
                    .collect(),
            )
        } else {
            None
        };

        Ok(PlanDetailResponseDto {
            plan: response,
            ai_tasks,
            publish_tasks,
        })
    }

    /// List plans for user
    pub async fn list_plans(
        &self,
        user_id: i32,
        query: PlanListQueryDto,
    ) -> Result<PageResponse<PlanResponseDto>, ApiError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).min(100);

        let (plans, total) = self
            .repo
            .find_plans_by_user(
                user_id,
                page,
                page_size,
                query.status,
                query.platform_id,
                query.content_type,
                query.plan_type,
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let total_pages = ((total as f64) / (page_size as f64)).ceil() as i64;

        let list: Vec<PlanResponseDto> = plans.into_iter().map(PlanResponseDto::from).collect();

        Ok(PageResponse {
            list,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// Update plan
    pub async fn update_plan(
        &self,
        user_id: i32,
        plan_id: i32,
        dto: UpdatePlanDto,
    ) -> Result<PlanResponseDto, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        // Check ownership
        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        // Don't allow updating plans that are completed or in progress
        if plan.status == PlanStatus::Completed.as_str()
            || plan.status == PlanStatus::AiProcessing.as_str()
        {
            return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                "Cannot update plan that is completed or in progress".to_string(),
            )));
        }

        // Phase 4 R3 Task 7a — validate behavior shape before persisting.
        behavior_validation::validate(dto.behavior.as_ref())?;

        let update = UpdateAipubPlan {
            name: dto.name,
            ai_input: dto.ai_input,
            chat_ai_model_id: dto.chat_ai_model_id.map(Some),
            video_ai_model_id: dto.video_ai_model_id.map(Some),
            image_ai_model_id: dto.image_ai_model_id.map(Some),
            updated_at: Some(Utc::now()),
            // dto.behavior == None  → leave the column unchanged.
            // dto.behavior == Some  → replace wholesale (no JSON merge).
            behavior: dto.behavior.map(Some),
            ..Default::default()
        };

        let updated_plan = self
            .repo
            .update_plan(plan_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let mut response = PlanResponseDto::from(updated_plan.clone());

        // Enrich with names
        if let Some(gid) = updated_plan.group_id {
            if let Ok(name) = self.repo.get_group_name(gid) {
                response.group_name = Some(name);
            }
        }
        if let Some(aid) = updated_plan.social_account_id {
            if let Ok(name) = self.repo.get_account_username(aid) {
                response.account_username = Some(name);
            }
        }
        if let Ok(name) = self.repo.get_platform_name(updated_plan.platform_id) {
            response.platform_name = Some(name);
        }
        if let Some(mid) = updated_plan.chat_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.chat_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = updated_plan.video_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.video_ai_model_name = Some(name);
            }
        }
        if let Some(mid) = updated_plan.image_ai_model_id {
            if let Ok(name) = self.repo.get_model_name(mid) {
                response.image_ai_model_name = Some(name);
            }
        }

        Ok(response)
    }

    /// Delete plan
    pub async fn delete_plan(&self, user_id: i32, plan_id: i32) -> Result<(), ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        // Don't allow deleting plans that are in progress
        if plan.status == PlanStatus::AiProcessing.as_str() {
            return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                "Cannot delete plan that is in progress".to_string(),
            )));
        }

        // If billing is frozen, finalize first to refund remaining
        if plan.billing_status == "frozen" {
            self.repo
                .finalize_plan(plan_id, PlanStatus::Failed.as_str())
                .await
                .ok();
        }

        self.repo
            .delete_plan(plan_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(())
    }

    /// Retry failed plan
    pub async fn retry_plan(
        &self,
        user_id: i32,
        plan_id: i32,
        dto: RetryPlanDto,
    ) -> Result<RetryPlanResponseDto, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        let mut retried_ai_tasks = 0i64;
        let mut retried_publish_tasks = 0i64;

        // Retry failed AI tasks
        if dto.retry_scope == "all" || dto.retry_scope == "failed_ai" {
            let ai_tasks = self
                .repo
                .find_ai_tasks_by_plan(plan_id)
                .await
                .unwrap_or_default();
            for task in ai_tasks {
                if task.status == AiTaskStatus::Failed.as_str() {
                    let update = UpdateAipubAiTask {
                        status: Some(AiTaskStatus::Pending.as_str().to_string()),
                        error_message: Some(String::new()),
                        retry_count: Some(task.retry_count.unwrap_or(0) + 1),
                        updated_at: Some(Utc::now()),
                        ..Default::default()
                    };
                    self.repo.update_ai_task(task.id, update).await.ok();
                    retried_ai_tasks += 1;
                }
            }
        }

        // Retry failed publish tasks
        if dto.retry_scope == "all" || dto.retry_scope == "failed_publish" {
            let publish_tasks = self
                .repo
                .find_publish_tasks_by_plan(plan_id)
                .await
                .unwrap_or_default();
            for task in publish_tasks {
                if task.status == PublishTaskStatus::Failed.as_str() {
                    let update = UpdateAipubTask {
                        status: Some(PublishTaskStatus::Ready.as_str().to_string()),
                        error_message: Some(String::new()),
                        retry_count: Some(task.retry_count.unwrap_or(0) + 1),
                        updated_at: Some(Utc::now()),
                        ..Default::default()
                    };
                    self.repo.update_publish_task(task.id, update).await.ok();
                    retried_publish_tasks += 1;
                }
            }
        }

        // Update plan status
        let new_status = if retried_ai_tasks > 0 {
            PlanStatus::AiProcessing.as_str()
        } else if retried_publish_tasks > 0 {
            PlanStatus::Ready.as_str()
        } else {
            &plan.status
        };

        let update = UpdateAipubPlan {
            status: Some(new_status.to_string()),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };
        self.repo.update_plan(plan_id, update).await.ok();

        Ok(RetryPlanResponseDto {
            id: plan_id,
            status: new_status.to_string(),
            retried_ai_tasks,
            retried_publish_tasks,
        })
    }

    /// Get plan stats for user
    pub async fn get_plan_stats(&self, user_id: i32) -> Result<PlanStatsDto, ApiError> {
        let stats = self
            .repo
            .count_plans_by_status(user_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let mut result = PlanStatsDto {
            total_plans: 0,
            ai_processing: 0,
            ready: 0,
            completed: 0,
            failed: 0,
        };

        for (status, count) in stats {
            result.total_plans += count;
            if status == PlanStatus::AiProcessing.as_str() {
                result.ai_processing = count;
            } else if status == PlanStatus::Ready.as_str() {
                result.ready = count;
            } else if status == PlanStatus::Completed.as_str() {
                result.completed = count;
            } else if status == PlanStatus::Failed.as_str() {
                result.failed = count;
            }
        }

        Ok(result)
    }

    /// Estimate plan cost without creating a plan (pre-submit preview)
    pub async fn estimate_plan_cost(
        &self,
        dto: EstimatePlanCostDto,
    ) -> Result<PlanCostEstimateDto, ApiError> {
        // Seedance estimation path
        if let Some(ref seedance) = dto.seedance_config {
            let video_model_id = dto.video_model_id.ok_or_else(|| {
                ApiError::BusinessError(BusinessError::InvalidInput(
                    "video_model_id is required for Seedance estimation".to_string(),
                ))
            })?;

            let model_base_cost = self
                .repo
                .get_model_cost_multiplier(video_model_id)
                .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

            use bigdecimal::BigDecimal;
            use std::str::FromStr;

            let duration_f = BigDecimal::from(seedance.duration) / BigDecimal::from(4i32);
            let quality_f = if seedance.quality == "720p" {
                BigDecimal::from(2)
            } else {
                BigDecimal::from(1)
            };
            let audio_f = if seedance.generate_audio {
                BigDecimal::from_str("1.5").unwrap()
            } else {
                BigDecimal::from(1)
            };

            let video_cost = &model_base_cost * &duration_f * &quality_f * &audio_f;
            let chat_cost = if let Some(chat_id) = dto.chat_model_id {
                self.repo
                    .get_model_cost_multiplier(chat_id)
                    .unwrap_or_default()
            } else {
                BigDecimal::from(0)
            };
            let total = &video_cost + &chat_cost;

            return Ok(PlanCostEstimateDto {
                video_unit_cost: model_base_cost.clone(),
                total_cost: total.clone(),
                seedance_cost: Some(SeedanceCostBreakdown {
                    base_price: model_base_cost,
                    duration_factor: duration_f,
                    quality_factor: quality_f,
                    audio_factor: audio_f,
                    video_cost,
                    chat_cost,
                    total,
                }),
                ..Default::default()
            });
        }

        let account_count = dto.account_count.unwrap_or(1).max(1);

        let row = self
            .repo
            .estimate_plan_cost(
                dto.chat_model_id,
                dto.video_model_id,
                dto.image_model_id,
                account_count,
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(PlanCostEstimateDto {
            chat_unit_cost: row.chat_unit_cost,
            video_unit_cost: row.video_unit_cost,
            image_unit_cost: row.image_unit_cost,
            per_account_cost: row.per_account_cost,
            account_count: row.account_count,
            total_cost: row.total_cost,
            pricing_snapshot: row.pricing_snapshot,
            seedance_cost: None,
        })
    }

    // =========================================================================
    // AI Task Service Methods
    // =========================================================================

    /// Get AI tasks for a plan
    pub async fn get_ai_tasks_by_plan(
        &self,
        user_id: i32,
        plan_id: i32,
    ) -> Result<Vec<AiTaskResponseDto>, ApiError> {
        // Verify ownership
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        let tasks = self
            .repo
            .find_ai_tasks_by_plan(plan_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(tasks.into_iter().map(AiTaskResponseDto::from).collect())
    }

    /// Get processing AI tasks (for scheduler)
    pub async fn get_processing_ai_tasks(
        &self,
        limit: i64,
    ) -> Result<Vec<AiTaskResponseDto>, ApiError> {
        let tasks = self
            .repo
            .find_processing_ai_tasks(limit)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(tasks.into_iter().map(AiTaskResponseDto::from).collect())
    }

    /// Update AI task progress
    pub async fn update_ai_progress(
        &self,
        task_id: i32,
        progress: i32,
    ) -> Result<AiTaskResponseDto, ApiError> {
        let update = UpdateAipubAiTask {
            progress: Some(progress),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        let task = self
            .repo
            .update_ai_task(task_id, update)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("AI Task".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        Ok(AiTaskResponseDto::from(task))
    }

    /// Complete AI task and expand to publish tasks
    pub async fn complete_ai_task(
        &self,
        task_id: i32,
        result: serde_json::Value,
    ) -> Result<CompleteAiTaskResponseDto, ApiError> {
        let ai_task = self
            .repo
            .find_ai_task_by_id(task_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("AI Task".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if ai_task.status != AiTaskStatus::Processing.as_str() {
            return Err(ApiError::BusinessError(BusinessError::InvalidInput(
                "AI task is not in processing status".to_string(),
            )));
        }

        // Update AI task to completed
        let now = Utc::now();
        let update = UpdateAipubAiTask {
            status: Some(AiTaskStatus::Completed.as_str().to_string()),
            result: Some(result.clone()),
            progress: Some(100),
            updated_at: Some(now),
            completed_at: Some(now),
            ..Default::default()
        };

        self.repo
            .update_ai_task(task_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        // Check if all AI tasks for this plan are completed
        let ai_tasks = self
            .repo
            .find_ai_tasks_by_plan(ai_task.plan_id)
            .await
            .unwrap_or_default();

        let all_completed = ai_tasks
            .iter()
            .all(|t| t.id == task_id || t.status == AiTaskStatus::Completed.as_str());

        let expanded_count = if all_completed {
            // Build final content from all AI results
            let mut final_content = json!({});
            for task in ai_tasks {
                if let Some(r) = task.result {
                    if let Some(obj) = final_content.as_object_mut() {
                        if let Some(r_obj) = r.as_object() {
                            for (k, v) in r_obj {
                                obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }
            // Merge with the current result
            if let Some(obj) = final_content.as_object_mut() {
                if let Some(r_obj) = result.as_object() {
                    for (k, v) in r_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }

            // Expand to publish tasks
            let count = self
                .expand_plan_to_tasks(ai_task.plan_id, final_content)
                .await?;

            // Update plan status to ready
            let plan_update = UpdateAipubPlan {
                status: Some(PlanStatus::Ready.as_str().to_string()),
                updated_at: Some(now),
                ..Default::default()
            };
            self.repo
                .update_plan(ai_task.plan_id, plan_update)
                .await
                .ok();

            count
        } else {
            0
        };

        Ok(CompleteAiTaskResponseDto {
            id: task_id,
            status: AiTaskStatus::Completed.as_str().to_string(),
            completed_at: Some(now),
            expanded_tasks_count: expanded_count,
        })
    }

    /// Mark AI task as failed
    pub async fn fail_ai_task(
        &self,
        task_id: i32,
        error_message: String,
    ) -> Result<AiTaskResponseDto, ApiError> {
        let ai_task = self
            .repo
            .find_ai_task_by_id(task_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("AI Task".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        let update = UpdateAipubAiTask {
            status: Some(AiTaskStatus::Failed.as_str().to_string()),
            error_message: Some(error_message),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };

        let task = self
            .repo
            .update_ai_task(task_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        // Use fn_finalize_plan for atomic status update + billing refund
        self.repo
            .finalize_plan(ai_task.plan_id, PlanStatus::Failed.as_str())
            .await
            .ok();

        Ok(AiTaskResponseDto::from(task))
    }

    // =========================================================================
    // Publish Task Service Methods
    // =========================================================================

    /// Get publish tasks for a plan
    pub async fn get_publish_tasks_by_plan(
        &self,
        user_id: i32,
        plan_id: i32,
    ) -> Result<Vec<PublishTaskResponseDto>, ApiError> {
        // Verify ownership
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => {
                    ApiError::BusinessError(BusinessError::ResourceNotFound("Plan".to_string()))
                }
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        if plan.user_id != user_id {
            return Err(ApiError::BusinessError(BusinessError::ResourceNotFound(
                "Plan".to_string(),
            )));
        }

        let tasks = self
            .repo
            .find_publish_tasks_by_plan(plan_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(tasks
            .into_iter()
            .map(PublishTaskResponseDto::from)
            .collect())
    }

    /// List all publish tasks for a user (paginated)
    pub async fn list_user_publish_tasks(
        &self,
        user_id: i32,
        query: UserPublishTaskQueryDto,
    ) -> Result<PageResponse<PublishTaskResponseDto>, ApiError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).min(100);

        let (tasks, total) = self
            .repo
            .find_publish_tasks_by_user(user_id, query.status, query.platform_id, page, page_size)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let tasks_dto: Vec<PublishTaskResponseDto> = tasks
            .into_iter()
            .map(|(task, plan, platform, profile_name, platform_id)| {
                let mut dto = PublishTaskResponseDto::from(task);
                dto.platform = Some(platform);
                dto.platform_id = Some(platform_id);
                dto.profile_name = profile_name;
                dto.content_type = Some(plan.content_type);
                dto
            })
            .collect();

        Ok(PageResponse {
            list: tasks_dto,
            total,
            page,
            page_size,
            total_pages: (total as f64 / page_size as f64).ceil() as i64,
        })
    }

    /// Get ready publish tasks for executor
    pub async fn get_ready_publish_tasks(
        &self,
        query: PublishTaskQueryDto,
    ) -> Result<Vec<ExecutorPublishTaskDto>, ApiError> {
        let tasks = self
            .repo
            .find_ready_publish_tasks_by_device(
                &query.device_id,
                query.platform.as_deref(),
                query.limit,
            )
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(tasks
            .into_iter()
            .map(
                |(task, plan, platform, profile_name, platform_id)| ExecutorPublishTaskDto {
                    task_id: task.id,
                    plan_id: task.plan_id,
                    social_account_id: task.social_account_id,
                    platform,
                    platform_id,
                    content_type: plan.content_type.clone(),
                    plan_type: plan.plan_type.clone(),
                    profile_name: profile_name.unwrap_or_default(),
                    content: task.content,
                    created_at: task.created_at,
                },
            )
            .collect())
    }

    /// Update publish task status
    pub async fn update_publish_task_status(
        &self,
        task_id: i32,
        dto: UpdatePublishTaskStatusDto,
    ) -> Result<PublishTaskResponseDto, ApiError> {
        let task = self
            .repo
            .find_publish_task_by_id(task_id)
            .await
            .map_err(|e| match e {
                DieselError::NotFound => ApiError::BusinessError(BusinessError::ResourceNotFound(
                    "Publish Task".to_string(),
                )),
                _ => ApiError::from(DbError::SomethingWentWrong(e.to_string())),
            })?;

        let now = Utc::now();
        let update = UpdateAipubTask {
            status: Some(dto.status.clone()),
            result_url: dto.result_url,
            error_message: dto.error_message,
            updated_at: Some(now),
            published_at: if dto.status == PublishTaskStatus::Completed.as_str() {
                Some(now)
            } else {
                None
            },
            ..Default::default()
        };

        let updated_task = self
            .repo
            .update_publish_task(task_id, update)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        // Check if all publish tasks for this plan are completed
        if dto.status == PublishTaskStatus::Completed.as_str()
            || dto.status == PublishTaskStatus::Failed.as_str()
        {
            self.check_and_update_plan_completion(task.plan_id).await?;
        }

        Ok(PublishTaskResponseDto::from(updated_task))
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Expand plan to publish tasks (for each account in group or single account).
    ///
    /// Phase 4 R3 Task 7a — when `plan.behavior` is set, merges it into
    /// each task's `content.behavior` so workers consume it natively via
    /// `UnifiedPublishContent.from_dict` without an extra API hop. When
    /// `plan.behavior` is NULL the merge is a no-op and the task content
    /// stays exactly as the caller supplied it (no spurious empty
    /// behavior key — the worker would treat that as "override platform
    /// defaults to UNSPECIFIED visibility", which is wrong).
    async fn expand_plan_to_tasks(
        &self,
        plan_id: i32,
        content: serde_json::Value,
    ) -> Result<i64, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(plan_id)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        let account_ids: Vec<i32> = if let Some(gid) = plan.group_id {
            self.repo
                .get_group_account_ids(gid)
                .await
                .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?
        } else if let Some(aid) = plan.social_account_id {
            vec![aid]
        } else {
            return Ok(0);
        };

        let merged_content = merge_behavior_into_content(content, plan.behavior.as_ref());

        let new_tasks: Vec<NewAipubTask> = account_ids
            .iter()
            .map(|&account_id| NewAipubTask {
                plan_id,
                social_account_id: account_id,
                content: merged_content.clone(),
                status: PublishTaskStatus::Ready.as_str().to_string(),
                ai_task_id: None, // Direct content - no AI task
            })
            .collect();

        let count = new_tasks.len() as i64;

        self.repo
            .create_publish_tasks_batch(new_tasks)
            .await
            .map_err(|e| ApiError::from(DbError::SomethingWentWrong(e.to_string())))?;

        Ok(count)
    }

    /// Check and update plan completion status.
    /// Uses fn_finalize_plan stored procedure which atomically:
    /// - Updates plan status to completed/failed
    /// - Refunds remaining frozen budget
    /// - Sets billing_status to 'settled'
    async fn check_and_update_plan_completion(&self, plan_id: i32) -> Result<(), ApiError> {
        let stats = self
            .repo
            .count_publish_tasks_by_plan_and_status(plan_id)
            .await
            .unwrap_or_default();

        let total: i64 = stats.iter().map(|(_, c)| c).sum();
        let completed: i64 = stats
            .iter()
            .filter(|(s, _)| s == PublishTaskStatus::Completed.as_str())
            .map(|(_, c)| *c)
            .sum();
        let failed: i64 = stats
            .iter()
            .filter(|(s, _)| s == PublishTaskStatus::Failed.as_str())
            .map(|(_, c)| *c)
            .sum();

        if total > 0 && completed + failed == total {
            let new_status = if failed > 0 && completed == 0 {
                PlanStatus::Failed.as_str()
            } else {
                PlanStatus::Completed.as_str()
            };

            // Use fn_finalize_plan for atomic status update + billing settlement
            self.repo.finalize_plan(plan_id, new_status).await.ok();
        }

        Ok(())
    }
}

/// Phase 4 R3 Task 7a — merge a plan-level PublishBehavior JSON blob
/// into a task's content under the `behavior` key.
///
/// * When `behavior` is `None`, returns `content` untouched. This is
///   the critical "no spurious empty behavior" invariant — emitting
///   `content.behavior = {}` would tell the worker to override
///   platform defaults with UNSPECIFIED visibility.
/// * When `content` is not a JSON object, behavior cannot be merged —
///   returns `content` untouched and lets the worker reject the
///   malformed payload (by design we don't try to be clever and
///   wrap a non-object in `{behavior: …, value: content}`).
fn merge_behavior_into_content(
    mut content: serde_json::Value,
    behavior: Option<&serde_json::Value>,
) -> serde_json::Value {
    let Some(behavior) = behavior else {
        return content;
    };
    if let Some(obj) = content.as_object_mut() {
        obj.insert("behavior".to_string(), behavior.clone());
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_behavior_into_content_skips_when_behavior_is_none() {
        let content = json!({"title": "T"});
        let merged = merge_behavior_into_content(content.clone(), None);
        assert_eq!(merged, content);
        assert!(merged.get("behavior").is_none());
    }

    #[test]
    fn merge_behavior_into_content_inserts_behavior_key() {
        let content = json!({"title": "T", "description": "D"});
        let behavior = json!({"visibility": "public", "is_nsfw": true});
        let merged = merge_behavior_into_content(content, Some(&behavior));
        assert_eq!(merged["title"], "T");
        assert_eq!(merged["description"], "D");
        assert_eq!(merged["behavior"]["visibility"], "public");
        assert_eq!(merged["behavior"]["is_nsfw"], true);
    }

    #[test]
    fn merge_behavior_into_content_overwrites_existing_behavior_key() {
        // Defensive: if upstream content already carries a behavior key
        // (e.g. AI-generated content), the plan-level behavior wins.
        let content = json!({"title": "T", "behavior": {"visibility": "private"}});
        let behavior = json!({"visibility": "public"});
        let merged = merge_behavior_into_content(content, Some(&behavior));
        assert_eq!(merged["behavior"]["visibility"], "public");
    }

    #[test]
    fn merge_behavior_into_content_leaves_non_object_content_alone() {
        let content = json!("just a string");
        let behavior = json!({"visibility": "public"});
        let merged = merge_behavior_into_content(content.clone(), Some(&behavior));
        assert_eq!(merged, content);
    }
}
