//! OSS Upload Handler
//!
//! Handles image uploads to Aliyun OSS for AI Publish module.

use axum::extract::Multipart;
use axum::Extension;

use crate::dto::oss_dto::{UploadAudioResponse, UploadImageResponse, UploadVideoResponse};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::response::api_result::ApiResult;
use crate::service::oss_service::{OssConfig, OssService};
use glance_mind_db::entity::user::User;
use tracing::{error, info};

/// Maximum file size for images: 30MB (Seedance supports up to 20MB, extra buffer)
const MAX_IMAGE_FILE_SIZE: usize = 30 * 1024 * 1024;

/// Maximum file size for videos: 100MB
const MAX_VIDEO_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Maximum file size for audio: 15MB
const MAX_AUDIO_FILE_SIZE: usize = 15 * 1024 * 1024;

/// Allowed image content types
const ALLOWED_IMAGE_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/tiff",
];

/// Allowed video content types
const ALLOWED_VIDEO_CONTENT_TYPES: &[&str] = &[
    "video/mp4",
    "video/quicktime", // mov
    "video/x-msvideo", // avi
    "video/webm",
    "video/x-matroska", // mkv
    "video/x-flv",
    "video/x-ms-wmv", // wmv
    "video/x-m4v",    // m4v
];

/// Allowed audio content types
const ALLOWED_AUDIO_CONTENT_TYPES: &[&str] = &[
    "audio/mpeg",  // mp3
    "audio/mp3",   // mp3 alt
    "audio/wav",   // wav
    "audio/x-wav", // wav alt
    "audio/wave",  // wav alt
    "audio/ogg",   // ogg
    "audio/aac",   // aac
    "audio/x-m4a", // m4a
    "audio/mp4",   // m4a alt
];

/// Upload image to OSS
/// POST /api/v1/aipub/upload-image
/// Content-Type: multipart/form-data
/// Field: "file" - the image file
pub async fn upload_image(
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<UploadImageResponse>, ApiError> {
    // Check if OSS is configured
    if !OssConfig::is_configured() {
        return Err(ApiError::InternalServerError(
            "OSS service not configured".to_string(),
        ));
    }

    // Parse multipart form data
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_filename: Option<String> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" || field_name == "image" {
            image_filename = field.file_name().map(|s| s.to_string());
            image_content_type = field.content_type().map(|s| s.to_string());

            let data = field.bytes().await.map_err(|e| {
                error!("Failed to read file data: {:?}", e);
                ApiError::BusinessError(BusinessError::InvalidFormField("file".to_string()))
            })?;

            image_data = Some(data.to_vec());
        }
    }

    // Validate file was provided
    let data = image_data.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("file".to_string()))
    })?;

    let filename = image_filename.unwrap_or_else(|| "image.jpg".to_string());
    let content_type = image_content_type.unwrap_or_else(|| "image/jpeg".to_string());

    // Validate content type
    if !ALLOWED_IMAGE_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BusinessError(BusinessError::InvalidFileType(
            content_type,
        )));
    }

    // Validate file size
    if data.len() > MAX_IMAGE_FILE_SIZE {
        return Err(ApiError::BusinessError(BusinessError::FileTooLarge(
            MAX_IMAGE_FILE_SIZE,
        )));
    }

    info!(
        "Uploading image: filename={}, content_type={}, size={} bytes, user_id={}",
        filename,
        content_type,
        data.len(),
        user.id
    );

    // Create OSS service and upload (async)
    let oss_service = OssService::from_env()?;
    let result = oss_service
        .upload_image(data, user.id, filename.clone(), content_type.clone())
        .await?;

    info!(
        "Image uploaded successfully: url={}, user_id={}",
        result.image_url, user.id
    );

    Ok(ApiResult::ok(UploadImageResponse {
        image_url: result.image_url,
        filename: result.filename,
        size: result.size,
    }))
}

