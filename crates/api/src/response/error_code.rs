/// Unified error code definition
///
/// Rules:
/// - 1xxx: Success
/// - 2xxx: Client errors
/// - 3xxx: Server errors
/// - 4xxx: Business logic errors
/// - 5xxx: Third-party service errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // ============ Success ============
    /// Operation successful
    Success = 1000,

    // ============ Client Errors (2xxx) ============
    /// Parameter validation failed
    ValidationError = 2000,
    /// Bad request parameters
    BadRequest = 2001,
    /// Unauthorized (not logged in)
    Unauthorized = 2002,
    /// Access forbidden
    Forbidden = 2003,
    /// Resource not found
    NotFound = 2004,
    /// Method not allowed
    MethodNotAllowed = 2005,
    /// Too many requests
    TooManyRequests = 2006,
    /// Token expired
    TokenExpired = 2007,
    /// Token invalid
    TokenInvalid = 2008,

    // ============ Server Errors (3xxx) ============
    /// Internal server error
    InternalServerError = 3000,
    /// Database error
    DatabaseError = 3001,
    /// Cache error
    CacheError = 3002,
    /// Configuration error
    ConfigError = 3003,

    // ============ Business Logic Errors (4xxx) ============
    /// User not found
    UserNotFound = 4000,
    /// User already exists
    UserAlreadyExists = 4001,
    /// Invalid password
    InvalidPassword = 4002,
    /// Email already in use
    EmailAlreadyInUse = 4003,
    /// Invalid verification code
    InvalidVerificationCode = 4004,
    /// Verification code expired
    VerificationCodeExpired = 4005,

    /// Permission denied (feature not enabled)
    PermissionDenied = 4050,

    /// Insufficient balance
    InsufficientBalance = 4100,
    /// Charge failed
    ChargeFailed = 4101,
    /// Pricing rule not found
    PricingRuleNotFound = 4102,

    /// Template not found
    TemplateNotFound = 4200,
    /// Campaign not found
    CampaignNotFound = 4201,
    /// Task not found
    TaskNotFound = 4202,
    /// Video not found
    VideoNotFound = 4203,

    /// Social account not found
    SocialAccountNotFound = 4300,
    /// Social account already bound
    SocialAccountAlreadyBound = 4301,

    /// Promo code not found
    PromoCodeNotFound = 4400,
    /// Promo code expired
    PromoCodeExpired = 4401,
    /// Promo code already used
    PromoCodeAlreadyUsed = 4402,
    /// Promo code usage limit reached
    PromoCodeUsageLimitReached = 4403,

    /// Task-template draft: the draft has passed its TTL and can no longer be acted on
    DraftExpired = 4500,
    /// Task-template draft: the draft is not in a state that permits this action
    DraftNotActionable = 4501,
    /// Task-template draft: the requested template/sample is unavailable
    TemplateUnavailable = 4502,
    /// Batch create: more than one item supplied (single-item MVP only)
    MultiItemNotSupported = 4503,
    /// Batch create: a batch with this idempotency key is still in progress
    BatchInProgress = 4504,

    // ============ Third-party Service Errors (5xxx) ============
    /// AI service error
    AiServiceError = 5000,
    /// Email service error
    EmailServiceError = 5001,
    /// Payment service error
    PaymentServiceError = 5002,
    /// TikTok API error
    TikTokApiError = 5100,
    /// Instagram API error
    InstagramApiError = 5101,
    /// Twitter API error
    TwitterApiError = 5102,
    /// Reddit API error
    RedditApiError = 5103,
}

impl ErrorCode {
    /// Get error code value
    pub fn code(&self) -> i32 {
        *self as i32
    }

