use serde::{Deserialize, Serialize};
use validator::Validate;

/// Send verification code request
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SendVerificationCodeRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub turnstile_token: String,
}

/// Send verification code response
#[derive(Debug, Serialize, Deserialize)]
pub struct SendVerificationCodeResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_minutes: Option<i32>,
}

/// Verify verification code request
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct VerifyCodeRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(equal = 6, message = "Code must be 6 digits"))]
    pub code: String,
}

/// Verify verification code response
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyCodeResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Send password reset verification code request
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct SendPasswordResetCodeRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub turnstile_token: String,
}

/// Reset password request
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(equal = 6, message = "Code must be 6 digits"))]
    pub code: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

/// Reset password response
#[derive(Debug, Serialize, Deserialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
    pub message: String,
}
