use bigdecimal::BigDecimal;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

use crate::dto::laozhang_dto::{CreateVideoFromImageRequest, CreateVideoFromTextRequest};
use crate::dto::video_dto::{
    CreateVideoRequest, CreateVideoResponse, VideoTaskListResponse, VideoTaskResponse,
};
use glance_mind_db::entity::ai_model::AiModel;
use glance_mind_db::entity::video::NewVideoGenerationTask;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::video_repository::VideoRepository;
use crate::repository::wallet_repository::WalletRepository;
use crate::service::config_service::ConfigService;
use crate::service::laozhang_client::LaoZhangClient;

#[derive(Clone)]
pub struct VideoService {
    db_pool: Pool<ConnectionManager<PgConnection>>,
    #[allow(dead_code)]
    wallet_repo: WalletRepository,
    laozhang_client: LaoZhangClient,
    #[allow(dead_code)]
    config_service: ConfigService,
}

impl VideoService {
    pub fn new(
        db_pool: Pool<ConnectionManager<PgConnection>>,
        wallet_repo: WalletRepository,
        laozhang_client: LaoZhangClient,
        config_service: ConfigService,
    ) -> Self {
        Self {
            db_pool,
            wallet_repo,
            laozhang_client,
            config_service,
        }
    }

    /// Create video generation task (supports text-to-video, single-image-to-video, dual-image-to-video)
    #[allow(clippy::too_many_arguments)]
    pub async fn create_video(
        &self,
        user_id: i32,
        request: CreateVideoRequest,
        // Single image mode (backward compatible)
        image_data: Option<Vec<u8>>,
        image_filename: Option<String>,
        _image_content_type: Option<String>,
        // Dual image mode (new)
        start_frame_data: Option<Vec<u8>>,
        start_frame_filename: Option<String>,
        end_frame_data: Option<Vec<u8>>,
        end_frame_filename: Option<String>,
    ) -> Result<CreateVideoResponse, ApiError> {
        // Detect dual image mode
        let is_dual_image = start_frame_data.is_some() && end_frame_data.is_some();
        let has_any_image = image_data.is_some() || start_frame_data.is_some();

        // Validate parameters: must have prompt or image
        if request.prompt.is_none() && !has_any_image {
            return Err(ApiError::BusinessError(
                BusinessError::PromptOrImageRequired,
            ));
        }

        // Validate dual image mode completeness
        if start_frame_data.is_some() != end_frame_data.is_some() {
            return Err(ApiError::BusinessError(
                BusinessError::DualImageRequiresStartAndEnd,
            ));
        }

        // Validate and get model info
        let ai_model_info = if let Some(model_id) = request.ai_model_id {
            // Validate model ID exists
            use glance_mind_db::schema::gm_ai_models::dsl::*;
            use diesel::prelude::*;

            let mut conn = self.db_pool.get().map_err(|_| {
                ApiError::InternalServerError("Database connection failed".to_string())
            })?;

            let model = gm_ai_models
                .filter(id.eq(model_id))
                .filter(model_type.eq("video"))
                .filter(is_active.eq(true))
                .first::<AiModel>(&mut conn)
                .optional()
                .map_err(|_| ApiError::InternalServerError("Failed to query model".to_string()))?;

            if model.is_none() {
                return Err(ApiError::BusinessError(BusinessError::ModelNotFound(
                    model_id,
                )));
            }
            model
        } else {
            None
        };

        // Determine model name to use
        let (model_key, model_name) = if let Some(ref model) = ai_model_info {
            (model.model_key.clone(), Some(model.name.clone()))
        } else {
            // Default to sora-2
            ("sora-2".to_string(), Some("Sora 2".to_string()))
        };

        // Validate model supports dual image
        if is_dual_image {
            let model_supports_dual = model_key.contains("-fl");
            if !model_supports_dual {
                return Err(ApiError::BusinessError(
                    BusinessError::ModelNotSupportDualImage,
                ));
            }
        }

        // Determine if landscape mode
        let is_landscape = model_key.contains("landscape");

        tracing::info!(
            "Video generation request: user_id={}, model_key={}, mode={}, prompt_len={}",
            user_id,
            model_key,
            if is_dual_image {
                "dual-image"
            } else if has_any_image {
                "single-image"
            } else {
                "text-to-video"
            },
            request.prompt.as_ref().map(|p| p.len()).unwrap_or(0)
        );

        // Note: Fee calculation and deduction already handled by charging_middleware

        // Call LaoZhang API - Select different API call based on mode
        let task_response = if is_dual_image {
            // Dual image mode
            tracing::info!(
                "Calling dual-image-to-video API: model={}, landscape={}, start={}, end={}",
                model_key,
                is_landscape,
                start_frame_filename
                    .as_ref()
                    .unwrap_or(&"unknown".to_string()),
                end_frame_filename
                    .as_ref()
                    .unwrap_or(&"unknown".to_string())
            );

            self.laozhang_client
                .create_video_from_two_images_veo(
                    request.prompt.clone().unwrap_or_default(),
                    start_frame_data.unwrap(),
                    start_frame_filename.unwrap_or_else(|| "start.png".to_string()),
                    end_frame_data.unwrap(),
                    end_frame_filename.unwrap_or_else(|| "end.png".to_string()),
                    is_landscape,
                )
                .await?
        } else if let Some(img_data) = image_data.or(start_frame_data) {
            // Single image mode (compatible with image or start_frame)
            let filename = image_filename
                .or(start_frame_filename)
                .unwrap_or_else(|| "image.png".to_string());

            let req = CreateVideoFromImageRequest {
                model: model_key.clone(),
                prompt: request.prompt.clone().unwrap_or_default(),
                image_data: img_data,
                image_filename: filename.clone(),
                size: request.size.clone(),
                seconds: request.seconds.clone(),
            };

            tracing::info!(
                "Calling image-to-video API: model={}, image_filename={}, size={}, seconds={}",
                req.model,
                filename,
                req.size,
                req.seconds
            );

            self.laozhang_client.create_video_from_image(req).await?
        } else {
            // Text-to-video
            let req = CreateVideoFromTextRequest {
                model: model_key.clone(),
                prompt: request.prompt.clone().ok_or(ApiError::BusinessError(
                    BusinessError::TextToVideoRequiresPrompt,
                ))?,
                size: request.size.clone(),
                seconds: request.seconds.clone(),
            };

            tracing::info!(
                "Calling text-to-video API: model={}, prompt_len={}, size={}, seconds={}",
                req.model,
                req.prompt.len(),
                req.size,
                req.seconds
            );

            self.laozhang_client.create_video_from_text(req).await?
        };

        // Save task to database
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        // Calculate actual cost: pricing_rules.VIDEO_GENERATE * ai_models.cost_multiplier
        // This needs to be consistent with charging_middleware
        use crate::middleware::charging::ActionType;
        use glance_mind_db::schema::gm_pricing_rules::dsl::*;
        use diesel::prelude::*;

        // Query VIDEO_GENERATE base price
        let base_cost = gm_pricing_rules
            .filter(action_type.eq(ActionType::VideoGenerate.as_str()))
            .filter(platform_id.is_null())
            .select(cost_points)
            .first::<BigDecimal>(&mut conn)
            .map_err(|_| {
                ApiError::InternalServerError("Failed to query pricing rule".to_string())
            })?;

        // Calculate actual cost
        let actual_cost = if let Some(ref model) = ai_model_info {
            &base_cost * &model.cost_multiplier
        } else {
            base_cost
        };

        let new_task = NewVideoGenerationTask {
            user_id,
            task_id: task_response.id.clone(),
            prompt: request.prompt,
            media_id: None,
            status: "pending".to_string(),
            cost_points: actual_cost.clone(), // Consistent with charging_middleware calculation
            wallet_transaction_id: None,      // transaction_id created by charging_middleware
            model_id: request.ai_model_id,
            title: request.title,
            orientation: Some(request.orientation.as_str().to_string()),
            video_seconds: Some(request.seconds),
            video_size: Some(request.size),
        };

        VideoRepository::create_task(&mut conn, new_task).map_err(|_| {
            ApiError::InternalServerError("Failed to create task record".to_string())
        })?;

        Ok(CreateVideoResponse {
            task_id: task_response.id,
            status: task_response.status,
            cost_points: actual_cost,
            estimated_time: "2-5 minutes".to_string(),
            ai_model_name: model_name,
            expires_at: task_response.expires,
        })
    }

