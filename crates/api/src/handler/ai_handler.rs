use crate::api_ok;
use crate::dto::ai_dto::AiGenerateRequest;
use crate::error::api_error::ApiError;
use crate::service::ai_service::AiService;
use axum::{response::IntoResponse, Json};
use validator::Validate;

/// AI content generation endpoint
///
/// Returns unified ApiResponse format
pub async fn generate_content(
    Json(payload): Json<AiGenerateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request parameters
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Call AI service to generate content
    let response = AiService::generate(payload)
        .await
        .map_err(ApiError::AiServiceError)?;

    // Return success response
    Ok(api_ok!(response))
}
