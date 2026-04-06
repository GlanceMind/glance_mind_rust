use crate::error::{
    business_error::BusinessError, db_error::DbError, infrastructure_error::InfrastructureError,
    token_error::TokenError, user_error::UserError,
};
use crate::response::error_code::ErrorCode;
use crate::response::unified_response::ApiResponse;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    // ============ General Errors ============
    #[error("Internal Server Error: {0}")]
    InternalServerError(String),

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),

    // ============ Business Errors - User Related ============
    #[error("User not found")]
    UserNotFound,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Email already in use")]
    EmailAlreadyInUse,

    #[error("Invalid verification code")]
    InvalidVerificationCode,

    #[error("Verification code expired")]
    VerificationCodeExpired,

    // ============ Business Errors - Permission Related ============
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    // ============ Business Errors - Wallet Related ============
    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Charge failed: {0}")]
    ChargeFailed(String),

    #[error("Pricing rule not found: {0}")]
    PricingRuleNotFound(String),

    // ============ Business Errors - Resource Related ============
    #[error("Template not found")]
    TemplateNotFound,

    #[error("Campaign not found")]
    CampaignNotFound,

    #[error("Task not found")]
    TaskNotFound,

    #[error("Video not found")]
    VideoNotFound,

    // ============ Business Errors - Social Account Related ============
    #[error("Social account not found")]
    SocialAccountNotFound,

    #[error("Social account already bound")]
    SocialAccountAlreadyBound,

    // ============ Business Errors - Promo Code Related ============
    #[error("Promo code not found")]
    PromoCodeNotFound,

    #[error("Promo code expired")]
    PromoCodeExpired,

    #[error("Promo code already used")]
    PromoCodeAlreadyUsed,

    #[error("Promo code usage limit reached")]
    PromoCodeUsageLimitReached,

    // ============ Third-party Service Errors ============
    #[error("AI service error: {0}")]
    AiServiceError(String),

    #[error("Email service error: {0}")]
    EmailServiceError(String),

    #[error("Payment service error: {0}")]
    PaymentServiceError(String),

    #[error("TikTok API error: {0}")]
    TikTok(String),

    #[error("Instagram API error: {0}")]
    Instagram(String),

    #[error("Twitter API error: {0}")]
    Twitter(String),

    #[error("Reddit API error: {0}")]
    Reddit(String),

    // ============ Database Errors ============
    #[error("Database error: {0}")]
    DatabaseError(String),

    // ============ Other Error Types ============
    #[error(transparent)]
    TokenError(#[from] TokenError),

    #[error(transparent)]
    UserError(#[from] UserError),

    #[error(transparent)]
    DbError(#[from] DbError),

    #[error(transparent)]
    BusinessError(#[from] BusinessError),

    #[error(transparent)]
    InfrastructureError(#[from] InfrastructureError),
}

impl ApiError {
    /// Convert ApiError to ErrorCode
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            // General errors
            ApiError::InternalServerError(_) => ErrorCode::InternalServerError,
            ApiError::BadRequest(_) => ErrorCode::BadRequest,
            ApiError::Forbidden(_) => ErrorCode::Forbidden,
            ApiError::NotFound(_) => ErrorCode::NotFound,
            ApiError::Unauthorized(_) => ErrorCode::Unauthorized,
            ApiError::ValidationError(_) => ErrorCode::ValidationError,

            // User related
            ApiError::UserNotFound => ErrorCode::UserNotFound,
            ApiError::UserAlreadyExists => ErrorCode::UserAlreadyExists,
            ApiError::InvalidPassword => ErrorCode::InvalidPassword,
            ApiError::EmailAlreadyInUse => ErrorCode::EmailAlreadyInUse,
            ApiError::InvalidVerificationCode => ErrorCode::InvalidVerificationCode,
            ApiError::VerificationCodeExpired => ErrorCode::VerificationCodeExpired,

            // Permission related
            ApiError::PermissionDenied(_) => ErrorCode::PermissionDenied,

            // Wallet related
            ApiError::InsufficientBalance => ErrorCode::InsufficientBalance,
            ApiError::ChargeFailed(_) => ErrorCode::ChargeFailed,
            ApiError::PricingRuleNotFound(_) => ErrorCode::PricingRuleNotFound,

            // Resource related
            ApiError::TemplateNotFound => ErrorCode::TemplateNotFound,
            ApiError::CampaignNotFound => ErrorCode::CampaignNotFound,
            ApiError::TaskNotFound => ErrorCode::TaskNotFound,
            ApiError::VideoNotFound => ErrorCode::VideoNotFound,

            // Social account related
            ApiError::SocialAccountNotFound => ErrorCode::SocialAccountNotFound,
            ApiError::SocialAccountAlreadyBound => ErrorCode::SocialAccountAlreadyBound,

            // Promo code related
            ApiError::PromoCodeNotFound => ErrorCode::PromoCodeNotFound,
            ApiError::PromoCodeExpired => ErrorCode::PromoCodeExpired,
            ApiError::PromoCodeAlreadyUsed => ErrorCode::PromoCodeAlreadyUsed,
            ApiError::PromoCodeUsageLimitReached => ErrorCode::PromoCodeUsageLimitReached,

            // Third-party services
            ApiError::AiServiceError(_) => ErrorCode::AiServiceError,
            ApiError::EmailServiceError(_) => ErrorCode::EmailServiceError,
            ApiError::PaymentServiceError(_) => ErrorCode::PaymentServiceError,
            ApiError::TikTok(_) => ErrorCode::TikTokApiError,
            ApiError::Instagram(_) => ErrorCode::InstagramApiError,
            ApiError::Twitter(_) => ErrorCode::TwitterApiError,
            ApiError::Reddit(_) => ErrorCode::RedditApiError,

            // Database errors
            ApiError::DatabaseError(_) => ErrorCode::DatabaseError,

            // Other error types
            ApiError::TokenError(err) => err.to_error_code(),
            ApiError::UserError(err) => err.to_error_code(),
            ApiError::DbError(_) => ErrorCode::DatabaseError,
            ApiError::BusinessError(err) => err.to_error_code(),
            ApiError::InfrastructureError(err) => err.to_error_code(),
        }
    }

    /// Get Chinese error message (safe for client display)
    pub fn to_message_cn(&self) -> String {
        match self {
            ApiError::InternalServerError(_) => "Internal server error".to_string(),
            ApiError::DatabaseError(_) => "Database operation failed".to_string(),
            ApiError::BadRequest(msg) => format!("Bad request: {}", msg),
            ApiError::Forbidden(msg) => format!("Access forbidden: {}", msg),
            ApiError::NotFound(msg) => format!("Resource not found: {}", msg),
            ApiError::Unauthorized(msg) => format!("Unauthorized: {}", msg),
            ApiError::ValidationError(msg) => format!("Validation failed: {}", msg),
            ApiError::PermissionDenied(feature) => format!("Feature not enabled: {}", feature),
            ApiError::ChargeFailed(msg) => format!("Charge failed: {}", msg),
            ApiError::PricingRuleNotFound(action) => format!("Pricing rule not found: {}", action),
            ApiError::AiServiceError(_) => "AI service error, please try again later".to_string(),
            ApiError::EmailServiceError(_) => {
                "Email service error, please try again later".to_string()
            }
            ApiError::PaymentServiceError(_) => {
                "Payment service error, please try again later".to_string()
            }
            ApiError::TikTok(_) => "TikTok API error".to_string(),
            ApiError::Instagram(_) => "Instagram API error".to_string(),
            ApiError::Twitter(_) => "Twitter API error".to_string(),
            ApiError::Reddit(_) => "Reddit API error".to_string(),

            ApiError::BusinessError(err) => err.to_message_cn(),
            ApiError::InfrastructureError(err) => err.to_message_cn(),

            _ => self.to_error_code().message_cn().to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let error_code = self.to_error_code();

        let (msg, msg_cn) = match &self {
            ApiError::InternalServerError(detail) => {
                tracing::error!("Internal server error: {}", detail);
                (
                    "Internal server error".to_string(),
                    "Internal server error".to_string(),
                )
            }
            ApiError::DatabaseError(detail) => {
                tracing::error!("Database error: {}", detail);
                (
                    "A database error occurred".to_string(),
                    "Database operation failed".to_string(),
                )
            }
            ApiError::TokenError(token_err) => {
                if let crate::error::token_error::TokenError::TokenCreationError(detail) = token_err
                {
                    tracing::error!("Token creation error: {}", detail);
                    (
                        "Authentication service error".to_string(),
                        "Authentication service error".to_string(),
                    )
                } else {
                    (self.to_string(), self.to_message_cn())
                }
            }
            _ => (self.to_string(), self.to_message_cn()),
        };

        ApiResponse::<()>::error_with_message(error_code, msg, msg_cn).into_response()
    }
}