    /// Get error message (English)
    pub fn message(&self) -> &'static str {
        match self {
            // Success
            ErrorCode::Success => "Success",

            // Client errors
            ErrorCode::ValidationError => "Validation error",
            ErrorCode::BadRequest => "Bad request",
            ErrorCode::Unauthorized => "Unauthorized",
            ErrorCode::Forbidden => "Forbidden",
            ErrorCode::NotFound => "Resource not found",
            ErrorCode::MethodNotAllowed => "Method not allowed",
            ErrorCode::TooManyRequests => "Too many requests",
            ErrorCode::TokenExpired => "Token expired",
            ErrorCode::TokenInvalid => "Token invalid",

            // Server errors
            ErrorCode::InternalServerError => "Internal server error",
            ErrorCode::DatabaseError => "Database error",
            ErrorCode::CacheError => "Cache error",
            ErrorCode::ConfigError => "Configuration error",

            // Business logic errors - User related
            ErrorCode::UserNotFound => "User not found",
            ErrorCode::UserAlreadyExists => "User already exists",
            ErrorCode::InvalidPassword => "Invalid password",
            ErrorCode::EmailAlreadyInUse => "Email already in use",
            ErrorCode::InvalidVerificationCode => "Invalid verification code",
            ErrorCode::VerificationCodeExpired => "Verification code expired",

            // Business logic errors - Permission related
            ErrorCode::PermissionDenied => "Feature not enabled",

            // Business logic errors - Wallet related
            ErrorCode::InsufficientBalance => "Insufficient balance",
            ErrorCode::ChargeFailed => "Charge failed",
            ErrorCode::PricingRuleNotFound => "Pricing rule not found",

            // Business logic errors - Resource related
            ErrorCode::TemplateNotFound => "Template not found",
            ErrorCode::CampaignNotFound => "Campaign not found",
            ErrorCode::TaskNotFound => "Task not found",
            ErrorCode::VideoNotFound => "Video not found",

            // Business logic errors - Social account related
            ErrorCode::SocialAccountNotFound => "Social account not found",
            ErrorCode::SocialAccountAlreadyBound => "Social account already bound",

            // Business logic errors - Promo code related
            ErrorCode::PromoCodeNotFound => "Promo code not found",
            ErrorCode::PromoCodeExpired => "Promo code expired",
            ErrorCode::PromoCodeAlreadyUsed => "Promo code already used",
            ErrorCode::PromoCodeUsageLimitReached => "Promo code usage limit reached",

            // Business logic errors - Task-template draft
            ErrorCode::DraftExpired => "This draft has expired",
            ErrorCode::DraftNotActionable => "This draft cannot be acted on in its current state",
            ErrorCode::TemplateUnavailable => "The requested template is unavailable",

            // Business logic errors - Batch create
            ErrorCode::MultiItemNotSupported => "Multiple items are not supported yet",
            ErrorCode::BatchInProgress => "A batch with this idempotency key is still in progress",

            // Third-party service errors
            ErrorCode::AiServiceError => "AI service error",
            ErrorCode::EmailServiceError => "Email service error",
            ErrorCode::PaymentServiceError => "Payment service error",
            ErrorCode::TikTokApiError => "TikTok API error",
            ErrorCode::InstagramApiError => "Instagram API error",
            ErrorCode::TwitterApiError => "Twitter API error",
            ErrorCode::RedditApiError => "Reddit API error",
        }
    }

    /// Get error message (Chinese)
    pub fn message_cn(&self) -> &'static str {
        match self {
            // Success
            ErrorCode::Success => "Success",

            // Client errors
            ErrorCode::ValidationError => "Validation failed",
            ErrorCode::BadRequest => "Bad request",
            ErrorCode::Unauthorized => "Unauthorized, please login",
            ErrorCode::Forbidden => "Access forbidden",
            ErrorCode::NotFound => "Resource not found",
            ErrorCode::MethodNotAllowed => "Method not allowed",
            ErrorCode::TooManyRequests => "Too many requests, please try again later",
            ErrorCode::TokenExpired => "Session expired, please login again",
            ErrorCode::TokenInvalid => "Invalid authentication",

            // Server errors
            ErrorCode::InternalServerError => "Internal server error",
            ErrorCode::DatabaseError => "Database error",
            ErrorCode::CacheError => "Cache error",
            ErrorCode::ConfigError => "Configuration error",

            // Business logic errors - User related
            ErrorCode::UserNotFound => "User not found",
            ErrorCode::UserAlreadyExists => "User already exists",
            ErrorCode::InvalidPassword => "Invalid password",
            ErrorCode::EmailAlreadyInUse => "Email already in use",
            ErrorCode::InvalidVerificationCode => "Invalid verification code",
            ErrorCode::VerificationCodeExpired => "Verification code expired",

            // Business logic errors - Permission related
            ErrorCode::PermissionDenied => "Feature not enabled, please contact support",
            ErrorCode::DraftExpired => "该草稿已过期，请重新生成",
            ErrorCode::DraftNotActionable => "该草稿当前状态不支持此操作",
            ErrorCode::TemplateUnavailable => "所请求的模板暂不可用",
            ErrorCode::MultiItemNotSupported => "Batch create only supports a single item",
            ErrorCode::BatchInProgress => "A batch with this idempotency key is still in progress",

            // Business logic errors - Wallet related
            ErrorCode::InsufficientBalance => "Insufficient balance, please top up",
            ErrorCode::ChargeFailed => "Charge failed",
            ErrorCode::PricingRuleNotFound => "Pricing rule not found",

            // Business logic errors - Resource related
            ErrorCode::TemplateNotFound => "Template not found",
            ErrorCode::CampaignNotFound => "Campaign not found",
            ErrorCode::TaskNotFound => "Task not found",
            ErrorCode::VideoNotFound => "Video not found",

            // Business logic errors - Social account related
            ErrorCode::SocialAccountNotFound => "Social account not found",
            ErrorCode::SocialAccountAlreadyBound => "Social account already bound",

            // Business logic errors - Promo code related
            ErrorCode::PromoCodeNotFound => "Promo code not found",
            ErrorCode::PromoCodeExpired => "Promo code expired",
            ErrorCode::PromoCodeAlreadyUsed => "Promo code already used",
            ErrorCode::PromoCodeUsageLimitReached => "Promo code usage limit reached",

            // Third-party service errors
            ErrorCode::AiServiceError => "AI service error",
            ErrorCode::EmailServiceError => "Email service error",
            ErrorCode::PaymentServiceError => "Payment service error",
            ErrorCode::TikTokApiError => "TikTok API error",
            ErrorCode::InstagramApiError => "Instagram API error",
            ErrorCode::TwitterApiError => "Twitter API error",
            ErrorCode::RedditApiError => "Reddit API error",
        }
    }

    /// Get HTTP status code
    pub fn http_status(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;

        match self {
            // Success
            ErrorCode::Success => StatusCode::OK,

            // Client errors
            ErrorCode::ValidationError | ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized | ErrorCode::TokenExpired | ErrorCode::TokenInvalid => {
                StatusCode::UNAUTHORIZED
            }
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound
            | ErrorCode::UserNotFound
            | ErrorCode::TemplateNotFound
            | ErrorCode::CampaignNotFound
            | ErrorCode::TaskNotFound
            | ErrorCode::VideoNotFound
            | ErrorCode::SocialAccountNotFound
            | ErrorCode::PromoCodeNotFound => StatusCode::NOT_FOUND,
            ErrorCode::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            ErrorCode::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
            ErrorCode::InsufficientBalance => StatusCode::PAYMENT_REQUIRED,

            // Task-template draft
            ErrorCode::DraftExpired => StatusCode::GONE,
            ErrorCode::DraftNotActionable => StatusCode::CONFLICT,

            // Batch create
            ErrorCode::BatchInProgress => StatusCode::CONFLICT,

            // Business logic errors
            ErrorCode::UserAlreadyExists
            | ErrorCode::InvalidPassword
            | ErrorCode::EmailAlreadyInUse
            | ErrorCode::InvalidVerificationCode
            | ErrorCode::VerificationCodeExpired
            | ErrorCode::ChargeFailed
            | ErrorCode::PricingRuleNotFound
            | ErrorCode::SocialAccountAlreadyBound
            | ErrorCode::PromoCodeExpired
            | ErrorCode::PromoCodeAlreadyUsed
            | ErrorCode::PromoCodeUsageLimitReached
            | ErrorCode::TemplateUnavailable
            | ErrorCode::MultiItemNotSupported => StatusCode::BAD_REQUEST,

            // Server errors and third-party service errors
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// NOTE: `impl From<i32> for ErrorCode` lives in `unified_response.rs` (the
// canonical numeric round-trip). The draft codes 4500-4502 are registered
// there.

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// The task-template draft error codes map to the documented HTTP statuses
    /// (DraftExpired→410 Gone, DraftNotActionable→409 Conflict,
    /// TemplateUnavailable→400 Bad Request) and each carries a non-empty Chinese
    /// message. This pins the public error contract for Module D2.
    #[test]
    fn draft_errors_map_to_status() {
        assert_eq!(
            ErrorCode::DraftExpired.http_status(),
            StatusCode::GONE,
            "DraftExpired must map to 410 Gone"
        );
        assert_eq!(
            ErrorCode::DraftNotActionable.http_status(),
            StatusCode::CONFLICT,
            "DraftNotActionable must map to 409 Conflict"
        );
        assert_eq!(
            ErrorCode::TemplateUnavailable.http_status(),
            StatusCode::BAD_REQUEST,
            "TemplateUnavailable must map to 400 Bad Request"
        );

        // Stable numeric codes.
        assert_eq!(ErrorCode::DraftExpired.code(), 4500);
        assert_eq!(ErrorCode::DraftNotActionable.code(), 4501);
        assert_eq!(ErrorCode::TemplateUnavailable.code(), 4502);

        // Round-trip through From<i32>.
        assert_eq!(ErrorCode::from(4500), ErrorCode::DraftExpired);
        assert_eq!(ErrorCode::from(4501), ErrorCode::DraftNotActionable);
        assert_eq!(ErrorCode::from(4502), ErrorCode::TemplateUnavailable);

        // Each draft error must carry a non-empty Chinese message.
        for code in [
            ErrorCode::DraftExpired,
            ErrorCode::DraftNotActionable,
            ErrorCode::TemplateUnavailable,
        ] {
            assert!(
                !code.message_cn().is_empty(),
                "{code:?} must have a non-empty Chinese message"
            );
        }
    }
}