/// Upload video to OSS
/// POST /api/v1/oss/upload-video
/// Content-Type: multipart/form-data
/// Field: "file" or "video" - the video file
pub async fn upload_video(
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<UploadVideoResponse>, ApiError> {
    // Parse multipart form data first to validate request
    let mut video_data: Option<Vec<u8>> = None;
    let mut video_filename: Option<String> = None;
    let mut video_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" || field_name == "video" {
            video_filename = field.file_name().map(|s| s.to_string());
            video_content_type = field.content_type().map(|s| s.to_string());

            let data = field.bytes().await.map_err(|e| {
                error!("Failed to read video file data: {:?}", e);
                ApiError::BusinessError(BusinessError::InvalidFormField("file".to_string()))
            })?;

            video_data = Some(data.to_vec());
        }
    }

    // Validate file was provided
    let data = video_data.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("file".to_string()))
    })?;

    let filename = video_filename.unwrap_or_else(|| "video.mp4".to_string());
    let content_type = video_content_type.unwrap_or_else(|| "video/mp4".to_string());

    // Validate content type before checking OSS availability
    if !ALLOWED_VIDEO_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BusinessError(BusinessError::InvalidFileType(
            content_type,
        )));
    }

    // Validate file size
    if data.len() > MAX_VIDEO_FILE_SIZE {
        return Err(ApiError::BusinessError(BusinessError::FileTooLarge(
            MAX_VIDEO_FILE_SIZE,
        )));
    }

    // Check if OSS is configured (after input validation)
    if !OssConfig::is_configured() {
        return Err(ApiError::InternalServerError(
            "OSS service not configured".to_string(),
        ));
    }

    info!(
        "Uploading video: filename={}, content_type={}, size={} bytes, user_id={}",
        filename,
        content_type,
        data.len(),
        user.id
    );

    // Create OSS service and upload (async)
    let oss_service = OssService::from_env()?;
    let result = oss_service
        .upload_video(data, user.id, filename.clone(), content_type.clone())
        .await?;

    info!(
        "Video uploaded successfully: url={}, user_id={}",
        result.image_url, user.id
    );

    Ok(ApiResult::ok(UploadVideoResponse {
        video_url: result.image_url, // Reuse image_url field as video_url
        filename: result.filename,
        size: result.size,
        duration: None, // Duration extraction can be added later if needed
    }))
}

/// Upload audio to OSS
/// POST /api/v1/oss/upload-audio
/// Content-Type: multipart/form-data
/// Field: "file" or "audio" - the audio file
pub async fn upload_audio(
    Extension(user): Extension<User>,
    mut multipart: Multipart,
) -> Result<ApiResult<UploadAudioResponse>, ApiError> {
    let mut audio_data: Option<Vec<u8>> = None;
    let mut audio_filename: Option<String> = None;
    let mut audio_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::BusinessError(BusinessError::FormParsingFailed))?
    {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" || field_name == "audio" {
            audio_filename = field.file_name().map(|s| s.to_string());
            audio_content_type = field.content_type().map(|s| s.to_string());

            let data = field.bytes().await.map_err(|e| {
                error!("Failed to read audio file data: {:?}", e);
                ApiError::BusinessError(BusinessError::InvalidFormField("file".to_string()))
            })?;

            audio_data = Some(data.to_vec());
        }
    }

    let data = audio_data.ok_or_else(|| {
        ApiError::BusinessError(BusinessError::MissingRequiredParameter("file".to_string()))
    })?;

    let filename = audio_filename.unwrap_or_else(|| "audio.mp3".to_string());
    let content_type = audio_content_type.unwrap_or_else(|| "audio/mpeg".to_string());

    if !ALLOWED_AUDIO_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BusinessError(BusinessError::InvalidFileType(
            content_type,
        )));
    }

    if data.len() > MAX_AUDIO_FILE_SIZE {
        return Err(ApiError::BusinessError(BusinessError::FileTooLarge(
            MAX_AUDIO_FILE_SIZE,
        )));
    }

    if !OssConfig::is_configured() {
        return Err(ApiError::InternalServerError(
            "OSS service not configured".to_string(),
        ));
    }

    info!(
        "Uploading audio: filename={}, content_type={}, size={} bytes, user_id={}",
        filename,
        content_type,
        data.len(),
        user.id
    );

    let oss_service = OssService::from_env()?;
    let result = oss_service
        .upload_audio(data, user.id, filename.clone(), content_type.clone())
        .await?;

    info!(
        "Audio uploaded successfully: url={}, user_id={}",
        result.image_url, user.id
    );

    Ok(ApiResult::ok(UploadAudioResponse {
        audio_url: result.image_url,
        filename: result.filename,
        size: result.size,
    }))
}
