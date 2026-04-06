use crate::dto::email_verification_dto::{
    ResetPasswordRequest, ResetPasswordResponse, SendPasswordResetCodeRequest,
    SendVerificationCodeRequest, SendVerificationCodeResponse, VerifyCodeRequest,
    VerifyCodeResponse,
};
use crate::error::{api_error::ApiError, business_error::BusinessError};
use crate::response::ApiResult;
use crate::service::email_verification_service::EmailVerificationService;
use axum::extract::State;
use axum::http::HeaderMap;
use std::sync::Arc;
use validator::Validate;

/// Send verification code
pub async fn send_verification_code(
    State(service): State<Arc<EmailVerificationService>>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<SendVerificationCodeRequest>,
) -> Result<ApiResult<SendVerificationCodeResponse>, ApiError> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e.to_string())))?;

    // Get IP and User-Agent (first IP only from X-Forwarded-For chain)
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = service
        .send_verification_code(request, ip_address, user_agent)
        .await?;

    Ok(ApiResult::ok(response))
}

/// Verify verification code
pub async fn verify_code(
    State(service): State<Arc<EmailVerificationService>>,
    axum::Json(request): axum::Json<VerifyCodeRequest>,
) -> Result<ApiResult<VerifyCodeResponse>, ApiError> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e.to_string())))?;

    let response = service.verify_code(request).await?;

    Ok(ApiResult::ok(response))
}

/// Send password reset verification code
#[allow(dead_code)]
pub async fn send_password_reset_code(
    State(service): State<Arc<EmailVerificationService>>,
    headers: HeaderMap,
    axum::Json(request): axum::Json<SendPasswordResetCodeRequest>,
) -> Result<ApiResult<SendVerificationCodeResponse>, ApiError> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e.to_string())))?;

    // Get IP and User-Agent (first IP only from X-Forwarded-For chain)
    let ip_address = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = service
        .send_password_reset_code(request, ip_address, user_agent)
        .await?;

    Ok(ApiResult::ok(response))
}

/// Reset password
#[allow(dead_code)]
pub async fn reset_password(
    State(service): State<Arc<EmailVerificationService>>,
    axum::Json(request): axum::Json<ResetPasswordRequest>,
) -> Result<ApiResult<ResetPasswordResponse>, ApiError> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::BusinessError(BusinessError::ValidationFailed(e.to_string())))?;

    let response = service.reset_password(request).await?;

    Ok(ApiResult::ok(response))
}
