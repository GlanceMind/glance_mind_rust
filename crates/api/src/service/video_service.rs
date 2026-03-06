use bigdecimal::BigDecimal;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;

use crate::dto::jimeng_dto::{JimengResolution, JimengTaskHandle, JimengVideoParams};
use crate::dto::laozhang_dto::{CreateVideoFromImageRequest, CreateVideoFromTextRequest};
use crate::dto::video_dto::{
    CreateVideoRequest, CreateVideoResponse, VideoTaskListResponse, VideoTaskResponse,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::repository::video_repository::VideoRepository;
use crate::repository::wallet_repository::WalletRepository;
use crate::service::config_service::ConfigService;
use crate::service::jimeng_client::{is_jimeng_model, JimengClient};
use crate::service::laozhang_client::LaoZhangClient;
use crate::service::oss_service::{OssConfig, OssService};
use glance_mind_db::entity::ai_model::AiModel;
use glance_mind_db::entity::video::{NewVideoGenerationTask, VideoGenerationTask};

#[derive(Clone)]
pub struct VideoService {
    db_pool: Pool<ConnectionManager<PgConnection>>,
    #[allow(dead_code)]
    wallet_repo: WalletRepository,
    laozhang_client: LaoZhangClient,
    jimeng_client: Option<JimengClient>,
    #[allow(dead_code)]
    config_service: ConfigService,
    oss_config: Option<OssConfig>,
}

impl VideoService {
    pub fn new(
        db_pool: Pool<ConnectionManager<PgConnection>>,
        wallet_repo: WalletRepository,
        laozhang_client: LaoZhangClient,
        jimeng_client: Option<JimengClient>,
        config_service: ConfigService,
    ) -> Self {
        let oss_config = OssConfig::from_env().ok();
        if oss_config.is_some() {
            tracing::info!("VideoService: OSS configured for video persistence");
        } else {
            tracing::warn!("VideoService: OSS not configured, Jimeng videos will use temporary CDN URLs");
        }
        Self {
            db_pool,
            wallet_repo,
            laozhang_client,
            jimeng_client,
            config_service,
            oss_config,
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
            use diesel::prelude::*;
            use glance_mind_db::schema::gm_ai_models::dsl::*;

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

        // Route to Jimeng if model is jimeng-*
        if is_jimeng_model(&model_key) {
            return self
                .create_video_jimeng(
                    user_id,
                    &model_key,
                    &request,
                    image_data.or(start_frame_data),
                    end_frame_data,
                    &ai_model_info,
                )
                .await;
        }

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
        use diesel::prelude::*;
        use glance_mind_db::schema::gm_pricing_rules::dsl::*;

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
            generation_id: None,
            prompt: request.prompt,
            media_id: None,
            status: "pending".to_string(),
            cost_points: actual_cost.clone(),
            wallet_transaction_id: None,
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

    /// Get user's video task list.
    /// For pending Jimeng tasks, lazily polls Volcengine and updates the DB.
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

        let mut task_responses = Vec::with_capacity(tasks.len());
        for task in tasks {
            let final_task = if Self::is_jimeng_pending(&task) {
                self.poll_jimeng_task(&mut conn, &task).await.unwrap_or(task)
            } else {
                task
            };
            task_responses.push(Self::task_to_response(final_task));
        }

        Ok(VideoTaskListResponse {
            tasks: task_responses,
            total,
            page,
            page_size,
        })
    }

    /// Get single video task details.
    /// For pending Jimeng tasks, lazily polls Volcengine and updates the DB.
    pub async fn get_user_task(
        &self,
        user_id: i32,
        task_id: &str,
    ) -> Result<VideoTaskResponse, ApiError> {
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        let mut task = VideoRepository::get_by_task_id(&mut conn, task_id).map_err(|_| {
            ApiError::BusinessError(BusinessError::VideoTaskNotFound(task_id.to_string()))
        })?;

        if task.user_id != user_id {
            return Err(ApiError::BusinessError(
                BusinessError::VideoTaskPermissionDenied,
            ));
        }

        if Self::is_jimeng_pending(&task) {
            if let Some(updated) = self.poll_jimeng_task(&mut conn, &task).await {
                task = updated;
            }
        }

        Ok(Self::task_to_response(task))
    }

    /// Jimeng video generation path
    #[allow(clippy::too_many_arguments)]
    async fn create_video_jimeng(
        &self,
        user_id: i32,
        model_key: &str,
        request: &CreateVideoRequest,
        image_data: Option<Vec<u8>>,
        _end_frame_data: Option<Vec<u8>>,
        ai_model_info: &Option<AiModel>,
    ) -> Result<CreateVideoResponse, ApiError> {
        let jimeng = self.jimeng_client.as_ref().ok_or_else(|| {
            ApiError::InternalServerError("Jimeng client not configured".to_string())
        })?;

        let resolution = Self::detect_jimeng_resolution(model_key);
        let seconds: i32 = request.seconds.parse().unwrap_or(5);

        let params = JimengVideoParams {
            prompt: request.prompt.clone().unwrap_or_default(),
            resolution,
            seconds,
            aspect_ratio: if image_data.is_none() { Some(self.detect_aspect_ratio(request)) } else { None },
            image_base64: image_data.map(|d| JimengClient::encode_image(&d)),
        };

        tracing::info!(
            "Jimeng video: model={}, resolution={}, seconds={}, has_image={}",
            model_key, resolution.label(), seconds, params.image_base64.is_some()
        );

        let handle = jimeng.create_video(params).await?;
        let task_id = handle.task_id;
        let req_key = handle.req_key;

        // Save task to database
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| ApiError::InternalServerError("Database connection failed".to_string()))?;

        use crate::middleware::charging::ActionType;
        use diesel::prelude::*;
        use glance_mind_db::schema::gm_pricing_rules::dsl::*;

        let base_cost = gm_pricing_rules
            .filter(action_type.eq(ActionType::VideoGenerate.as_str()))
            .filter(platform_id.is_null())
            .select(cost_points)
            .first::<BigDecimal>(&mut conn)
            .map_err(|_| {
                ApiError::InternalServerError("Failed to query pricing rule".to_string())
            })?;

        let actual_cost = if let Some(ref model) = ai_model_info {
            &base_cost * &model.cost_multiplier
        } else {
            base_cost
        };

        let model_name = ai_model_info.as_ref().map(|m| m.name.clone());

        let new_task = NewVideoGenerationTask {
            user_id,
            task_id: task_id.clone(),
            generation_id: Some(req_key),
            prompt: request.prompt.clone(),
            media_id: None,
            status: "pending".to_string(),
            cost_points: actual_cost.clone(),
            wallet_transaction_id: None,
            model_id: request.ai_model_id,
            title: request.title.clone(),
            orientation: Some(request.orientation.as_str().to_string()),
            video_seconds: Some(request.seconds.clone()),
            video_size: Some(request.size.clone()),
        };

        VideoRepository::create_task(&mut conn, new_task).map_err(|_| {
            ApiError::InternalServerError("Failed to create task record".to_string())
        })?;

        Ok(CreateVideoResponse {
            task_id,
            status: "pending".to_string(),
            cost_points: actual_cost,
            estimated_time: "2-5 minutes".to_string(),
            ai_model_name: model_name,
            expires_at: None,
        })
    }

    /// Map model_key to Jimeng resolution tier.
    fn detect_jimeng_resolution(model_key: &str) -> JimengResolution {
        if model_key.contains("pro") {
            JimengResolution::V30Pro
        } else if model_key.contains("1080") {
            JimengResolution::V30_1080p
        } else {
            JimengResolution::V30_720p
        }
    }

    fn detect_aspect_ratio(&self, request: &CreateVideoRequest) -> String {
        match request.orientation {
            crate::dto::video_dto::VideoOrientation::Portrait => "9:16".to_string(),
            crate::dto::video_dto::VideoOrientation::Landscape => "16:9".to_string(),
        }
    }

    fn is_jimeng_pending(task: &VideoGenerationTask) -> bool {
        matches!(task.status.as_str(), "pending" | "queued" | "processing")
            && task.generation_id.as_deref().map_or(false, |g| g.starts_with("jimeng_"))
    }

    /// Poll Volcengine for a single Jimeng task, update DB if done/failed.
    /// When done, downloads the video and uploads to OSS for permanent storage.
    async fn poll_jimeng_task(
        &self,
        conn: &mut crate::repository::video_repository::PgConnection,
        task: &VideoGenerationTask,
    ) -> Option<VideoGenerationTask> {
        let jimeng = self.jimeng_client.as_ref()?;
        let req_key = task.generation_id.as_deref()?;

        let handle = JimengTaskHandle {
            task_id: task.task_id.clone(),
            req_key: req_key.to_string(),
        };

        let result = match jimeng.get_task_status(&handle).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Jimeng poll failed for task {}: {}", task.task_id, e);
                return None;
            }
        };

        if result.is_done() {
            let temp_url = result.get_video_url();
            tracing::info!(
                "Jimeng task {} completed, temp_url={:?}",
                task.task_id,
                temp_url
            );

            let video_url = match temp_url {
                Some(ref url) => {
                    let oss_url = self.persist_video_to_oss(url, task.user_id, &task.task_id).await;
                    Some(oss_url.unwrap_or_else(|| url.clone()))
                }
                None => None,
            };

            VideoRepository::mark_as_succeeded(
                conn,
                task.id,
                Some(task.task_id.clone()),
                video_url,
                None,
                None,
                None,
                None,
                None,
            )
            .ok()
        } else if result.is_failed() {
            tracing::warn!("Jimeng task {} failed: {:?}", task.task_id, result.resp_data);
            VideoRepository::mark_as_failed(
                conn,
                task.id,
                format!("Jimeng generation failed: {}", result.resp_data.unwrap_or_default()),
            )
            .ok()
        } else {
            let _ = VideoRepository::update_status(
                conn,
                task.id,
                "processing".to_string(),
                None,
            );
            None
        }
    }

    fn is_temporary_cdn_url(url: &str) -> bool {
        url.contains("vvecloud") || url.contains("byted.org") || url.contains("volcvod.com")
    }

    /// Download video from temporary CDN URL and re-upload to Aliyun OSS.
    /// Returns the permanent OSS URL, or None if OSS is not configured or upload fails.
    async fn persist_video_to_oss(
        &self,
        temp_url: &str,
        user_id: i32,
        task_id: &str,
    ) -> Option<String> {
        if !Self::is_temporary_cdn_url(temp_url) {
            return Some(temp_url.to_string());
        }

        let oss_config = match self.oss_config.as_ref() {
            Some(c) => c,
            None => {
                tracing::error!(
                    "⚠ TEMPORARY CDN URL will expire in ~1h but OSS is NOT configured! \
                     Set OSS_ACCESS_KEY_ID/OSS_ACCESS_KEY_SECRET/OSS_ENDPOINT/OSS_BUCKET. \
                     task={}", task_id
                );
                return None;
            }
        };
        let oss = OssService::new(oss_config.clone());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .ok()?;

        tracing::info!("Persisting Jimeng video to OSS: task={}", task_id);

        let resp = match client.get(temp_url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                tracing::error!("Jimeng video download HTTP {}: task={}", r.status(), task_id);
                return None;
            }
            Err(e) => {
                tracing::error!("Jimeng video download failed: task={}, err={}", task_id, e);
                return None;
            }
        };

        let video_bytes = match resp.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                tracing::error!("Jimeng video read failed: task={}, err={}", task_id, e);
                return None;
            }
        };

        tracing::info!(
            "Downloaded Jimeng video: task={}, size={}",
            task_id,
            video_bytes.len()
        );

        let filename = format!("jimeng_{}.mp4", task_id);
        match oss
            .upload_video(video_bytes, user_id, filename, "video/mp4".to_string())
            .await
        {
            Ok(result) => {
                tracing::info!(
                    "Jimeng video persisted to OSS: task={}, url={}",
                    task_id,
                    result.image_url
                );
                Some(result.image_url)
            }
            Err(e) => {
                tracing::error!("OSS upload failed: task={}, err={}", task_id, e);
                None
            }
        }
    }

    fn task_to_response(task: VideoGenerationTask) -> VideoTaskResponse {
        VideoTaskResponse {
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
            ai_model_name: None,
            orientation: task.orientation,
            video_seconds: task.video_seconds,
            video_size: task.video_size,
        }
    }
}
