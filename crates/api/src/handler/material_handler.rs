use crate::dto::material_dto::{CreateMaterialRequest, MaterialListQuery, UpdateMaterialRequest};
use crate::error::api_error::ApiError;
use crate::error::request_error::ValidatedRequest;
use crate::response::api_result::ApiResult;
use crate::state::user_state::UserState;
use axum::extract::{Extension, Multipart, Path, Query, State};
use glance_mind_db::entity::user::User;
use tracing::info;

/// List user materials
/// GET /api/v1/materials
pub async fn list_materials(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Query(query): Query<MaterialListQuery>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialListResponse>, ApiError> {
    let response = state
        .material_service
        .list_materials(user.id, query)
        .await?;
    Ok(ApiResult::ok(response))
}

/// Get material by ID
/// GET /api/v1/materials/:id
pub async fn get_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state.material_service.get_material(id, user.id).await?;
    Ok(ApiResult::ok(material))
}

/// Create material (user upload)
/// POST /api/v1/materials
pub async fn create_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    ValidatedRequest(request): ValidatedRequest<CreateMaterialRequest>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .create_material(user.id, request)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Update material
/// PUT /api/v1/materials/:id
pub async fn update_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
    ValidatedRequest(request): ValidatedRequest<UpdateMaterialRequest>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .update_material(id, user.id, request)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Delete material
/// DELETE /api/v1/materials/:id
pub async fn delete_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<()>, ApiError> {
    state.material_service.delete_material(id, user.id).await?;
    Ok(ApiResult::ok(()))
}

/// Collect tags from video_cases
/// GET /api/v1/material-tags
pub async fn list_tags(
    State(state): State<UserState>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialTagsResponse>, ApiError> {
    let response = state.material_service.collect_tags().await?;
    Ok(ApiResult::ok(response))
}

/// Re-analyze material (trigger AI analysis retry)
/// POST /api/v1/materials/:id/analyze
pub async fn re_analyze_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .re_analyze_material(id, user.id)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Favorite material from video_case
/// POST /api/v1/video-cases/:task_no/favorite
pub async fn favorite_from_video_case(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    Path(task_no): Path<String>,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let material = state
        .material_service
        .favorite_from_video_case(user.id, &task_no)
        .await?;
    Ok(ApiResult::ok(material))
}

/// Upload material (unified: image/video/audio)
/// POST /api/v1/materials/upload
pub async fn upload_material(
    State(state): State<UserState>,
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<crate::dto::material_dto::MaterialDetail>, ApiError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut folder_id: Option<i32> = None;
    let mut title: Option<String> = None;
    let mut tag: Option<String> = None;
    let mut description: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().map(|s| s.to_string());
                file_name = field.file_name().map(|s| s.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
                file_data = Some(bytes.to_vec());
            }
            "folder_id" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    folder_id = text.parse().ok();
                }
            }
            "title" => {
                title = Some(field.text().await.unwrap_or_default());
            }
            "tag" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    tag = Some(text);
                }
            }
            "description" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    description = Some(text);
                }
            }
            _ => {}
        }
    }

    let file_bytes = file_data.ok_or_else(|| ApiError::BadRequest("No file provided".into()))?;
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());
    let fname = file_name.unwrap_or_else(|| "unknown".to_string());

    let media_type = detect_media_type(&ct);
    if media_type.is_none() {
        return Err(ApiError::BadRequest(format!(
            "Unsupported file type: {}. Supported: images (jpg/png/gif/webp), videos (mp4/mov/avi/webm/mkv), audio (mp3/wav/aac/ogg/flac)",
            ct
        )));
    }
    let media_type = media_type.unwrap();

    validate_file_size(&media_type, file_bytes.len())?;

    let auto_title = title.unwrap_or_else(|| match fname.rsplit_once('.') {
        Some((name, _ext)) => name.to_string(),
        None => fname.clone(),
    });

    info!(
        "Uploading material: user={}, type={}, size={}, name={}",
        user.id,
        media_type,
        file_bytes.len(),
        auto_title
    );

    let oss_url = state
        .material_service
        .upload_to_oss(&file_bytes, &fname, &ct)
        .await?;

    let request = CreateMaterialRequest {
        video_url: if media_type == "video" {
            Some(oss_url.clone())
        } else {
            None
        },
        file_url: if media_type != "video" {
            Some(oss_url)
        } else {
            None
        },
        tag,
        title: Some(auto_title),
        description,
        folder_id,
        media_type: Some(media_type),
        mime_type: Some(ct),
    };

    let material = state
        .material_service
        .create_material(user.id, request)
        .await?;

    Ok(ApiResult::ok(material))
}

fn detect_media_type(content_type: &str) -> Option<String> {
    let ct = content_type.to_lowercase();
    if ct.starts_with("image/") {
        let supported = [
            "image/jpeg",
            "image/png",
            "image/gif",
            "image/webp",
            "image/jpg",
        ];
        if supported.iter().any(|s| ct.starts_with(s)) {
            return Some("image".to_string());
        }
    }
    if ct.starts_with("video/") {
        let supported = [
            "video/mp4",
            "video/quicktime",
            "video/x-msvideo",
            "video/webm",
            "video/x-matroska",
        ];
        if supported.iter().any(|s| ct == *s) {
            return Some("video".to_string());
        }
    }
    if ct.starts_with("audio/") {
        let supported = [
            "audio/mpeg",
            "audio/wav",
            "audio/aac",
            "audio/ogg",
            "audio/flac",
            "audio/mp3",
            "audio/x-wav",
        ];
        if supported.iter().any(|s| ct.starts_with(s)) {
            return Some("audio".to_string());
        }
    }
    None
}

fn validate_file_size(media_type: &str, size: usize) -> Result<(), ApiError> {
    let max_bytes = match media_type {
        "image" => 20 * 1024 * 1024,
        "video" => 100 * 1024 * 1024,
        "audio" => 50 * 1024 * 1024,
        _ => return Err(ApiError::BadRequest("Unknown media type".into())),
    };
    if size > max_bytes {
        return Err(ApiError::BadRequest(format!(
            "File too large. Max size for {}: {} MB",
            media_type,
            max_bytes / 1024 / 1024
        )));
    }
    Ok(())
}
