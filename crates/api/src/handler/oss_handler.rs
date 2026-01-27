//! OSS Upload Handler
//!
//! Handles image uploads to Aliyun OSS for AI Publish module.

use axum::extract::Multipart;
use axum::Extension;

use crate::dto::oss_dto::UploadImageResponse;
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::response::api_result::ApiResult;
use crate::service::oss_service::{OssConfig, OssService};
use glance_mind_db::entity::user::User;
use tracing::{error, info};

/// Maximum file size: 10MB
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Allowed image content types
const ALLOWED_CONTENT_TYPES: &[&str] = &[
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
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
    if !ALLOWED_CONTENT_TYPES.contains(&content_type.as_str()) {
        return Err(ApiError::BusinessError(BusinessError::InvalidFileType(
            content_type,
        )));
    }

    // Validate file size
    if data.len() > MAX_FILE_SIZE {
        return Err(ApiError::BusinessError(BusinessError::FileTooLarge(
            MAX_FILE_SIZE,
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