    /// Get user's video task list
    pub async fn get_user_tasks(
        &self,
        user_id: i32,
        page: i32,
        page_size: i32,
    ) -> Result<VideoTaskListResponse, ApiError> {
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        let (tasks, total) =
            VideoRepository::get_by_user_id(&mut conn, user_id, None, page, page_size)
                .map_err(|_| ApiError::InternalServerError("Failed to query tasks".to_string()))?;

        let task_responses: Vec<VideoTaskResponse> = tasks
            .into_iter()
            .map(|task| VideoTaskResponse {
                id: task.id,
                task_id: task.task_id,
                title: task.title,
                prompt: task.prompt,
                status: task.status,
                progress_pct: task.progress_pct,
                video_url: task.video_url,
                thumbnail_url: task.thumbnail_url,
                cost_points: task.cost_points,
                error_message: task.error_message,
                created_at: task.created_at,
                completed_at: task.completed_at,
                ai_model_name: None, // TODO: Join query for model name
                orientation: task.orientation,
                video_seconds: task.video_seconds,
                video_size: task.video_size,
            })
            .collect();

        Ok(VideoTaskListResponse {
            tasks: task_responses,
            total,
            page,
            page_size,
        })
    }

    /// Get single video task details
    pub async fn get_user_task(
        &self,
        user_id: i32,
        task_id: &str,
    ) -> Result<VideoTaskResponse, ApiError> {
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        let task = VideoRepository::get_by_task_id(&mut conn, task_id).map_err(|_| {
            ApiError::BusinessError(BusinessError::VideoTaskNotFound(task_id.to_string()))
        })?;

        // Validate task belongs to user
        if task.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::VideoTaskPermissionDenied,
            ));
        }

        Ok(VideoTaskResponse {
            id: task.id,
            task_id: task.task_id,
            title: task.title,
            prompt: task.prompt,
            status: task.status,
            progress_pct: task.progress_pct,
            video_url: task.video_url,
            thumbnail_url: task.thumbnail_url,
            cost_points: task.cost_points,
            error_message: task.error_message,
            created_at: task.created_at,
            completed_at: task.completed_at,
            ai_model_name: None, // TODO: Join query for model name
            orientation: task.orientation,
            video_seconds: task.video_seconds,
            video_size: task.video_size,
        })
    }
}
