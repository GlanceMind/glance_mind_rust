use axum::{
    extract::{Multipart, Query, State},
    Extension,
};
use serde::{Deserialize, Serialize};

use crate::dto::video_dto::{
    CreateVideoRequest, CreateVideoResponse, VideoOrientation, VideoTaskListResponse,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::middleware::charging::ActionType;
use crate::response::api_result::ApiResult;
use crate::state::user_state::UserState;
use glance_mind_db::entity::user::User;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

/// Create video generation task (supports text-to-video and image-to-video)
/// POST /api/v1/video/generate
/// Content-Type: multipart/form-data
pub async fn create_video(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<CreateVideoResponse>, ApiError> {
    // Parse multipart form data
    let mut title: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut ai_model_id: Option<i32> = None;
    let mut orientation: Option<VideoOrientation> = None;
    let mut seconds: Option<String> = None;
    let mut size: Option<String> = None;
    // Single image mode (backward compatible)
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_filename: Option<String> = None;
    let mut image_content_type: Option<String> = None;
    // Dual image mode (new)
    let mut start_frame_data: Option<Vec<u8>> = None;
    let mut start_frame_filename: Option<String> = None;
    let mut end_frame_data: Option<Vec<u8>> = None;
    let mut end_frame_filename: Option<String> = None;
    // Multi-image mode (ref2v / multiframe: up to 9 images)
    let mut reference_images: Vec<Vec<u8>> = Vec::new();
    // Per-keyframe transition prompts (multi-frame only, JSON-encoded string[])
    let mut keyframe_prompts: Option<Vec<String>> = None;
    // Vidu unified mode fields
    let mut vidu_mode: Option<String> = None;
    let mut vidu_quality: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "title" => {
                title = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("title".to_string()))
                })?);
            }
            "prompt" => {
                prompt = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("prompt".to_string()))
                })?);
            }
            "ai_model_id" => {
                let text = field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "ai_model_id".to_string(),
                    ))
                })?;
                ai_model_id = text.parse().ok();
            }
            "orientation" => {
                let text = field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "orientation".to_string(),
                    ))
                })?;
                orientation = match text.to_lowercase().as_str() {
                    "portrait" => Some(VideoOrientation::Portrait),
                    "landscape" => Some(VideoOrientation::Landscape),
                    _ => None,
                };
            }
            "seconds" => {
                seconds = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("seconds".to_string()))
                })?);
            }
            "size" => {
                size = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("size".to_string()))
                })?);
            }
            "image" | "file" => {
                // Single image mode (backward compatible)
                image_filename = field.file_name().map(|s| s.to_string());
                image_content_type = field.content_type().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("image".to_string()))
                })?;
                image_data = Some(data.to_vec());
            }
            "start_frame" => {
                // Dual image mode: start frame
                start_frame_filename = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "start_frame".to_string(),
                    ))
                })?;
                start_frame_data = Some(data.to_vec());
            }
            "end_frame" => {
                // Dual image mode: end frame
                end_frame_filename = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "end_frame".to_string(),
                    ))
                })?;
                end_frame_data = Some(data.to_vec());
            }
            "reference_images" => {
                let data = field.bytes().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "reference_images".to_string(),
                    ))
                })?;
                if !data.is_empty() {
                    reference_images.push(data.to_vec());
                }
            }
            "keyframe_prompts" => {
                let text = field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "keyframe_prompts".to_string(),
                    ))
                })?;
                keyframe_prompts = serde_json::from_str::<Vec<String>>(&text).ok();
            }
            "vidu_mode" => {
                vidu_mode = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "vidu_mode".to_string(),
                    ))
                })?);
            }
            "vidu_quality" => {
                vidu_quality = Some(field.text().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField(
                        "vidu_quality".to_string(),
                    ))
                })?);
            }
            _ => {}
        }
    }

    // Keep reference_images intact for modes that consume them (ref2v, multiframe, film, template).
    // Only collapse to start_frame/end_frame when NO reference_images were sent and no start_frame
    // was explicitly provided -- this preserves backward compatibility for dual-image modes
    // (start-end, jimeng) that use the "start_frame"/"end_frame" field names directly.

    // Validate required fields: need at least a prompt or any image
    if prompt.is_none()
        && image_data.is_none()
        && start_frame_data.is_none()
        && reference_images.is_empty()
    {
        return Err(ApiError::BusinessError(
            BusinessError::PromptOrImageRequired,
        ));
    }

    let seconds = seconds.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter(
            "seconds".to_string(),
        ))
    })?;

    let size = size.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("size".to_string()))
    })?;

    // Build request object
    let request = CreateVideoRequest {
        title,
        prompt,
        ai_model_id,
        orientation: orientation.unwrap_or_default(),
        seconds,
        size,
        vidu_mode,
        vidu_quality,
    };

    // Validate parameters
    request
        .validate_params()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e)))?;

    // Charge video generation using the parsed multipart model id so billing
    // always matches the actual model selected by the request body.
    let charging_context = state
        .charging_manager
        .prepare_charging(
            user.id,
            ActionType::VideoGenerate,
            request.ai_model_id,
            None,
        )
        .await?;

    // Call service to process
    let result = state
        .video_service
        .create_video(
            user.id,
            request,
            image_data,
            image_filename,
            image_content_type,
            start_frame_data,
            start_frame_filename,
            end_frame_data,
            end_frame_filename,
            reference_images,
            keyframe_prompts,
        )
        .await?;

    state
        .charging_manager
        .execute_charging(user.id, &charging_context, None)
        .await?;

    Ok(ApiResult::ok(result))
}

/// Get user's video task list
/// GET /api/v1/video/tasks
pub async fn get_user_tasks(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(params): Query<PaginationParams>,
) -> Result<ApiResult<VideoTaskListResponse>, ApiError> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let result = state
        .video_service
        .get_user_tasks(user.id, page, page_size)
        .await?;

    Ok(ApiResult::ok(result))
}

/// Get single video task details
/// GET /api/v1/video/tasks/:task_id
pub async fn get_user_task(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Result<ApiResult<crate::dto::video_dto::VideoTaskResponse>, ApiError> {
    let result = state.video_service.get_user_task(user.id, &task_id).await?;

    Ok(ApiResult::ok(result))
}

#[derive(Debug, Serialize)]
pub struct UploadImageResponse {
    pub media_id: String,
    pub cost_points: i64,
}

/// Upload image to get media_id
/// POST /api/v1/video/upload-image
/// Content-Type: multipart/form-data
pub async fn upload_image(
    State(_state): State<UserState>,
    Extension(_user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<UploadImageResponse>, ApiError> {
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_filename: Option<String> = None;

    // Parse multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "image" | "file" => {
                image_filename = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|_| {
                    ApiError::BusinessError(BusinessError::InvalidFormField("image".to_string()))
                })?;
                image_data = Some(data.to_vec());
            }
            _ => {}
        }
    }

    // Validate image data exists
    let _data = image_data.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("image".to_string()))
    })?;
    let filename = image_filename.unwrap_or_else(|| "image.png".to_string());

    // Generate temporary media_id (in production may need to upload to storage service)
    let media_id = format!("img_{}_{}", chrono::Utc::now().timestamp(), filename);

    // Return response
    Ok(ApiResult::ok(UploadImageResponse {
        media_id,
        cost_points: 10, // Fixed cost of 10 points
    }))
}
