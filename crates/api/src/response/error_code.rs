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
            ErrorCode::InsufficientBalance => StatusCode::PAYMENT_REQUIRED,

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
            | ErrorCode::PromoCodeUsageLimitReached => StatusCode::BAD_REQUEST,

            // Server errors and third-party service errors
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
